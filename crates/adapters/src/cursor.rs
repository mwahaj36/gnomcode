//! Cursor (`Cursor/User/globalStorage/state.vscdb` / session transcripts)
//! adapter (`SPEC §2`, `§3.5`).

use crate::{FileCtx, FileKind, ParsedBatch, SourceAdapter};
use chrono::{DateTime, Utc};
use serde_json::Value;
use spawnt_core::{
    ActivityEvent, ActivityKind, AgentId, Project, Session, SessionState, Surface, UsageRecord,
    pricing::{TokenBreakdown, calculate_cost},
    redact::redact_summary,
};
use std::collections::HashSet;
use std::path::Path;

/// Stateful parser for Cursor session transcripts / export JSON records.
#[derive(Debug, Default)]
pub struct CursorAdapter {
    seen_dedup_keys: HashSet<String>,
}

impl CursorAdapter {
    /// Creates a new Cursor adapter instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl SourceAdapter for CursorAdapter {
    fn agent_id(&self) -> AgentId {
        AgentId::Cursor
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

        let session_raw_id = val
            .get("sessionId")
            .or_else(|| val.get("conversationId"))
            .and_then(Value::as_str)
            .map_or_else(
                || extract_session_id_from_path(&ctx.source_path),
                str::to_owned,
            );

        let session_id = Session::compose_id(AgentId::Cursor, &session_raw_id);
        let cwd = ctx.fallback_cwd.as_deref().unwrap_or("/unknown");
        let project = Project::from_origin(None, cwd, None);
        batch.projects.push(project.clone());

        let mut session = Session {
            id: session_id.clone(),
            agent: AgentId::Cursor,
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

        let empty_vec = Vec::new();
        let messages = val
            .get("messages")
            .or_else(|| val.get("conversation"))
            .and_then(Value::as_array)
            .unwrap_or(&empty_vec);

        for (idx, msg_val) in messages.iter().enumerate() {
            let Some(msg) = msg_val.as_object() else {
                continue;
            };

            let ts = msg
                .get("timestamp")
                .or_else(|| msg.get("createdAt"))
                .and_then(|v| {
                    if let Some(s) = v.as_str() {
                        DateTime::parse_from_rfc3339(s)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                    } else {
                        v.as_i64().and_then(DateTime::from_timestamp_millis)
                    }
                })
                .unwrap_or_else(Utc::now);

            if idx == 0 {
                session.started_at = ts;
            }
            session.last_event_at = ts;

            let role = msg.get("role").and_then(Value::as_str).unwrap_or("");
            if let Some(m) = msg.get("model").and_then(Value::as_str) {
                session.model = Some(m.to_owned());
            }

            if role == "user" {
                session.state = SessionState::Thinking;
            } else if role == "assistant" {
                let has_tools = msg
                    .get("tool_calls")
                    .and_then(Value::as_array)
                    .is_some_and(|a| !a.is_empty());
                session.state = if has_tools {
                    SessionState::Tool
                } else {
                    SessionState::Idle
                };

                if let Some(calls) = msg.get("tool_calls").and_then(Value::as_array) {
                    for call in calls {
                        let name = call.get("name").and_then(Value::as_str).unwrap_or("tool");
                        let summary = redact_summary(name, "");
                        batch.activity_events.push(ActivityEvent {
                            session_id: session_id.clone(),
                            timestamp: ts,
                            kind: ActivityKind::ToolStart,
                            tool_name: Some(name.to_owned()),
                            summary_redacted: Some(summary),
                            duration_ms: None,
                        });
                    }
                }
            }

            // Usage extraction
            if let Some(usage) = msg.get("usage").and_then(Value::as_object) {
                let in_tok = usage
                    .get("prompt_tokens")
                    .or_else(|| usage.get("input_tokens"))
                    .and_then(Value::as_u64)
                    .unwrap_or(0);
                let out_tok = usage
                    .get("completion_tokens")
                    .or_else(|| usage.get("output_tokens"))
                    .and_then(Value::as_u64)
                    .unwrap_or(0);
                let model = session.model.as_deref().unwrap_or("gpt-4o");

                if in_tok > 0 || out_tok > 0 {
                    let dedup_key = format!("{session_raw_id}:{idx}:{ts}");
                    if self.seen_dedup_keys.insert(dedup_key.clone()) {
                        let breakdown = TokenBreakdown {
                            input_tokens: in_tok,
                            cache_write_5m_tokens: 0,
                            cache_write_1h_tokens: 0,
                            cache_read_tokens: 0,
                            output_tokens: out_tok,
                            reasoning_tokens: 0,
                        };
                        let calc = calculate_cost(model, &breakdown);
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
                            context_window: calc.context_window,
                            cost_usd: calc.cost_usd,
                            cache_savings_usd: 0.0,
                            cost_basis: calc.cost_basis,
                            pricing_version: calc.pricing_version.to_owned(),
                        });
                    }
                }
            }
        }

        batch.sessions.push(session);
        batch
    }
}

fn extract_session_id_from_path(path_str: &str) -> String {
    Path::new(path_str)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("cursor-session")
        .to_owned()
}
