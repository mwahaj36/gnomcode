//! OpenAI Codex CLI & ChatGPT/Codex Desktop App log adapter (`SPEC §2.1`, `§3.5`, `§3.9`).
//!
//! Both the CLI and Desktop App (`OpenAI.Codex`) write rollout logs under
//! `$CODEX_HOME/sessions/YYYY/MM/DD/rollout-*.jsonl`.
//! Uses `last_token_usage` for per-turn deltas and extracts ground-truth
//! `rate_limits` pools (`primary` 5h and `secondary` 7d windows).

use crate::{FileCtx, FileKind, ParsedBatch, SourceAdapter, claude::fallback_id_from_path};
use chrono::{DateTime, TimeZone, Utc};
use serde_json::Value;
use spawnt_core::redact::clean_user_text;
use spawnt_core::{
    ActivityEvent, ActivityKind, AgentId, Project, QuotaSnapshot, QuotaWindow, Session,
    SessionState, Surface, TrustLevel, UsageRecord,
    pricing::{TokenBreakdown, calculate_cost},
};
use std::collections::HashSet;

/// Stateful parser for a Codex `rollout-*.jsonl` session log.
#[derive(Debug, Default)]
pub struct CodexAdapter {
    seen_dedup_keys: HashSet<String>,
    raw_session_id: Option<String>,
    active_model: Option<String>,
    last_turn_model: Option<String>,
}

impl CodexAdapter {
    /// Creates a new Codex adapter instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl SourceAdapter for CodexAdapter {
    fn agent_id(&self) -> AgentId {
        AgentId::Codex
    }

    fn file_kind(&self) -> FileKind {
        FileKind::AppendOnlyJsonl
    }

