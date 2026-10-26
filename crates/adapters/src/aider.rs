//! Aider (`--analytics-log <file>` / `.aider.analytics.jsonl`)
//! adapter (`SPEC §2`, `§3.5`).

use crate::{FileCtx, FileKind, ParsedBatch, SourceAdapter};
use chrono::{DateTime, Utc};
use fleethud_core::{
    ActivityEvent, ActivityKind, AgentId, Project, Session, SessionState, Surface, TrustLevel,
    UsageRecord,
    pricing::{TokenBreakdown, calculate_cost},
    redact::redact_summary,
};
use serde_json::Value;
use std::collections::HashSet;
use std::path::Path;

/// Stateful parser for Aider analytics log files (`.jsonl`).
#[derive(Debug, Default)]
pub struct AiderAdapter {
    seen_dedup_keys: HashSet<String>,
}

impl AiderAdapter {
    /// Creates a new Aider adapter instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl SourceAdapter for AiderAdapter {
    fn agent_id(&self) -> AgentId {
        AgentId::Aider
    }

    fn file_kind(&self) -> FileKind {
        FileKind::AppendOnlyJsonl
    }

    fn parse(&mut self, ctx: &FileCtx, chunk: &[u8]) -> ParsedBatch {
        let mut batch = ParsedBatch::default();
        let chunk_str = match std::str::from_utf8(chunk) {
            Ok(s) => s,
            Err(_) => {
                batch.parse_errors += 1;
                return batch;
            }
        };

        let raw_sid = extract_session_id_from_path(&ctx.source_path);
        let session_id = Session::compose_id(AgentId::Aider, &raw_sid);

        let cwd = ctx.fallback_cwd.as_deref().unwrap_or("/unknown");
        let project = Project::from_origin(None, cwd, None);
        batch.projects.push(project.clone());

        let mut current_session: Option<Session> = None;

        for (line_idx, line) in chunk_str.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let obj: Value = match serde_json::from_str(trimmed) {
                Ok(v) => v,
                Err(_) => {
                    batch.parse_errors += 1;
                    continue;
                }
            };

            let ts = obj
                .get("timestamp")
                .or_else(|| obj.get("ts"))
                .and_then(|v| {
                    if let Some(s) = v.as_str() {
                        DateTime::parse_from_rfc3339(s)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                    } else if let Some(f) = v.as_f64() {
                        DateTime::from_timestamp(f as i64, 0)
                    } else {
                        v.as_i64().and_then(DateTime::from_timestamp_millis)
                    }
                })
                .unwrap_or_else(Utc::now);

            let sess = current_session.get_or_insert_with(|| Session {
                id: session_id.clone(),
                agent: AgentId::Aider,
                surface: Surface::Cli,
                project_id: project.id.clone(),
                parent_session_id: None,
                model: ctx.fallback_model.clone(),
                title: None,
                state: SessionState::Idle,
                pid: None,
                started_at: ts,
                last_event_at: ts,
            });

            sess.last_event_at = ts;

            let model = obj
                .get("model")
                .and_then(Value::as_str)
                .unwrap_or("claude-3-5-sonnet");
            sess.model = Some(model.to_owned());

            let event = obj.get("event").and_then(Value::as_str).unwrap_or("chat");
            let is_turn_end = event == "chat" || event == "response" || event == "turn_complete";

            sess.state = if is_turn_end {
                SessionState::Idle
            } else {
                SessionState::Thinking
            };

            if let Some(cmd) = obj
                .get("command")
                .or_else(|| obj.get("tool"))
                .and_then(Value::as_str)
            {
                let summary = redact_summary(cmd, "");
                batch.activity_events.push(ActivityEvent {
                    session_id: session_id.clone(),
                    timestamp: ts,
                    kind: ActivityKind::ToolStart,
                    tool_name: Some(cmd.to_owned()),
                    summary_redacted: Some(summary),
                    duration_ms: None,
                });
            }

            // Usage extraction
            let in_tok = obj
                .get("tokens_sent")
                .or_else(|| obj.get("prompt_tokens"))
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let out_tok = obj
                .get("tokens_received")
                .or_else(|| obj.get("completion_tokens"))
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let explicit_cost = obj.get("cost").and_then(Value::as_f64);

            if in_tok > 0 || out_tok > 0 {
                let dedup_key = format!("{raw_sid}:{line_idx}:{ts}");
                if self.seen_dedup_keys.insert(dedup_key.clone()) {
                    let (cost_usd, cache_savings_usd, basis, window, version) =
                        if let Some(c) = explicit_cost {
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
                                cache_write_5m_tokens: 0,
                                cache_write_1h_tokens: 0,
                                cache_read_tokens: 0,
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
                        cache_read_tokens: 0,
                        cache_write_5m_tokens: 0,
                        cache_write_1h_tokens: 0,
                        output_tokens: out_tok,
                        reasoning_tokens: 0,
                        context_tokens: in_tok,
                        context_window: window,
                        cost_usd,
                        cache_savings_usd,
                        cost_basis: basis,
                        pricing_version: version,
                    });
                }
            }
        }

        if let Some(sess) = current_session {
            batch.sessions.push(sess);
        }
        batch
    }
}

fn extract_session_id_from_path(path_str: &str) -> String {
    Path::new(path_str)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("aider-session")
        .to_owned()
}
