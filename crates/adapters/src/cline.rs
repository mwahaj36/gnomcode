//! Cline / Roo Code (`<editor>/User/globalStorage/{saoudrizwan.claude-dev,rooveterinaryinc.roo-cline}/tasks/<id>/ui_messages.json`)
//! adapter (`SPEC §2`, `§3.5`).

use crate::{FileCtx, FileKind, ParsedBatch, SourceAdapter};
use chrono::{DateTime, Utc};
use serde_json::Value;
use spawnt_core::{
    ActivityEvent, ActivityKind, AgentId, Project, Session, SessionState, Surface, TrustLevel,
    UsageRecord,
    pricing::{TokenBreakdown, calculate_cost},
    redact::redact_summary,
};
use std::collections::HashSet;
use std::path::Path;

/// Stateful parser for Cline / Roo Code `ui_messages.json`.
#[derive(Debug, Default)]
pub struct ClineAdapter {
    seen_dedup_keys: HashSet<String>,
}

impl ClineAdapter {
    /// Creates a new Cline / Roo Code adapter instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl SourceAdapter for ClineAdapter {
    fn agent_id(&self) -> AgentId {
        AgentId::Cline
    }

    fn file_kind(&self) -> FileKind {
        FileKind::RewrittenJson
    }

    fn parse(&mut self, ctx: &FileCtx, chunk: &[u8]) -> ParsedBatch {
        let mut batch = ParsedBatch::default();
        let val: Value = match serde_json::from_slice(chunk) {
            Ok(v) => v,
            Err(_) => {
                batch.parse_errors += 1;
                return batch;
            }
        };

        let messages = if let Some(arr) = val.as_array() {
            arr.as_slice()
        } else {
            batch.parse_errors += 1;
            return batch;
        };

        let task_id = extract_task_id_from_path(&ctx.source_path);
        let session_id = Session::compose_id(AgentId::Cline, &task_id);

        let cwd = ctx.fallback_cwd.as_deref().unwrap_or("/unknown");
        let project = Project::from_origin(None, cwd, None);
        batch.projects.push(project.clone());

        let mut session = Session {
            id: session_id.clone(),
            agent: AgentId::Cline,
            surface: Surface::Ide,
            project_id: project.id,
            parent_session_id: None,
            model: ctx.fallback_model.clone(),
            title: None,
            state: SessionState::Idle,
            pid: None,
            started_at: Utc::now(),
            last_event_at: Utc::now(),
        };

        for (idx, msg_val) in messages.iter().enumerate() {
            let Some(msg) = msg_val.as_object() else {
                continue;
            };

            let ts = msg.get("ts").and_then(parse_ts).unwrap_or_else(Utc::now);

            if idx == 0 {
                session.started_at = ts;
            }
            session.last_event_at = ts;

            let msg_type = msg.get("type").and_then(Value::as_str).unwrap_or("");
            let say = msg.get("say").and_then(Value::as_str).unwrap_or("");
            let ask = msg.get("ask").and_then(Value::as_str).unwrap_or("");

            // Determine state transitions
            if msg_type == "ask" {
                session.state = SessionState::AwaitingApproval;
                if let Some(text) = msg.get("text").and_then(Value::as_str) {
                    let summary =
                        redact_summary(if ask.is_empty() { "approval" } else { ask }, text);
                    batch.activity_events.push(ActivityEvent {
                        session_id: session_id.clone(),
                        timestamp: ts,
                        kind: ActivityKind::ApprovalWait,
                        tool_name: if ask.is_empty() {
                            None
                        } else {
                            Some(ask.to_owned())
                        },
                        summary_redacted: Some(summary),
                        duration_ms: None,
                    });
                }
            } else if say == "api_req_started" {
                session.state = SessionState::Thinking;
                if let Some(text_val) = msg.get("text") {
                    if let Some(parsed) = parse_inner_json(text_val) {
                        if let Some(model) = parsed
                            .get("modelId")
                            .or_else(|| parsed.get("model"))
                            .and_then(Value::as_str)
                        {
                            session.model = Some(model.to_owned());
                        }
                    }
                }
            } else if say == "tool" || say == "command" {
                session.state = SessionState::Tool;
                if let Some(text) = msg.get("text").and_then(Value::as_str) {
                    let summary = redact_summary(say, text);
                    batch.activity_events.push(ActivityEvent {
                        session_id: session_id.clone(),
                        timestamp: ts,
                        kind: ActivityKind::ToolStart,
                        tool_name: Some(say.to_owned()),
                        summary_redacted: Some(summary),
                        duration_ms: None,
                    });
                }
            } else if say == "completion_result" || say == "api_req_finished" {
                session.state = SessionState::Idle;
            }

            // Extract usage records from API calls
            if say == "api_req_started" || say == "api_req_finished" {
                if let Some(text_val) = msg.get("text") {
                    if let Some(meta) = parse_inner_json(text_val) {
                        let in_tok = meta.get("tokensIn").and_then(Value::as_u64).unwrap_or(0);
                        let out_tok = meta.get("tokensOut").and_then(Value::as_u64).unwrap_or(0);
                        let cache_reads =
                            meta.get("cacheReads").and_then(Value::as_u64).unwrap_or(0);
                        let cache_writes =
                            meta.get("cacheWrites").and_then(Value::as_u64).unwrap_or(0);
                        let cost = meta.get("cost").and_then(Value::as_f64);
                        let model = meta
                            .get("modelId")
                            .or_else(|| meta.get("model"))
                            .and_then(Value::as_str)
                            .or(session.model.as_deref())
                            .unwrap_or("claude-3-5-sonnet");

                        if in_tok > 0 || out_tok > 0 || cache_reads > 0 {
                            let dedup_key = format!("{task_id}:{idx}:{ts}");
                            if self.seen_dedup_keys.insert(dedup_key.clone()) {
                                let (cost_usd, cache_savings_usd, basis, window, version) =
                                    if let Some(c) = cost {
                                        (
                                            c,
                                            0.0,
                                            TrustLevel::Exact,
                                            None,
                                            "vendor-reported".to_owned(),
                                        )
                                    } else {
                                        let breakdown = TokenBreakdown {
                                            input_tokens: in_tok,
                                            cache_write_5m_tokens: cache_writes,
                                            cache_write_1h_tokens: 0,
                                            cache_read_tokens: cache_reads,
                                            output_tokens: out_tok,
                                            reasoning_tokens: 0,
                                        };
                                        let calc = calculate_cost(model, &breakdown);
                                        (
                                            calc.cost_usd,
                                            calc.cache_savings_usd,
                                            calc.cost_basis,
                                            calc.context_window,
                                            calc.pricing_version.to_owned(),
                                        )
                                    };

                                batch.usage_records.push(UsageRecord {
                                    dedup_key,
                                    session_id: session_id.clone(),
                                    timestamp: ts,
                                    model: model.to_owned(),
                                    input_tokens: in_tok,
                                    cache_read_tokens: cache_reads,
                                    cache_write_5m_tokens: cache_writes,
                                    cache_write_1h_tokens: 0,
                                    output_tokens: out_tok,
                                    reasoning_tokens: 0,
                                    context_tokens: in_tok + cache_reads + cache_writes,
                                    context_window: window,
                                    cost_usd,
                                    cache_savings_usd,
                                    cost_basis: basis,
                                    pricing_version: version,
                                });
                            }
                        }
                    }
                }
            }
        }

        batch.sessions.push(session);
        batch
    }
}

fn extract_task_id_from_path(path_str: &str) -> String {
    let p = Path::new(path_str);
    p.parent()
        .and_then(|parent| parent.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("task-default")
        .to_owned()
}

fn parse_ts(val: &Value) -> Option<DateTime<Utc>> {
    if let Some(millis) = val.as_i64() {
        return DateTime::from_timestamp_millis(millis);
    }
    if let Some(s) = val.as_str() {
        if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
            return Some(dt.with_timezone(&Utc));
        }
    }
    None
}

fn parse_inner_json(val: &Value) -> Option<Value> {
    if val.is_object() {
        return Some(val.clone());
    }
    if let Some(s) = val.as_str() {
        serde_json::from_str(s).ok()
    } else {
        None
    }
}