    fn parse(&mut self, ctx: &FileCtx, chunk: &[u8]) -> ParsedBatch {
        let mut batch = ParsedBatch::default();
        let mut current_session: Option<Session> = None;

        if self.active_model.is_none() {
            self.active_model.clone_from(&ctx.fallback_model);
        }
        if self.last_turn_model.is_none() && ctx.source_offset > 0 {
            self.last_turn_model.clone_from(&ctx.fallback_model);
        }

        let default_cwd = ctx.fallback_cwd.as_deref().unwrap_or("/unknown");
        let default_project = Project::from_origin(None, default_cwd, None);

        let mut line_offset = ctx.source_offset;
        for raw_line in chunk.split_inclusive(|byte| *byte == b'\n') {
            let current_offset = line_offset;
            line_offset = line_offset.saturating_add(raw_line.len() as u64);
            let raw_line = String::from_utf8_lossy(raw_line);
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

            let ordinal = obj.get("ordinal").and_then(Value::as_u64).unwrap_or(0);
            let line_type = obj.get("type").and_then(Value::as_str).unwrap_or("");
            let Some(payload) = obj.get("payload").and_then(Value::as_object) else {
                continue;
            };

            if self.raw_session_id.is_none() {
                if let Some(sid) = payload
                    .get("session_id")
                    .or_else(|| payload.get("thread_id"))
                    .and_then(Value::as_str)
                {
                    self.raw_session_id = Some(sid.to_owned());
                }
            }

            if line_type == "session_meta" {
                let raw_sid = payload
                    .get("session_id")
                    .or_else(|| payload.get("id"))
                    .and_then(Value::as_str)
                    .unwrap_or_else(|| codex_session_id_from_path(&ctx.source_path));
                self.raw_session_id = Some(raw_sid.to_owned());

                let model = payload
                    .get("base_instructions")
                    .and_then(|b| b.get("provenance"))
                    .and_then(|p| p.get("model"))
                    .and_then(Value::as_str)
                    .or_else(|| payload.get("model").and_then(Value::as_str));
                if let Some(m) = model {
                    self.active_model = Some(m.to_owned());
                }

                let cwd = payload
                    .get("cwd")
                    .and_then(Value::as_str)
                    .or(ctx.fallback_cwd.as_deref())
                    .unwrap_or("/unknown");
                let git_obj = payload.get("git").and_then(Value::as_object);
                let repo_url = git_obj
                    .and_then(|g| g.get("repository_url"))
                    .and_then(Value::as_str);
                let branch = git_obj
                    .and_then(|g| g.get("branch"))
                    .and_then(Value::as_str);

                let project = Project::from_origin(repo_url, cwd, branch);
                if !batch.projects.iter().any(|p| p.id == project.id) {
                    batch.projects.push(project.clone());
                }

                let originator = payload
                    .get("originator")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let source = payload.get("source").and_then(Value::as_str).unwrap_or("");
                let surface = if originator.to_ascii_lowercase().contains("desktop") {
                    Surface::DesktopApp
                } else if source.contains("vscode") || source.contains("cursor") {
                    Surface::Ide
                } else {
                    Surface::Cli
                };

                let session_id = Session::compose_id(AgentId::Codex, raw_sid);
                current_session = Some(Session {
                    id: session_id,
                    agent: AgentId::Codex,
                    surface,
                    project_id: project.id,
                    parent_session_id: None,
                    model: self.active_model.clone(),
                    title: None,
                    state: SessionState::Thinking,
                    pid: None,
                    started_at: ts,
                    last_event_at: ts,
                });
            } else {
                let raw_sid = self
                    .raw_session_id
                    .as_deref()
                    .unwrap_or_else(|| codex_session_id_from_path(&ctx.source_path));
                let session_id = Session::compose_id(AgentId::Codex, raw_sid);

                if line_type == "response_item"
                    && payload.get("type").and_then(Value::as_str) == Some("message")
                {
                    let role = match payload.get("role").and_then(Value::as_str) {
                        Some("user") => Some(spawnt_core::ChatRole::User),
                        Some("assistant") => Some(spawnt_core::ChatRole::Assistant),
                        _ => None,
                    };
                    if let (Some(role), Some(content)) =
                        (role, payload.get("content").and_then(Value::as_array))
                    {
                        let text = content
                            .iter()
                            .enumerate()
                            .filter(|(index, _)| {
                                let provenance = payload
                                    .get("internal_chat_message_metadata_passthrough")
                                    .and_then(|meta| meta.get("content_item_metadata"))
                                    .and_then(|items| items.get(*index))
                                    .and_then(|item| item.get("provenance"))
                                    .and_then(|value| value.get("type"))
                                    .and_then(Value::as_str);
                                role != spawnt_core::ChatRole::User
                                    || !matches!(provenance, Some("agents_md" | "harness"))
                            })
                            .filter_map(|(_, part)| {
                                match part.get("type").and_then(Value::as_str) {
                                    Some("input_text" | "output_text") => {
                                        part.get("text").and_then(Value::as_str)
                                    }
                                    _ => None,
                                }
                            })
                            .map(|text| {
                                if role == spawnt_core::ChatRole::User {
                                    clean_user_text(text)
                                } else {
                                    text
                                }
                            })
                            .filter(|text| !text.is_empty())
                            .collect::<Vec<_>>()
                            .join("\n");
                        if !text.is_empty() {
                            batch.chat_events.push(spawnt_core::ChatEvent {
                                event_id: format!("{}:{}", session_id, current_offset),
                                session_id: session_id.clone(),
                                turn_index: i64::try_from(current_offset).unwrap_or(i64::MAX),
                                role,
                                content: spawnt_core::redact::redact_chat_content(&text),
                                tool_name: None,
                                status: None,
                                created_at: ts,
                            });
                        }
                    }
                }

                if batch.projects.is_empty() {
                    batch.projects.push(default_project.clone());
                }

                let sess = current_session.get_or_insert_with(|| Session {
                    id: session_id.clone(),
                    agent: AgentId::Codex,
                    surface: Surface::DesktopApp,
                    project_id: default_project.id.clone(),
                    parent_session_id: None,
                    model: self.active_model.clone(),
                    title: None,
                    state: SessionState::Thinking,
                    pid: None,
                    started_at: ts,
                    last_event_at: ts,
                });
                sess.last_event_at = ts;
                sess.state = SessionState::Thinking;
                if line_type == "event_msg"
                    && matches!(
                        payload.get("type").and_then(Value::as_str),
                        Some("task_complete" | "turn_aborted" | "token_count")
                    )
                {
                    sess.state = SessionState::Idle;
                }

                let turn_model = payload.get("model").and_then(Value::as_str).or_else(|| {
                    payload
                        .get("thread_settings")
                        .and_then(|ts_obj| ts_obj.get("model"))
                        .and_then(Value::as_str)
                });
                if let Some(m) = turn_model {
                    if !m.trim().is_empty() {
                        if line_type == "turn_context" && self.last_turn_model.as_deref() != Some(m)
                        {
                            let (content, status) = self.last_turn_model.as_ref().map_or_else(
                                || (format!("Model active: {m}"), "model_active"),
                                |previous| {
                                    (
                                        format!("Model changed from {previous} to {m}"),
                                        "model_changed",
                                    )
                                },
                            );
                            batch.chat_events.push(spawnt_core::ChatEvent {
                                event_id: format!("{session_id}:{current_offset}:model"),
                                session_id: session_id.clone(),
                                turn_index: i64::try_from(current_offset).unwrap_or(i64::MAX),
                                role: spawnt_core::ChatRole::System,
                                content: spawnt_core::redact::redact_chat_content(&content),
                                tool_name: None,
                                status: Some(status.to_owned()),
                                created_at: ts,
                            });
                            self.last_turn_model = Some(m.to_owned());
                        }
                        self.active_model = Some(m.to_owned());
                        sess.model = Some(m.to_owned());
                    }
                }

                let turn_cwd = payload.get("cwd").and_then(Value::as_str).or_else(|| {
                    payload
                        .get("thread_settings")
                        .and_then(|ts_obj| ts_obj.get("cwd"))
                        .and_then(Value::as_str)
                });
                if let Some(cwd_str) = turn_cwd {
                    let proj = Project::from_origin(None, cwd_str, None);
                    if !batch.projects.iter().any(|p| p.id == proj.id) {
                        batch.projects.push(proj.clone());
                    }
                    sess.project_id = proj.id;
                }

                if line_type == "event_msg"
                    && payload.get("type").and_then(Value::as_str) == Some("token_count")
                {
                    let dedup_key = format!("{raw_sid}:{ordinal}");

                    if self.seen_dedup_keys.insert(dedup_key.clone()) {
                        if let Some(info) = payload.get("info").and_then(Value::as_object) {
                            if let Some(last) =
                                info.get("last_token_usage").and_then(Value::as_object)
                            {
                                let model = self
                                    .active_model
                                    .as_deref()
                                    .or(ctx.fallback_model.as_deref())
                                    .unwrap_or("gpt-codex");
                                let context_win =
                                    info.get("model_context_window").and_then(Value::as_u64);
                                let record = build_codex_usage(
                                    dedup_key,
                                    session_id.clone(),
                                    ts,
                                    model,
                                    last,
                                    context_win,
                                );
                                batch.usage_records.push(record);
                            }
                        }
                    }

                    if let Some(rl) = payload.get("rate_limits").and_then(Value::as_object) {
                        let (snapshots, is_limited) = parse_codex_rate_limits(ts, rl);
                        if is_limited {
                            batch.activity_events.push(ActivityEvent {
                                session_id,
                                timestamp: ts,
                                kind: ActivityKind::RateLimited,
                                tool_name: None,
                                summary_redacted: Some("Rate limit reached".to_owned()),
                                duration_ms: None,
                            });
                        }
                        batch.quota_snapshots.extend(snapshots);
                    }
                }
            }
        }

        if let Some(sess) = current_session {
            batch.sessions.push(sess);
        }
        crate::titles::apply_native_titles(&mut batch, AgentId::Codex, chunk);
        batch
    }
}

