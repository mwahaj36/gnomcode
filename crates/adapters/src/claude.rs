//! Claude Code JSONL log adapter (`SPEC §2.1`, `§3.5`).
//!
//! Reads `~/.claude/projects/**/*.jsonl`. Deduplicates assistant blocks by
//! `format!("{}:{}", message.id, request_id)` because Claude Code writes one
//! JSONL line per content block and copies prior history on `--resume`.

use crate::{FileCtx, FileKind, ParsedBatch, SourceAdapter};
use chrono::{DateTime, Utc};
use gnomcode_core::{
    ActivityEvent, ActivityKind, AgentId, ChatEvent, ChatRole, Project, QuotaSnapshot, QuotaWindow,
    Session, SessionState, Surface, TrustLevel, UsageRecord,
    pricing::{TokenBreakdown, calculate_cost},
    redact::{redact_chat_content, redact_summary},
};
use serde_json::Value;
use std::collections::HashSet;

/// Stateful parser for a Claude Code `.jsonl` session log.
#[derive(Debug, Default)]
pub struct ClaudeAdapter {
    seen_dedup_keys: HashSet<String>,
    parent_session_id: Option<String>,
    subagent_title: Option<String>,
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
        let mut sessions: Vec<Session> = Vec::new();
        let mut line_offset = ctx.source_offset;
        let path_agent = subagent_identity(&ctx.source_path);

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
            if matches!(
                obj.get("type").and_then(Value::as_str),
                Some("custom-title" | "summary")
            ) {
                continue;
            }

