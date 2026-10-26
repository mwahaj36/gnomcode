//! Claude Code JSONL log adapter (`SPEC §2.1`, `§3.5`).
//!
//! Reads `~/.claude/projects/**/*.jsonl`. Deduplicates assistant blocks by
//! `format!("{}:{}", message.id, request_id)` because Claude Code writes one
//! JSONL line per content block and copies prior history on `--resume`.

use crate::{FileCtx, FileKind, ParsedBatch, SourceAdapter};
use chrono::{DateTime, Utc};
use fleethud_core::{
    ActivityEvent, ActivityKind, AgentId, Project, Session, SessionState, Surface, UsageRecord,
    pricing::{TokenBreakdown, calculate_cost},
    redact::redact_summary,
};
use serde_json::Value;
use std::collections::HashSet;

/// Stateful parser for a Claude Code `.jsonl` session log.
#[derive(Debug, Default)]
pub struct ClaudeAdapter {
    seen_dedup_keys: HashSet<String>,
}

impl ClaudeAdapter {
    /// Creates a new Claude Code adapter instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl SourceAdapter for ClaudeAdapter {
    fn agent_id(&self) -> AgentId {
        AgentId::ClaudeCode
    }

    fn file_kind(&self) -> FileKind {
        FileKind::AppendOnlyJsonl
    }