/// Hides injected Codex context in legacy rows without changing history or paging cursors.
#[must_use]
pub fn clean_stored_conversation(
    mut events: Vec<spawnt_core::ChatEvent>,
) -> Vec<spawnt_core::ChatEvent> {
    for event in &mut events {
        if event.session_id.starts_with("codex:") && event.role == spawnt_core::ChatRole::User {
            event.content = clean_user_text(&event.content).to_owned();
        }
    }
    events
}

fn build_codex_usage(
    dedup_key: String,
    session_id: String,
    timestamp: DateTime<Utc>,
    model: &str,
    last: &serde_json::Map<String, Value>,
    explicit_context_window: Option<u64>,
) -> UsageRecord {
    let total_input = last
        .get("input_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let cache_read = last
        .get("cached_input_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let cache_write = last
        .get("cache_write_input_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let uncached_input = total_input.saturating_sub(cache_read);
    let output_tokens = last
        .get("output_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let reasoning_tokens = last
        .get("reasoning_output_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);

    let breakdown = TokenBreakdown {
        input_tokens: uncached_input,
        output_tokens,
        reasoning_tokens,
        cache_read_tokens: cache_read,
        cache_write_5m_tokens: cache_write,
        cache_write_1h_tokens: 0,
    };
    let eval = calculate_cost(model, &breakdown);

    UsageRecord {
        dedup_key,
        session_id,
        timestamp,
        model: model.to_owned(),
        input_tokens: total_input,
        output_tokens,
        reasoning_tokens,
        cache_read_tokens: cache_read,
        cache_write_5m_tokens: cache_write,
        cache_write_1h_tokens: 0,
        // `total_token_usage` is cumulative across the rollout, including
        // context that has since been compacted. `last_token_usage` is the
        // current request's context snapshot and resets after compaction.
        context_tokens: last
            .get("total_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(total_input),
        context_window: explicit_context_window.or(eval.context_window),
        cost_usd: eval.cost_usd,
        cache_savings_usd: eval.cache_savings_usd,
        cost_basis: eval.cost_basis,
        pricing_version: eval.pricing_version.to_owned(),
    }
}

fn parse_codex_rate_limits(
    observed_at: DateTime<Utc>,
    rl: &serde_json::Map<String, Value>,
) -> (Vec<QuotaSnapshot>, bool) {
    let mut out = Vec::new();
    let limit_id = rl
        .get("limit_id")
        .and_then(Value::as_str)
        .unwrap_or("codex");
    let plan = rl
        .get("plan_type")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let is_rate_limited = rl
        .get("rate_limit_reached_type")
        .is_some_and(|v| !v.is_null());

    for (key, default_window) in [
        ("primary", QuotaWindow::FiveHour),
        ("secondary", QuotaWindow::Weekly),
    ] {
        if let Some(win_obj) = rl.get(key).and_then(Value::as_object) {
            let used_pct = win_obj
                .get("used_percent")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            let resets_at = win_obj
                .get("resets_at")
                .and_then(Value::as_i64)
                .and_then(|secs| Utc.timestamp_opt(secs, 0).single());
            out.push(QuotaSnapshot {
                provider: "codex".to_owned(),
                limit_id: limit_id.to_owned(),
                window: default_window,
                used_pct,
                resets_at,
                plan: plan.clone(),
                basis: TrustLevel::Exact,
                observed_at,
            });
        }
    }
    (out, is_rate_limited)
}

/// Extracts the 36-char session UUID from a Codex rollout filename
/// (`rollout-YYYY-MM-DDTHH-MM-SS-<uuid>.jsonl`), or falls back to the file stem.
#[must_use]
pub fn codex_session_id_from_path(path: &str) -> &str {
    let stem = fallback_id_from_path(path);
    if stem.len() >= 36 {
        let candidate = &stem[stem.len() - 36..];
        let bytes = candidate.as_bytes();
        if bytes[8] == b'-' && bytes[13] == b'-' && bytes[18] == b'-' && bytes[23] == b'-' {
            return candidate;
        }
    }
    stem
}