            let parsed_ts = obj
                .get("timestamp")
                .and_then(Value::as_str)
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc));

            // Session-less bookkeeping (for example `file-history-snapshot`) belongs to the
            // session already open in this chunk; it never opens one of its own.
            if obj.get("sessionId").is_none() && path_agent.is_none() && !sessions.is_empty() {
                continue;
            }

            let raw_sid = obj
                .get("sessionId")
                .and_then(Value::as_str)
                .or(path_agent.as_ref().map(|(parent, _)| parent.as_str()))
                .unwrap_or_else(|| fallback_id_from_path(&ctx.source_path));
            // Subagent transcripts repeat the parent's sessionId; their own identity is the
            // file name (`subagents/agent-<id>.jsonl`) or agentId. Bookkeeping lines such as
            // `api-request` carry agentId without isSidechain, so only an explicit
            // `isSidechain:false` marks a line as parent-owned.
            let sidechain_agent = path_agent
                .as_ref()
                .map(|(_, stem)| stem.clone())
                .or_else(|| {
                    obj.get("agentId")
                        .and_then(Value::as_str)
                        .filter(|id| {
                            !id.is_empty() && obj.get("isSidechain") != Some(&Value::Bool(false))
                        })
                        .map(|id| format!("agent-{id}"))
                });
            let session_id = match sidechain_agent {
                Some(agent) => {
                    if self.parent_session_id.is_none() {
                        self.parent_session_id =
                            Some(Session::compose_id(AgentId::ClaudeCode, raw_sid));
                    }
                    Session::compose_id(AgentId::ClaudeCode, &agent)
                }
                None => Session::compose_id(AgentId::ClaudeCode, raw_sid),
            };
            // A session is never its own parent; such rows vanish from every session list.
            let parent_for_session = self
                .parent_session_id
                .clone()
                .filter(|parent| *parent != session_id);

            let cwd = obj
                .get("cwd")
                .and_then(Value::as_str)
                .or(ctx.fallback_cwd.as_deref())
                .unwrap_or("/unknown");
            let git_branch = obj.get("gitBranch").and_then(Value::as_str);

            // Only lines that name their host decide the surface; `api-request` and similar
            // bookkeeping lines carry neither field and must not reset it to Cli.
            let line_surface = obj
                .get("entrypoint")
                .and_then(Value::as_str)
                .map(|ep| {
                    if ep.contains("vscode") || ep.contains("cursor") || ep.contains("ide") {
                        Surface::Ide
                    } else if ep.contains("desktop") {
                        Surface::DesktopApp
                    } else {
                        Surface::Cli
                    }
                })
                .or_else(|| {
                    (obj.get("clientPlatform").and_then(Value::as_str) == Some("desktop_app"))
                        .then_some(Surface::DesktopApp)
                });

            let project = Project::from_origin(None, cwd, git_branch);
            if !batch.projects.iter().any(|p| p.id == project.id) {
                batch.projects.push(project.clone());
            }

            if self.parent_session_id.is_none() {
                let parent_sid = obj
                    .get("parentSessionId")
                    .or_else(|| obj.get("parentId"))
                    .and_then(Value::as_str)
                    .filter(|s| !s.trim().is_empty())
                    .map(|pid| Session::compose_id(AgentId::ClaudeCode, pid));
                if parent_sid.is_some() {
                    self.parent_session_id.clone_from(&parent_sid);
                }
            }
            if parent_for_session.is_some() && self.subagent_title.is_none() {
                let subagent_name = obj
                    .get("subagentName")
                    .or_else(|| obj.get("agentName"))
                    .or_else(|| obj.get("agent_type"))
                    .or_else(|| obj.get("agentRole"))
                    .and_then(Value::as_str);
                if let Some(name) = subagent_name {
                    self.subagent_title = Some(format!("{name} · Subagent"));
                } else {
                    self.subagent_title = Some("Subagent".to_owned());
                }
            }

            let initial_ts = parsed_ts.unwrap_or_else(Utc::now);
            let index = if let Some(index) = sessions.iter().position(|s| s.id == session_id) {
                index
            } else {
                sessions.push(Session {
                    id: session_id.clone(),
                    agent: AgentId::ClaudeCode,
                    surface: line_surface.unwrap_or(Surface::Cli),
                    project_id: project.id.clone(),
                    parent_session_id: parent_for_session.clone(),
                    model: None,
                    title: parent_for_session.as_ref().and(self.subagent_title.clone()),
                    state: SessionState::Idle,
                    pid: None,
                    started_at: initial_ts,
                    last_event_at: initial_ts,
                });
                sessions.len() - 1
            };
            let sess = &mut sessions[index];
            if parent_for_session.is_some() && sess.parent_session_id.is_none() {
                sess.parent_session_id.clone_from(&parent_for_session);
            }
            if parent_for_session.is_some() && sess.title.is_none() {
                sess.title.clone_from(&self.subagent_title);
            }
            if let Some(valid_ts) = parsed_ts {
                sess.last_event_at = valid_ts;
            }
            if let Some(surface) = line_surface {
                sess.surface = surface;
            }
            let ts = sess.last_event_at;

            let role = match obj.get("type").and_then(Value::as_str) {
                Some("user") => Some(ChatRole::User),
                Some("assistant") => Some(ChatRole::Assistant),
                _ => None,
            };
            if let (Some(role), Some(message)) = (role, obj.get("message")) {
                sess.state = SessionState::Thinking;
                let content = message.get("content");
                let text = match content {
                    Some(Value::String(text)) => text.clone(),
                    Some(Value::Array(blocks)) => blocks
                        .iter()
                        .filter(|block| block.get("type").and_then(Value::as_str) == Some("text"))
                        .filter_map(|block| block.get("text").and_then(Value::as_str))
                        .collect::<Vec<_>>()
                        .join("\n"),
                    _ => String::new(),
                };
                if !text.is_empty() {
                    let event_key = obj
                        .get("uuid")
                        .and_then(Value::as_str)
                        .map_or_else(|| current_offset.to_string(), str::to_owned);
                    batch.chat_events.push(ChatEvent {
                        event_id: format!("{session_id}:{event_key}"),
                        session_id: session_id.clone(),
                        turn_index: i64::try_from(current_offset).unwrap_or(i64::MAX),
                        role,
                        content: redact_chat_content(&text),
                        tool_name: None,
                        status: None,
                        created_at: ts,
                    });
                }
            }

            if obj.get("type").and_then(Value::as_str) == Some("assistant") {
                if let Some(msg) = obj.get("message").and_then(Value::as_object) {
                    if let Some(model) = msg.get("model").and_then(Value::as_str) {
                        if model != "<synthetic>" {
                            sess.model = Some(model.to_owned());
                        }
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
                                sess.state = if name == "AskUserQuestion" {
                                    SessionState::AwaitingApproval
                                } else {
                                    SessionState::Tool
                                };
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
                    if msg.get("stop_reason").and_then(Value::as_str) == Some("end_turn") {
                        sess.state = SessionState::Idle;
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
                                    session_id.clone(),
                                    ts,
                                    sess.model.as_deref().unwrap_or("unknown"),
                                    usage_obj,
                                );
                                batch.usage_records.push(record);
                            }
                        }
                    }
                }
            }
            handle_claude_error_or_rate_limit(obj, sess, &session_id, ts, &mut batch);
        }

        batch.sessions.extend(sessions);
        crate::titles::apply_native_titles(&mut batch, AgentId::ClaudeCode, chunk);
        batch
    }
}

