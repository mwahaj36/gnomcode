//! Google Antigravity (`~/.gemini/antigravity/brain/<id>/.system_generated/logs/transcript.jsonl`)
//! log adapter (`SPEC §2.1`, `§3.5`, `§3.9`).
//!
//! Tracks `PLANNER_RESPONSE` token usage and extracts active model transitions
//! from `<USER_SETTINGS_CHANGE>` tags inside `USER_INPUT` steps.

use crate::{
    FileCtx, FileKind, ParsedBatch, SourceAdapter,
    claude::{classify_tool_kind, extract_tool_detail},
};
use chrono::{DateTime, Utc};
use fleethud_core::{
    ActivityEvent, AgentId, Project, Session, SessionState, Surface, UsageRecord,
    pricing::{TokenBreakdown, calculate_cost},
    redact::redact_summary,
};
use serde_json::Value;
use std::collections::HashSet;

/// Stateful parser for an Antigravity `transcript.jsonl` conversation log.
#[derive(Debug, Default)]
pub struct AntigravityAdapter {
    seen_dedup_keys: HashSet<String>,
    active_model: Option<String>,
}

impl AntigravityAdapter {
    /// Creates a new Antigravity adapter instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl SourceAdapter for AntigravityAdapter {
    fn agent_id(&self) -> AgentId {
        AgentId::Antigravity
    }

    fn file_kind(&self) -> FileKind {
        FileKind::AppendOnlyJsonl
    }

