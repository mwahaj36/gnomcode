//! Google Gemini CLI (`~/.gemini/tmp/<project-hash>/chats/session-*.json`)
//! rewritten JSON adapter (`SPEC §2.1`, `§3.5`).

use crate::{
    FileCtx, FileKind, ParsedBatch, SourceAdapter,
    claude::{classify_tool_kind, extract_tool_detail, fallback_id_from_path},
};
use chrono::{DateTime, Utc};
use serde_json::Value;
use spawnt_core::{
    ActivityEvent, AgentId, ChatEvent, ChatRole, Project, Session, SessionState, Surface,
    UsageRecord,
    pricing::{TokenBreakdown, calculate_cost},
    redact::{redact_chat_content, redact_summary},
};
use std::collections::HashSet;

/// Stateful parser for a Gemini CLI rewritten `.json` session log.
#[derive(Debug, Default)]
pub struct GeminiAdapter {
    seen_dedup_keys: HashSet<String>,
}

impl GeminiAdapter {
    /// Creates a new Gemini CLI adapter instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl SourceAdapter for GeminiAdapter {
    fn agent_id(&self) -> AgentId {
        AgentId::GeminiCli
    }

    fn file_kind(&self) -> FileKind {
        FileKind::RewrittenJson
    }

    fn parse(&mut self, ctx: &FileCtx, chunk: &[u8]) -> ParsedBatch {
        let mut batch = ParsedBatch::default();
        let Ok(val) = serde_json::from_slice::<Value>(chunk) else {
            batch.parse_errors += 1;
            return batch;
        };
        let Some(root) = val.as_object() else {
            batch.parse_errors += 1;
            return batch;
        };

        let raw_sid = root
            .get("sessionId")
            .and_then(Value::as_str)
            .unwrap_or_else(|| fallback_id_from_path(&ctx.source_path));
        let session_id = Session::compose_id(AgentId::GeminiCli, raw_sid);

        let cwd = root
            .get("cwd")
            .and_then(Value::as_str)
            .or(ctx.fallback_cwd.as_deref())
            .unwrap_or("/unknown");
        let branch = root.get("gitBranch").and_then(Value::as_str);
        let project = Project::from_origin(None, cwd, branch);
        batch.projects.push(project.clone());

        let mut session = Session {
            id: session_id.clone(),
            agent: AgentId::GeminiCli,
            surface: Surface::Cli,
            project_id: project.id,
            parent_session_id: None,
            model: None,
            title: None,
            state: SessionState::Idle,
            pid: None,
            started_at: Utc::now(),
            last_event_at: Utc::now(),
        };

        if let Some(messages) = root.get("messages").and_then(Value::as_array) {
            for (idx, msg_val) in messages.iter().enumerate() {
                let Some(msg) = msg_val.as_object() else {
                    continue;
                };
                let ts = msg
                    .get("timestamp")
                    .and_then(Value::as_str)
                    .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                    .map_or_else(Utc::now, |dt| dt.with_timezone(&Utc));
                if idx == 0 {
                    session.started_at = ts;
                }
                session.last_event_at = ts;

                let role = match msg.get("type").and_then(Value::as_str) {
                    Some("user") => Some(ChatRole::User),
                    Some("gemini") => Some(ChatRole::Assistant),
                    _ => None,
                };
                if let Some(role) = role {
                    let content = msg.get("content").or_else(|| msg.get("parts"));
                    let text = match content {
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
                            role,
                            content: redact_chat_content(&text),
                            tool_name: None,
                            status: None,
                            created_at: ts,
                        });
                    }
                }

                if msg.get("type").and_then(Value::as_str) == Some("gemini") {
                    let model = msg
                        .get("model")
                        .and_then(Value::as_str)
                        .filter(|model| !model.trim().is_empty())
                        .unwrap_or("unknown");
                    if model != "unknown" {
                        session.model = Some(model.to_owned());
                    }

                    if let Some(calls) = msg.get("toolCalls").and_then(Value::as_array) {
                        for call in calls {
                            let name = call.get("name").and_then(Value::as_str).unwrap_or("tool");
                            let detail = extract_tool_detail(call.get("args"));
                            let summary = redact_summary(name, &detail);
                            let kind = classify_tool_kind(name);
                            session.state = SessionState::Tool;
                            batch.activity_events.push(ActivityEvent {
                                session_id: session_id.clone(),
                                timestamp: ts,
                                kind,
                                tool_name: Some(name.to_owned()),
                                summary_redacted: Some(summary),
                                duration_ms: None,
                            });
                        }
                    }

                    let msg_id = msg
                        .get("id")
                        .and_then(Value::as_str)
                        .map_or_else(|| idx.to_string(), str::to_owned);
                    let dedup_key = format!("{raw_sid}:{msg_id}");

                    if self.seen_dedup_keys.insert(dedup_key.clone()) {
                        if let Some(tokens) = msg.get("tokens").and_then(Value::as_object) {
                            let total_in = tokens.get("input").and_then(Value::as_u64).unwrap_or(0);
                            let cached = tokens.get("cached").and_then(Value::as_u64).unwrap_or(0);
                            let output = tokens.get("output").and_then(Value::as_u64).unwrap_or(0);
                            let thoughts =
                                tokens.get("thoughts").and_then(Value::as_u64).unwrap_or(0);
                            let uncached_in = total_in.saturating_sub(cached);

                            let breakdown = TokenBreakdown {
                                input_tokens: uncached_in,
                                output_tokens: output,
                                reasoning_tokens: thoughts,
                                cache_read_tokens: cached,
                                cache_write_5m_tokens: 0,
                                cache_write_1h_tokens: 0,
                            };
                            let eval = calculate_cost(model, &breakdown);
                            batch.usage_records.push(UsageRecord {
                                dedup_key,
                                session_id: session_id.clone(),
                                timestamp: ts,
                                model: model.to_owned(),
                                input_tokens: total_in,
                                output_tokens: output,
                                reasoning_tokens: thoughts,
                                cache_read_tokens: cached,
                                cache_write_5m_tokens: 0,
                                cache_write_1h_tokens: 0,
                                context_tokens: total_in,
                                context_window: eval.context_window,
                                cost_usd: eval.cost_usd,
                                cache_savings_usd: eval.cache_savings_usd,
                                cost_basis: eval.cost_basis,
                                pricing_version: eval.pricing_version.to_owned(),
                            });
                        }
                    }
                }
            }
        }

        batch.sessions.push(session);
        crate::titles::apply_native_titles(&mut batch, AgentId::GeminiCli, chunk);
        batch
    }
}