/// Splits `.../<session-id>/subagents/agent-<id>.jsonl` into the parent id and the child file stem.
fn subagent_identity(path: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = path
        .split(['/', '\\'])
        .filter(|part| !part.is_empty())
        .collect();
    let [.., parent, folder, file] = parts.as_slice() else {
        return None;
    };
    let stem = file.strip_suffix(".jsonl")?;
    (*folder == "subagents" && stem.starts_with("agent-"))
        .then(|| ((*parent).to_owned(), stem.to_owned()))
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
            let name = name.strip_suffix(".zst").unwrap_or(name);
            name.strip_suffix(".jsonl")
                .or_else(|| name.strip_suffix(".json"))
        })
        .unwrap_or("unknown")
}

/// Converts the limits file written by `gnomcode statusline` into exact account-wide quota rows.
///
/// Claude reports `rate_limits.{five_hour,seven_day}.{used_percentage,resets_at}` (epoch seconds)
/// for Pro/Max plans; absent windows stay absent rather than becoming zero.
#[must_use]
pub fn parse_status_limits(bytes: &[u8]) -> Vec<QuotaSnapshot> {
    let Ok(value) = serde_json::from_slice::<Value>(bytes) else {
        return Vec::new();
    };
    let Some(observed_at) = value
        .get("observed_at")
        .and_then(Value::as_str)
        .and_then(|at| DateTime::parse_from_rfc3339(at).ok())
        .map(|at| at.with_timezone(&Utc))
    else {
        return Vec::new();
    };
    [
        ("five_hour", QuotaWindow::FiveHour),
        ("seven_day", QuotaWindow::Weekly),
    ]
    .into_iter()
    .filter_map(|(key, window)| {
        let limit = value.get("rate_limits")?.get(key)?;
        let used_pct = limit.get("used_percentage")?.as_f64()?;
        Some(QuotaSnapshot {
            provider: AgentId::ClaudeCode.as_str().to_owned(),
            limit_id: "claude".to_owned(),
            window,
            used_pct: used_pct.clamp(0.0, 100.0),
            resets_at: limit
                .get("resets_at")
                .and_then(Value::as_i64)
                .and_then(|at| DateTime::from_timestamp(at, 0)),
            plan: None,
            basis: TrustLevel::Exact,
            observed_at,
        })
    })
    .collect()
}

/// Converts Anthropic's OAuth usage response into exact account-wide quota rows.
///
/// Windows: `five_hour`, `seven_day` and per-model weeklies (`seven_day_opus`, `seven_day_sonnet`),
/// each `{utilization: 0..=100, resets_at: RFC3339}`. Null or missing windows stay absent.
#[must_use]
pub fn parse_oauth_usage(value: &Value, observed_at: DateTime<Utc>) -> Vec<QuotaSnapshot> {
    [
        ("five_hour", "claude", QuotaWindow::FiveHour),
        ("seven_day", "claude", QuotaWindow::Weekly),
        ("seven_day_opus", "opus", QuotaWindow::Weekly),
        ("seven_day_sonnet", "sonnet", QuotaWindow::Weekly),
    ]
    .into_iter()
    .filter_map(|(key, limit_id, window)| {
        let limit = value.get(key)?;
        Some(QuotaSnapshot {
            provider: AgentId::ClaudeCode.as_str().to_owned(),
            limit_id: limit_id.to_owned(),
            window,
            used_pct: limit.get("utilization")?.as_f64()?.clamp(0.0, 100.0),
            resets_at: limit
                .get("resets_at")
                .and_then(Value::as_str)
                .and_then(|at| DateTime::parse_from_rfc3339(at).ok())
                .map(|at| at.with_timezone(&Utc)),
            plan: None,
            basis: TrustLevel::Exact,
            observed_at,
        })
    })
    .collect()
}