    fn parse(&mut self, ctx: &FileCtx, chunk: &[u8]) -> ParsedBatch {
        let mut batch = ParsedBatch::default();
        let text = String::from_utf8_lossy(chunk);
        let mut current_session: Option<Session> = None;

        for raw_line in text.lines() {
            let line = raw_line.trim();
            if line.is_empty() {
                continue;
            }
            let Ok(val) = serde_json::from_str::<Value>(line) else {
                batch.parse_errors += 1;
                continue;
            };
            let Some(obj) = val.as_object() else {
                batch.parse_errors += 1;
                continue;
            };

            let ts = obj
                .get("timestamp")
                .and_then(Value::as_str)
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map_or_else(Utc::now, |dt| dt.with_timezone(&Utc));

            let raw_sid = obj
                .get("sessionId")
                .and_then(Value::as_str)
                .unwrap_or_else(|| fallback_id_from_path(&ctx.source_path));
            let session_id = Session::compose_id(AgentId::ClaudeCode, raw_sid);

            let cwd = obj
                .get("cwd")
                .and_then(Value::as_str)
                .or(ctx.fallback_cwd.as_deref())
                .unwrap_or("/unknown");
            let git_branch = obj.get("gitBranch").and_then(Value::as_str);

            let surface = match obj.get("entrypoint").and_then(Value::as_str) {
                Some(ep)
                    if ep.contains("vscode") || ep.contains("cursor") || ep.contains("ide") =>
                {
                    Surface::Ide
                }
                _ => Surface::Cli,
            };

            let project = Project::from_origin(None, cwd, git_branch);
            if !batch.projects.iter().any(|p| p.id == project.id) {
                batch.projects.push(project.clone());
            }

            let sess = current_session.get_or_insert_with(|| Session {
                id: session_id.clone(),
                agent: AgentId::ClaudeCode,
                surface,
                project_id: project.id.clone(),
                parent_session_id: None,
                model: None,
                title: None,
                state: SessionState::Idle,
                pid: None,
                started_at: ts,
                last_event_at: ts,
            });
            sess.last_event_at = ts;
            sess.surface = surface;

            if obj.get("type").and_then(Value::as_str) == Some("assistant") {
                if let Some(msg) = obj.get("message").and_then(Value::as_object) {
                    if let Some(model) = msg.get("model").and_then(Value::as_str) {
                        sess.model = Some(model.to_owned());
                    }

                    // Extract tool_use summaries from content blocks
                    if let Some(blocks) = msg.get("content").and_then(Value::as_array) {
                        for block in blocks {
                            if block.get("type").and_then(Value::as_str) == Some("tool_use") {
                                let name =
                                    block.get("name").and_then(Value::as_str).unwrap_or("tool");
                                let detail = extract_tool_detail(block.get("input"));
                                let summary = redact_summary(name, &detail);
                                let kind = classify_tool_kind(name);
                                sess.state = SessionState::Tool;
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
                    }

                    // Deduplicate token usage across multi-block assistant lines
                    let msg_id = msg.get("id").and_then(Value::as_str).unwrap_or("");
                    let req_id = obj.get("requestId").and_then(Value::as_str).unwrap_or("");
                    if !msg_id.is_empty() && !req_id.is_empty() {
                        let dedup_key = format!("{msg_id}:{req_id}");
                        if self.seen_dedup_keys.insert(dedup_key.clone()) {
                            if let Some(usage_obj) = msg.get("usage").and_then(Value::as_object) {
                                let record = build_usage_record(
                                    dedup_key,
                                    session_id,
                                    ts,
                                    sess.model.as_deref().unwrap_or("claude-sonnet"),
                                    usage_obj,
                                );
                                batch.usage_records.push(record);
                            }
                        }
                    }
                }
            }
        }

        if let Some(sess) = current_session {
            batch.sessions.push(sess);
        }
        batch
    }
}

fn build_usage_record(
    dedup_key: String,
    session_id: String,
    timestamp: DateTime<Utc>,
    model: &str,
    usage_obj: &serde_json::Map<String, Value>,
) -> UsageRecord {
    let input_tokens = usage_obj
        .get("input_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let output_tokens = usage_obj
        .get("output_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let cache_read_tokens = usage_obj
        .get("cache_read_input_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let total_cache_creation = usage_obj
        .get("cache_creation_input_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);

    let (w5m, w1h) = if let Some(cc) = usage_obj.get("cache_creation").and_then(Value::as_object) {
        let e5 = cc
            .get("ephemeral_5m_input_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let e1 = cc
            .get("ephemeral_1h_input_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        if e5 == 0 && e1 == 0 {
            (total_cache_creation, 0)
        } else {
            (e5, e1)
        }
    } else {
        (total_cache_creation, 0)
    };

    let breakdown = TokenBreakdown {
        input_tokens,
        output_tokens,
        reasoning_tokens: 0,
        cache_read_tokens,
        cache_write_5m_tokens: w5m,
        cache_write_1h_tokens: w1h,
    };
    let eval = calculate_cost(model, &breakdown);
    let context_tokens = input_tokens
        .saturating_add(cache_read_tokens)
        .saturating_add(w5m)
        .saturating_add(w1h);

    UsageRecord {
        dedup_key,
        session_id,
        timestamp,
        model: model.to_owned(),
        input_tokens,
        output_tokens,
        reasoning_tokens: 0,
        cache_read_tokens,
        cache_write_5m_tokens: w5m,
        cache_write_1h_tokens: w1h,
        context_tokens,
        context_window: eval.context_window,
        cost_usd: eval.cost_usd,
        cache_savings_usd: eval.cache_savings_usd,
        cost_basis: eval.cost_basis,
        pricing_version: eval.pricing_version.to_owned(),
    }
}

pub(crate) fn classify_tool_kind(tool_name: &str) -> ActivityKind {
    let lower = tool_name.to_ascii_lowercase();
    if lower.contains("agent") || lower.contains("task") {
        ActivityKind::Subagent
    } else {
        ActivityKind::ToolStart
    }
}

pub(crate) fn extract_tool_detail(input: Option<&Value>) -> String {
    let Some(obj) = input.and_then(Value::as_object) else {
        return String::new();
    };
    for key in [
        "file_path",
        "path",
        "Path",
        "command",
        "CommandLine",
        "description",
        "toolSummary",
        "pattern",
    ] {
        if let Some(val) = obj.get(key).and_then(Value::as_str) {
            if val != "[REDACTED]" && !val.trim().is_empty() {
                return val.to_owned();
            }
        }
    }
    String::new()
}

pub(crate) fn fallback_id_from_path(path: &str) -> &str {
    path.rsplit(['/', '\\'])
        .next()
        .and_then(|name| {
            name.strip_suffix(".jsonl")
                .or_else(|| name.strip_suffix(".json"))
        })
        .unwrap_or("unknown")
}
