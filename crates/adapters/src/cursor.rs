//! Cursor (`Cursor/User/globalStorage/state.vscdb` / session transcripts)
//! adapter (`SPEC §2`, `§3.5`).

use crate::{FileCtx, FileKind, ParsedBatch, SourceAdapter};
use chrono::{DateTime, Utc};
use gnomcode_core::{
    ActivityEvent, ActivityKind, AgentId, ChatEvent, ChatRole, Project, Session, SessionState,
    Surface, UsageRecord,
    pricing::{TokenBreakdown, calculate_cost},
    redact::{redact_chat_content, redact_summary},
};
use serde_json::Value;
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

        let Some(messages) = val
            .get("messages")
            .or_else(|| val.get("conversation"))
            .and_then(Value::as_array)
        else {
            return batch;
        };
        let Some(started_at) = messages.iter().filter_map(message_timestamp).min() else {
            return batch;
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
            state: SessionState::Ended,
            pid: None,
            started_at,
            last_event_at: started_at,
        };

        for (idx, msg_val) in messages.iter().enumerate() {
            let Some(msg) = msg_val.as_object() else {
                continue;
            };

            let Some(ts) = message_timestamp(msg_val) else {
                continue;
            };
            session.last_event_at = session.last_event_at.max(ts);

            let role = msg.get("role").and_then(Value::as_str).unwrap_or("");
            let chat_role = match role {
                "user" => Some(ChatRole::User),
                "assistant" => Some(ChatRole::Assistant),
                _ => None,
            };
            if let Some(chat_role) = chat_role {
                let text = match msg.get("content") {
                    Some(Value::String(text)) => text.clone(),
                    Some(Value::Array(parts)) => parts
                        .iter()
                        .filter_map(|part| part.get("text").and_then(Value::as_str))
                        .collect::<Vec<_>>()
                        .join("\n"),
                    _ => String::new(),
                };
                if !text.is_empty() {
                    let message_id = msg
                        .get("id")
                        .and_then(Value::as_str)
                        .map_or_else(|| idx.to_string(), str::to_owned);
                    batch.chat_events.push(ChatEvent {
                        event_id: format!("{session_id}:{message_id}"),
                        session_id: session_id.clone(),
                        turn_index: i64::try_from(idx).unwrap_or(i64::MAX),
                        role: chat_role,
                        content: redact_chat_content(&text),
                        tool_name: None,
                        status: None,
                        created_at: ts,
                    });
                }
            }
            if let Some(m) = msg.get("model").and_then(Value::as_str) {
                session.model = Some(m.to_owned());
            }

            if role == "assistant" {
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
                let model = session.model.as_deref().unwrap_or("unknown");

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
        crate::titles::apply_native_titles(&mut batch, AgentId::Cursor, chunk);
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

fn message_timestamp(message: &Value) -> Option<DateTime<Utc>> {
    if !matches!(message.get("role")?.as_str()?, "user" | "assistant") {
        return None;
    }
    let timestamp = message
        .get("timestamp")
        .or_else(|| message.get("createdAt"))?;
    if let Some(text) = timestamp.as_str() {
        DateTime::parse_from_rfc3339(text)
            .ok()
            .map(|date| date.with_timezone(&Utc))
    } else {
        timestamp.as_i64().and_then(DateTime::from_timestamp_millis)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_settings_do_not_create_a_conversation() {
        let batch = CursorAdapter::new().parse(
            &FileCtx {
                source_path: "argv.json".into(),
                ..FileCtx::default()
            },
            include_bytes!("../../../fixtures/cursor/2026-11-01/settings.json"),
        );
        insta::assert_json_snapshot!(batch, @r#"
        {
          "projects": [],
          "sessions": [],
          "usage_records": [],
          "activity_events": [],
          "chat_events": [],
          "quota_snapshots": [],
          "parse_errors": 0
        }
        "#);
    }

    #[test]
    fn untimed_cursor_messages_never_acquire_the_ingestion_time() {
        let bytes = br#"{"sessionId":"saved","messages":[{"role":"user","content":"hello"}]}"#;
        assert!(
            CursorAdapter::new()
                .parse(&FileCtx::default(), bytes)
                .sessions
                .is_empty()
        );
    }

    #[test]
    fn cursor_export_is_deterministic_and_does_not_claim_a_live_process() {
        let bytes = br#"{"sessionId":"saved","messages":[{"role":"user","timestamp":"2026-10-26T12:00:00Z","content":"hello"},{"role":"assistant","timestamp":"2026-10-26T12:00:05Z","content":"hi","usage":{"prompt_tokens":5,"completion_tokens":2}}]}"#;
        let first = CursorAdapter::new().parse(&FileCtx::default(), bytes);
        let second = CursorAdapter::new().parse(&FileCtx::default(), bytes);
        assert_eq!(first, second);
        assert_eq!(first.sessions[0].state, SessionState::Ended);
        assert_eq!(
            first.sessions[0].last_event_at.to_rfc3339(),
            "2026-10-26T12:00:05+00:00"
        );
    }
}