    fn parse(&mut self, ctx: &FileCtx, chunk: &[u8]) -> ParsedBatch {
        let mut batch = ParsedBatch::default();
        let text = String::from_utf8_lossy(chunk);
        let conv_id = extract_conversation_id(&ctx.source_path);
        let session_id = Session::compose_id(AgentId::Antigravity, conv_id);
        let cwd = ctx.fallback_cwd.as_deref().unwrap_or("/unknown");
        let project = Project::from_origin(None, cwd, None);
        batch.projects.push(project.clone());

        if self.active_model.is_none() {
            self.active_model.clone_from(&ctx.fallback_model);
        }

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
                .get("created_at")
                .and_then(Value::as_str)
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map_or_else(Utc::now, |dt| dt.with_timezone(&Utc));

            let step_idx = obj.get("step_index").and_then(Value::as_u64).unwrap_or(0);
            let step_type = obj.get("type").and_then(Value::as_str).unwrap_or("");

            let sess = current_session.get_or_insert_with(|| Session {
                id: session_id.clone(),
                agent: AgentId::Antigravity,
                surface: Surface::Ide,
                project_id: project.id.clone(),
                parent_session_id: None,
                model: self.active_model.clone(),
                title: None,
                state: SessionState::Thinking,
                pid: None,
                started_at: ts,
                last_event_at: ts,
            });
            sess.last_event_at = ts;

            if step_type == "USER_INPUT" {
                sess.state = SessionState::Thinking;
                if let Some(content) = obj.get("content").and_then(Value::as_str) {
                    if let Some(new_model) = parse_model_selection_change(content) {
                        self.active_model = Some(new_model.clone());
                        sess.model = Some(new_model);
                    }
                    if let Some(ws) = extract_workspace_from_text(content) {
                        let proj = Project::from_origin(None, &ws, None);
                        sess.project_id = proj.id.clone();
                        if !batch.projects.iter().any(|p| p.id == proj.id) {
                            batch.projects.push(proj);
                        }
                    }
                }
            } else if step_type == "PLANNER_RESPONSE" {
                let has_tools = obj
                    .get("tool_calls")
                    .and_then(Value::as_array)
                    .is_some_and(|calls| !calls.is_empty());

                sess.state = if has_tools {
                    SessionState::Tool
                } else {
                    SessionState::Idle
                };

                let raw_model = self
                    .active_model
                    .as_deref()
                    .or(ctx.fallback_model.as_deref())
                    .unwrap_or("Gemini 3.8 Flash (Default)");

                let model = if raw_model.contains("MODEL_PLACEHOLDER") {
                    "Gemini 3.8 Flash (Default)"
                } else {
                    raw_model
                };

                sess.model = Some(model.to_owned());

                if let Some(calls) = obj.get("tool_calls").and_then(Value::as_array) {
                    for call in calls {
                        let name = call.get("name").and_then(Value::as_str).unwrap_or("tool");
                        let detail = extract_tool_detail(call.get("args"));
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

                        if let Some(args) = call.get("args").and_then(Value::as_object) {
                            for k in ["Path", "path", "file_path"] {
                                if let Some(p_str) = args.get(k).and_then(Value::as_str) {
                                    if let Some(root) = find_repo_root_from_path(p_str) {
                                        let proj = Project::from_origin(None, &root, None);
                                        sess.project_id = proj.id.clone();
                                        if !batch.projects.iter().any(|p| p.id == proj.id) {
                                            batch.projects.push(proj);
                                        }
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }

                let dedup_key = format!("{conv_id}:{step_idx}");
                if self.seen_dedup_keys.insert(dedup_key.clone()) {
                    let input_tokens = obj.get("input_tokens").and_then(Value::as_u64).unwrap_or(0);
                    let cache_read = obj
                        .get("cache_read_tokens")
                        .and_then(Value::as_u64)
                        .unwrap_or(0);
                    let output_tokens = obj
                        .get("output_tokens")
                        .and_then(Value::as_u64)
                        .unwrap_or(0);

                    if input_tokens > 0 || output_tokens > 0 || cache_read > 0 {
                        let uncached_in = input_tokens.saturating_sub(cache_read);
                        let breakdown = TokenBreakdown {
                            input_tokens: uncached_in,
                            output_tokens,
                            reasoning_tokens: 0,
                            cache_read_tokens: cache_read,
                            cache_write_5m_tokens: 0,
                            cache_write_1h_tokens: 0,
                        };
                        let eval = calculate_cost(model, &breakdown);
                        batch.usage_records.push(UsageRecord {
                            dedup_key,
                            session_id: session_id.clone(),
                            timestamp: ts,
                            model: model.to_owned(),
                            input_tokens,
                            output_tokens,
                            reasoning_tokens: 0,
                            cache_read_tokens: cache_read,
                            cache_write_5m_tokens: 0,
                            cache_write_1h_tokens: 0,
                            context_tokens: input_tokens,
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

        if let Some(sess) = current_session {
            batch.sessions.push(sess);
        }
        batch
    }
}

/// Extracts the model name from `<USER_SETTINGS_CHANGE>` text:
/// `The user changed setting \`Model Selection\` from <old> to <new>.`
#[must_use]
pub fn parse_model_selection_change(content: &str) -> Option<String> {
    let marker = "Model Selection` from ";
    let idx = content.find(marker)?;
    let after = &content[idx + marker.len()..];
    let to_idx = after.find(" to ")?;
    let rest = &after[to_idx + 4..];
    let end_idx = if let Some(i) = rest.find(". ") {
        i
    } else if let Some(i) = rest.find(".\n") {
        i
    } else if let Some(i) = rest.find("</") {
        i
    } else if let Some(i) = rest.find('\n') {
        i
    } else {
        rest.len()
    };
    let model = rest[..end_idx].trim().trim_end_matches('.');
    if model.is_empty() {
        None
    } else {
        Some(model.to_owned())
    }
}

/// Extracts the conversation UUID from an Antigravity `brain/<uuid>/...` path.
#[must_use]
pub fn extract_conversation_id(path: &str) -> &str {
    let parts: Vec<&str> = path.split(['/', '\\']).collect();
    for (i, part) in parts.iter().enumerate() {
        if *part == "brain" {
            if let Some(id) = parts.get(i + 1) {
                if !id.is_empty() {
                    return id;
                }
            }
        }
    }
    "unknown-conversation"
}

/// Extracts workspace directory hint from system instruction text or environment tags.
#[must_use]
pub fn extract_workspace_from_text(content: &str) -> Option<String> {
    for marker in [
        "Active Workspaces:\n- ",
        "Active Workspaces:\r\n- ",
        "Command Working Directory: ",
    ] {
        if let Some(idx) = content.find(marker) {
            let start = idx + marker.len();
            let line = content[start..].lines().next().unwrap_or("").trim();
            if !line.is_empty() && (line.contains('/') || line.contains('\\')) {
                return Some(line.to_owned());
            }
        }
    }
    None
}

/// Recursively detects the repository or workspace root for a given file path.
#[must_use]
pub fn find_repo_root_from_path(file_path: &str) -> Option<String> {
    let clean = file_path.trim_matches('"').trim_matches('\'');
    let path_obj = std::path::Path::new(clean);
    let mut check = if path_obj.is_file() {
        path_obj.parent()?
    } else {
        path_obj
    };

    loop {
        if check.join(".git").exists()
            || check.join("Cargo.toml").exists()
            || check.join("package.json").exists()
        {
            return Some(check.to_string_lossy().into_owned());
        }
        match check.parent() {
            Some(p) if p != check => check = p,
            _ => break,
        }
    }

    if check.exists() && check.is_dir() {
        Some(check.to_string_lossy().into_owned())
    } else {
        None
    }
}