fn handle_claude_error_or_rate_limit(
    obj: &serde_json::Map<String, Value>,
    sess: &mut Session,
    session_id: &str,
    ts: DateTime<Utc>,
    batch: &mut ParsedBatch,
) {
    let record_type = obj.get("type").and_then(Value::as_str);
    if record_type == Some("rate_limit_event") {
        if let Some(info) = obj.get("rate_limit_info").and_then(Value::as_object) {
            record_claude_rate_limit(info, sess, session_id, ts, batch);
        }
        return;
    }

    if record_type == Some("result") && obj.get("is_error").and_then(Value::as_bool) == Some(true) {
        let result_text = obj.get("result").and_then(Value::as_str).unwrap_or("");
        let lower = result_text.to_ascii_lowercase();
        let is_limit = lower.contains("rate limit")
            || lower.contains("usage limit")
            || lower.contains("hit your limit")
            || lower.contains("429");
        emit_claude_error(
            sess,
            session_id,
            ts,
            batch,
            is_limit,
            None,
            Some(result_text),
        );
        return;
    }

    if obj.get("isApiErrorMessage").and_then(Value::as_bool) != Some(true) {
        return;
    }

    let status = obj.get("apiErrorStatus").and_then(Value::as_u64);
    let err_kind = obj.get("error").and_then(Value::as_str);
    let api_err = obj.get("apiError").and_then(Value::as_str);
    let limit_info = obj
        .get("quotaLimits")
        .and_then(Value::as_object)
        .or_else(|| {
            obj.get("apiErrorParams")
                .and_then(|p| p.get("rate_limit_info"))
                .and_then(Value::as_object)
        });
    let rejected_limit = limit_info
        .is_some_and(|info| info.get("status").and_then(Value::as_str) == Some("rejected"));
    let is_limit = status == Some(429)
        || rejected_limit
        || err_kind == Some("rate_limit")
        || api_err.is_some_and(|s| s.contains("limit"));

    if is_limit {
        if let Some(info) = limit_info {
            if record_claude_rate_limit(info, sess, session_id, ts, batch) {
                return;
            }
        }
        emit_claude_error(sess, session_id, ts, batch, true, status, err_kind);
    } else {
        emit_claude_error(
            sess,
            session_id,
            ts,
            batch,
            false,
            status,
            err_kind.or(api_err),
        );
    }
}

fn record_claude_rate_limit(
    info: &serde_json::Map<String, Value>,
    sess: &mut Session,
    session_id: &str,
    ts: DateTime<Utc>,
    batch: &mut ParsedBatch,
) -> bool {
    if info.get("status").and_then(Value::as_str) != Some("rejected") {
        return false;
    }
    let window = match info.get("rateLimitType").and_then(Value::as_str) {
        Some(kind) if kind.contains("seven_day") || kind.contains("weekly") => QuotaWindow::Weekly,
        _ => QuotaWindow::FiveHour,
    };
    let resets_at = info
        .get("resetsAt")
        .and_then(Value::as_i64)
        .and_then(|sec| DateTime::from_timestamp(sec, 0));
    batch.quota_snapshots.push(QuotaSnapshot {
        provider: AgentId::ClaudeCode.as_str().to_owned(),
        limit_id: "claude".to_owned(),
        window,
        used_pct: 100.0,
        resets_at,
        plan: None,
        basis: TrustLevel::Exact,
        observed_at: ts,
    });
    emit_claude_error(sess, session_id, ts, batch, true, Some(429), None);
    true
}

fn emit_claude_error(
    sess: &mut Session,
    session_id: &str,
    ts: DateTime<Utc>,
    batch: &mut ParsedBatch,
    is_limit: bool,
    status: Option<u64>,
    hint: Option<&str>,
) {
    sess.state = SessionState::Error;
    let kind = if is_limit {
        ActivityKind::RateLimited
    } else {
        ActivityKind::Error
    };
    let friendly = if is_limit {
        "Claude usage limit reached"
    } else {
        match (status, hint) {
            (Some(401), _) | (_, Some("authentication_failed")) => {
                "Authentication failed; run claude auth login"
            }
            (Some(403), _) | (_, Some("billing_error")) => {
                "Billing or permission error; check Claude account"
            }
            (Some(404), _) | (_, Some("model_not_found")) => "Requested model unavailable",
            (Some(529), _) => "Anthropic API overloaded; retry shortly",
            (Some(500..=599), _) | (_, Some("server_error")) => "Anthropic API server error",
            _ => "Claude API error",
        }
    };
    let label = if is_limit { "rate_limit" } else { "api_error" };
    batch.activity_events.push(ActivityEvent {
        session_id: session_id.to_owned(),
        timestamp: ts,
        kind,
        tool_name: None,
        summary_redacted: Some(redact_summary(label, friendly)),
        duration_ms: None,
    });
}
