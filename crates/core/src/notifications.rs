//! Pure notification decision engine and payload redaction (`SPEC §3.14.3`).
//!
//! Decides when to notify for the four Needs-you / lifecycle events, applies
//! deduplication, 5-minute per-(session, event) rate limiting, burst grouping,
//! and strict payload redaction (only session title, agent, and event; never chat).

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::presence::{SessionPresence, classify_presence};
use crate::quota::{QuotaState, quota_state, reset_passed};
use crate::redact::redact_chat_content;
use crate::{AgentId, QuotaSnapshot, QuotaWindow, Session, SessionState, TrustLevel};

/// Minimum seconds between two notifications for the same `(subject, event)`.
pub const RATE_LIMIT_WINDOW_SECS: i64 = 300;
/// Maximum character length of a redacted session title in notifications.
pub const MAX_TITLE_CHARS: usize = 64;
/// Maximum character length of a formatted desktop notification message.
pub const MAX_DESKTOP_CHARS: usize = 240;
/// Seconds a session must remain `Idle` after busy work before triggering turn-finished.
const TURN_SETTLE_SECS: i64 = 8;
/// Maximum age of an observation on first transition before suppressing stale startup alerts.
const MAX_FRESH_EVENT_SECS: i64 = 300;
/// Cap in-memory dedup history size to keep daemon memory bounded.
const MAX_DEDUP_IDS: usize = 2048;

/// Supported notification event types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationEventKind {
    /// Agent is waiting on user approval or permission.
    ApprovalWaiting,
    /// Agent completed a turn lasting longer than the configured threshold.
    TurnFinished,
    /// Provider quota entered the `Low` state (`< 20%` remaining).
    LimitLow,
    /// Provider quota reset (`reset_passed` followed by a fresh reading).
    LimitReset,
    /// A working or waiting session's process exited or the session ended.
    SessionStopped,
    /// A session hit an error that needs the user.
    NeedsAttention,
}

impl NotificationEventKind {
    /// Stable identifier used in API payloads and settings.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ApprovalWaiting => "approval_waiting",
            Self::TurnFinished => "turn_finished",
            Self::LimitLow => "limit_low",
            Self::LimitReset => "limit_reset",
            Self::SessionStopped => "session_stopped",
            Self::NeedsAttention => "needs_attention",
        }
    }

    /// Short plain-text event label without emojis.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::ApprovalWaiting => "Needs approval",
            Self::TurnFinished => "Turn finished",
            Self::LimitLow => "Limit low",
            Self::LimitReset => "Limit reset",
            Self::SessionStopped => "Agent stopped",
            Self::NeedsAttention => "Needs attention",
        }
    }
}

/// Persisted notification preferences (`SPEC §3.14.3`, `§3.16`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct NotificationSettings {
    /// Notify when a session needs user approval.
    pub approval_waiting: bool,
    /// Notify when a turn finishes after more than `turn_finished_min_minutes`.
    pub turn_finished: bool,
    /// Minimum turn duration (minutes, strictly exceeded) before notifying.
    pub turn_finished_min_minutes: u32,
    /// Notify when a provider quota window enters `QuotaState::Low`.
    pub limit_low: bool,
    /// Notify when a provider quota window passes reset and receives a fresh reading.
    pub limit_reset: bool,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            approval_waiting: true,
            turn_finished: true,
            turn_finished_min_minutes: 2,
            limit_low: true,
            limit_reset: true,
        }
    }
}

impl NotificationSettings {
    /// Returns whether notifications for `kind` are enabled.
    #[must_use]
    pub const fn is_enabled(&self, kind: NotificationEventKind) -> bool {
        match kind {
            NotificationEventKind::ApprovalWaiting => self.approval_waiting,
            NotificationEventKind::TurnFinished => self.turn_finished,
            NotificationEventKind::LimitLow => self.limit_low,
            NotificationEventKind::LimitReset => self.limit_reset,
            // Stops share the turn-finished preference; errors share the needs-you preference.
            NotificationEventKind::SessionStopped => self.turn_finished,
            NotificationEventKind::NeedsAttention => self.approval_waiting,
        }
    }

    /// Clamps user-editable numeric settings to safe bounds.
    #[must_use]
    pub fn sanitized(mut self) -> Self {
        self.turn_finished_min_minutes = self.turn_finished_min_minutes.min(1440);
        self
    }
}

/// Strictly redacted notification payload containing zero chat content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactedNotificationPayload {
    /// Triggered event kind.
    pub event: NotificationEventKind,
    /// Human-readable agent or provider name.
    pub agent: String,
    /// Redacted session title (or short quota window label; empty if unavailable).
    pub session_title: String,
}

/// A single notification decision emitted by [`NotificationEngine`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationDecision {
    /// Stable deduplication key for storage/push idempotency.
    pub dedup_id: String,
    /// Session or conversation ID (`""` for provider-level quota events).
    pub conversation_id: String,
    /// Redacted payload (`event`, `agent`, `session_title`).
    pub payload: RedactedNotificationPayload,
}

/// Grouped output from one evaluation tick for desktop and Web Push delivery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationBatch {
    /// Individual decisions included in this tick (1 or more).
    pub decisions: Vec<NotificationDecision>,
    /// Single-line desktop notification message (`<= 240` chars, no control chars or emojis).
    pub desktop_message: String,
    /// Deduplication ID sent to `/api/push/send` and stored in `automation_notice`.
    pub push_event_id: String,
    /// Primary event kind for push rendering.
    pub push_event: NotificationEventKind,
    /// Agent label (or `"Agents"` on mixed burst) for push rendering.
    pub push_agent: String,
    /// Redacted session title (or grouped summary label like `"2 sessions"`) for push rendering.
    pub push_session_title: String,
}

/// Returns human-readable provider/agent name without emojis.
#[must_use]
pub fn agent_display_name(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "claude-code" | "claude" | "anthropic" => "Claude Code".to_owned(),
        "codex" | "openai" => "Codex".to_owned(),
        "antigravity" => "Antigravity".to_owned(),
        "" => "Agent".to_owned(),
        other => sanitize_plain_text(other, 32),
    }
}

const fn agent_id_display(agent: AgentId) -> &'static str {
    match agent {
        AgentId::ClaudeCode => "Claude Code",
        AgentId::Codex => "Codex",
        AgentId::Antigravity => "Antigravity",
    }
}

const fn quota_window_label(window: QuotaWindow) -> &'static str {
    match window {
        QuotaWindow::FiveHour => "5h quota",
        QuotaWindow::Weekly => "Weekly quota",
        QuotaWindow::Monthly => "Monthly quota",
    }
}

fn sanitize_plain_text(raw: &str, max_chars: usize) -> String {
    let redacted = redact_chat_content(raw);
    let cleaned: String = redacted
        .chars()
        .map(|ch| {
            if ch.is_control() {
                ' '
            } else if ch.is_ascii() || ch.is_alphanumeric() {
                ch
            } else {
                ' '
            }
        })
        .collect();
    let collapsed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    truncate_chars(&collapsed, max_chars)
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_owned();
    }
    let keep = max_chars.saturating_sub(3);
    let prefix: String = text.chars().take(keep).collect();
    format!("{}...", prefix.trim_end())
}

/// Redacts a session title for notification delivery (`<= 64` chars, single line, no secrets/emojis).
#[must_use]
pub fn redact_notification_title(raw: Option<&str>) -> String {
    let cleaned = raw
        .map(|value| sanitize_plain_text(value, MAX_TITLE_CHARS))
        .unwrap_or_default();
    if cleaned.is_empty() {
        "Untitled session".to_owned()
    } else {
        cleaned
    }
}

/// Formats one or more redacted payloads into a single-line desktop notification (`<= 240` chars).
#[must_use]
pub fn format_desktop_notification(payloads: &[RedactedNotificationPayload]) -> String {
    let raw = match payloads {
        [] => return String::new(),
        [single] => format_single_payload(single),
        many => {
            let items = many
                .iter()
                .map(|p| {
                    if p.session_title.is_empty() {
                        format!("[{}] ({})", p.agent, p.event.label())
                    } else {
                        format!("[{}] {} ({})", p.agent, p.session_title, p.event.label())
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("{} updates: {}", many.len(), items)
        }
    };
    truncate_chars(&raw, MAX_DESKTOP_CHARS)
}

fn format_single_payload(payload: &RedactedNotificationPayload) -> String {
    if payload.session_title.is_empty() {
        format!("[{}] {}", payload.agent, payload.event.label())
    } else {
        format!(
            "[{}] {}: {}",
            payload.agent,
            payload.session_title,
            payload.event.label()
        )
    }
}

#[derive(Debug, Clone)]
struct SessionWatchState {
    last_state: SessionState,
    busy_since: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
struct QuotaWatchState {
    saw_reset_passed: bool,
    last_resets_at: Option<DateTime<Utc>>,
    last_observed_at: DateTime<Utc>,
    last_low_cycle: Option<String>,
}

/// Pure stateful notification coordinator.
#[derive(Debug, Default, Clone)]
pub struct NotificationEngine {
    sessions: HashMap<String, SessionWatchState>,
    quotas: HashMap<String, QuotaWatchState>,
    last_sent: HashMap<(String, NotificationEventKind), DateTime<Utc>>,
    seen_dedup_ids: HashSet<String>,
}

impl NotificationEngine {
    /// Creates an empty notification engine.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Evaluates active sessions and latest quota snapshots at `now`.
    ///
    /// Returns one [`NotificationBatch`] when at least one non-duplicate,
    /// non-rate-limited event qualifies; bursts in the same tick are grouped.
    pub fn evaluate(
        &mut self,
        sessions: &[Session],
        quotas: &[QuotaSnapshot],
        settings: &NotificationSettings,
        now: DateTime<Utc>,
    ) -> Option<NotificationBatch> {
        let mut decisions = Vec::new();
        self.evaluate_sessions(sessions, settings, now, &mut decisions);
        self.evaluate_quotas(quotas, settings, now, &mut decisions);
        build_batch(decisions, now)
    }

    /// Hook point for live approval events (e.g., `PermissionRequest` / phone approvals).
    ///
    /// External approval hooks can call this immediately before database polling catches up.
    /// Recording state here also deduplicates the subsequent `evaluate()` poll.
    pub fn record_approval_waiting_hint(
        &mut self,
        session: &Session,
        settings: &NotificationSettings,
        now: DateTime<Utc>,
    ) -> Option<NotificationBatch> {
        if session.parent_session_id.is_some() {
            return None;
        }
        let existing_busy = self.sessions.get(&session.id).and_then(|s| s.busy_since);
        self.sessions.insert(
            session.id.clone(),
            SessionWatchState {
                last_state: SessionState::AwaitingApproval,
                busy_since: existing_busy,
            },
        );
        if !settings.is_enabled(NotificationEventKind::ApprovalWaiting) {
            return None;
        }
        let dedup_id = format!(
            "approval:{}:{}",
            session.id,
            session.last_event_at.to_rfc3339()
        );
        let decision = self.try_accept_decision(
            &session.id,
            &session.id,
            dedup_id,
            RedactedNotificationPayload {
                event: NotificationEventKind::ApprovalWaiting,
                agent: agent_id_display(session.agent).to_owned(),
                session_title: redact_notification_title(session.title.as_deref()),
            },
            now,
        )?;
        build_batch(vec![decision], now)
    }

    fn evaluate_sessions(
        &mut self,
        sessions: &[Session],
        settings: &NotificationSettings,
        now: DateTime<Utc>,
        out: &mut Vec<NotificationDecision>,
    ) {
        let mut next_sessions = HashMap::with_capacity(sessions.len());
        let settle = Duration::seconds(TURN_SETTLE_SECS);
        let fresh = Duration::seconds(MAX_FRESH_EVENT_SECS);
        let min_turn = Duration::minutes(i64::from(settings.turn_finished_min_minutes.min(1440)));

        for session in sessions.iter().filter(|s| s.parent_session_id.is_none()) {
            let prev = self.sessions.get(&session.id).cloned();

            if matches!(session.state, SessionState::Thinking | SessionState::Tool) {
                let busy_since = prev
                    .as_ref()
                    .and_then(|p| p.busy_since)
                    .unwrap_or_else(|| session.last_event_at.min(now));
                next_sessions.insert(
                    session.id.clone(),
                    SessionWatchState {
                        last_state: session.state,
                        busy_since: Some(busy_since),
                    },
                );
                continue;
            }

            if session.state == SessionState::Idle {
                let was_busy = prev.as_ref().is_some_and(|p| {
                    matches!(p.last_state, SessionState::Thinking | SessionState::Tool)
                });
                if was_busy {
                    if now.signed_duration_since(session.last_event_at) < settle {
                        if let Some(p) = prev {
                            next_sessions.insert(session.id.clone(), p);
                        }
                        continue;
                    }
                    if let Some(busy_since) = prev.as_ref().and_then(|p| p.busy_since) {
                        let elapsed = if session.last_event_at >= busy_since {
                            session.last_event_at.signed_duration_since(busy_since)
                        } else {
                            now.signed_duration_since(busy_since)
                        };
                        if elapsed > min_turn
                            && settings.is_enabled(NotificationEventKind::TurnFinished)
                        {
                            let dedup_id = format!(
                                "completed:{}:{}",
                                session.id,
                                session.last_event_at.to_rfc3339()
                            );
                            if let Some(decision) = self.try_accept_decision(
                                &session.id,
                                &session.id,
                                dedup_id,
                                RedactedNotificationPayload {
                                    event: NotificationEventKind::TurnFinished,
                                    agent: agent_id_display(session.agent).to_owned(),
                                    session_title: redact_notification_title(
                                        session.title.as_deref(),
                                    ),
                                },
                                now,
                            ) {
                                out.push(decision);
                            }
                        }
                    }
                }
                next_sessions.insert(
                    session.id.clone(),
                    SessionWatchState {
                        last_state: SessionState::Idle,
                        busy_since: None,
                    },
                );
                continue;
            }

            // One notification per stop: only a session that was working or waiting reports its
            // exit. A turn that already finished (Idle) and then exits stays silent, so a turn end
            // and a process exit in the same moment never notify twice.
            let fresh_event = now.signed_duration_since(session.last_event_at) <= fresh;
            let prev_state = prev.as_ref().map(|p| p.last_state);
            let stop_kind = match session.state {
                SessionState::Ended
                    if matches!(
                        prev_state,
                        Some(
                            SessionState::Thinking
                                | SessionState::Tool
                                | SessionState::AwaitingApproval
                        )
                    ) =>
                {
                    Some((NotificationEventKind::SessionStopped, "stopped"))
                }
                SessionState::Error if prev_state.is_some_and(|p| p != SessionState::Error) => {
                    Some((NotificationEventKind::NeedsAttention, "error"))
                }
                _ => None,
            };
            if let Some((kind, label)) = stop_kind
                && fresh_event
                && settings.is_enabled(kind)
            {
                let dedup_id = format!(
                    "{label}:{}:{}",
                    session.id,
                    session.last_event_at.to_rfc3339()
                );
                if let Some(decision) = self.try_accept_decision(
                    &session.id,
                    &session.id,
                    dedup_id,
                    RedactedNotificationPayload {
                        event: kind,
                        agent: agent_id_display(session.agent).to_owned(),
                        session_title: redact_notification_title(session.title.as_deref()),
                    },
                    now,
                ) {
                    out.push(decision);
                }
            }

            let age_ms = now
                .signed_duration_since(session.last_event_at)
                .num_milliseconds();
            let presence = classify_presence(session.state.as_str(), Some(age_ms), false);
            if presence == SessionPresence::AwaitingApproval {
                let transitioned = prev
                    .as_ref()
                    .is_none_or(|p| p.last_state != SessionState::AwaitingApproval);
                if transitioned
                    && now.signed_duration_since(session.last_event_at) <= fresh
                    && settings.is_enabled(NotificationEventKind::ApprovalWaiting)
                {
                    let dedup_id = format!(
                        "approval:{}:{}",
                        session.id,
                        session.last_event_at.to_rfc3339()
                    );
                    if let Some(decision) = self.try_accept_decision(
                        &session.id,
                        &session.id,
                        dedup_id,
                        RedactedNotificationPayload {
                            event: NotificationEventKind::ApprovalWaiting,
                            agent: agent_id_display(session.agent).to_owned(),
                            session_title: redact_notification_title(session.title.as_deref()),
                        },
                        now,
                    ) {
                        out.push(decision);
                    }
                }
            }

            next_sessions.insert(
                session.id.clone(),
                SessionWatchState {
                    last_state: session.state,
                    busy_since: prev.and_then(|p| p.busy_since),
                },
            );
        }

        self.sessions = next_sessions;
    }

    fn evaluate_quotas(
        &mut self,
        quotas: &[QuotaSnapshot],
        settings: &NotificationSettings,
        now: DateTime<Utc>,
        out: &mut Vec<NotificationDecision>,
    ) {
        let fresh = Duration::seconds(MAX_FRESH_EVENT_SECS);
        let mut next_quotas = HashMap::with_capacity(quotas.len());

        for quota in quotas.iter().filter(|q| {
            q.basis != TrustLevel::Unknown
                && q.observed_at <= now
                && (0.0..=100.0).contains(&q.used_pct)
        }) {
            let key = format!("{}:{}:{:?}", quota.provider, quota.limit_id, quota.window);
            let prev = self.quotas.get(&key).cloned();
            let passed_now = reset_passed(quota, now);
            let passed_since_prev = prev.as_ref().is_some_and(|p| {
                p.last_resets_at
                    .is_some_and(|r| r <= now && quota.observed_at > r)
            });
            let mut saw_reset_passed = prev.as_ref().is_some_and(|p| p.saw_reset_passed)
                || passed_now
                || passed_since_prev;

            let qstate = quota_state(quota, now);
            let fresh_reading = now.signed_duration_since(quota.observed_at) <= fresh
                && prev
                    .as_ref()
                    .is_some_and(|p| quota.observed_at > p.last_observed_at);

            if saw_reset_passed
                && !passed_now
                && matches!(qstate, QuotaState::Ok | QuotaState::Low)
                && fresh_reading
            {
                saw_reset_passed = false;
                if settings.is_enabled(NotificationEventKind::LimitReset) {
                    let dedup_id = format!("quota-reset:{key}:{}", quota.observed_at.to_rfc3339());
                    if let Some(decision) = self.try_accept_decision(
                        &key,
                        "",
                        dedup_id,
                        RedactedNotificationPayload {
                            event: NotificationEventKind::LimitReset,
                            agent: agent_display_name(&quota.provider),
                            session_title: quota_window_label(quota.window).to_owned(),
                        },
                        now,
                    ) {
                        out.push(decision);
                    }
                }
            }

            let mut low_cycle = prev.and_then(|p| p.last_low_cycle);
            if qstate == QuotaState::Low && now.signed_duration_since(quota.observed_at) <= fresh {
                let cycle = quota
                    .resets_at
                    .map_or_else(|| now.date_naive().to_string(), |t| t.to_rfc3339());
                if low_cycle.as_deref() != Some(cycle.as_str()) {
                    low_cycle = Some(cycle.clone());
                    if settings.is_enabled(NotificationEventKind::LimitLow) {
                        let dedup_id = format!("quota-low:{key}:{cycle}");
                        if let Some(decision) = self.try_accept_decision(
                            &key,
                            "",
                            dedup_id,
                            RedactedNotificationPayload {
                                event: NotificationEventKind::LimitLow,
                                agent: agent_display_name(&quota.provider),
                                session_title: quota_window_label(quota.window).to_owned(),
                            },
                            now,
                        ) {
                            out.push(decision);
                        }
                    }
                }
            } else if qstate != QuotaState::Low {
                low_cycle = None;
            }

            next_quotas.insert(
                key,
                QuotaWatchState {
                    saw_reset_passed,
                    last_resets_at: quota.resets_at,
                    last_observed_at: quota.observed_at,
                    last_low_cycle: low_cycle,
                },
            );
        }

        self.quotas = next_quotas;
    }

    fn try_accept_decision(
        &mut self,
        subject_key: &str,
        conversation_id: &str,
        dedup_id: String,
        payload: RedactedNotificationPayload,
        now: DateTime<Utc>,
    ) -> Option<NotificationDecision> {
        if self.seen_dedup_ids.contains(&dedup_id) {
            return None;
        }
        let rate_key = (subject_key.to_owned(), payload.event);
        if self.last_sent.get(&rate_key).is_some_and(|prev| {
            now.signed_duration_since(*prev) < Duration::seconds(RATE_LIMIT_WINDOW_SECS)
        }) {
            return None;
        }
        if self.seen_dedup_ids.len() >= MAX_DEDUP_IDS {
            self.seen_dedup_ids.clear();
        }
        self.seen_dedup_ids.insert(dedup_id.clone());
        self.last_sent.insert(rate_key, now);
        Some(NotificationDecision {
            dedup_id,
            conversation_id: conversation_id.to_owned(),
            payload,
        })
    }
}

fn build_batch(
    decisions: Vec<NotificationDecision>,
    now: DateTime<Utc>,
) -> Option<NotificationBatch> {
    if decisions.is_empty() {
        return None;
    }
    let payloads = decisions
        .iter()
        .map(|d| d.payload.clone())
        .collect::<Vec<_>>();
    let desktop_message = format_desktop_notification(&payloads);
    if decisions.len() == 1 {
        let first = &decisions[0];
        return Some(NotificationBatch {
            push_event_id: sanitize_push_id(&first.dedup_id),
            push_event: first.payload.event,
            push_agent: first.payload.agent.clone(),
            push_session_title: first.payload.session_title.clone(),
            decisions,
            desktop_message,
        });
    }
    let first_event = decisions[0].payload.event;
    let same_agent = decisions
        .iter()
        .all(|d| d.payload.agent == decisions[0].payload.agent);
    let push_agent = if same_agent {
        decisions[0].payload.agent.clone()
    } else {
        "Agents".to_owned()
    };
    let push_event_id = sanitize_push_id(&format!(
        "burst:{}:{}:{}",
        now.timestamp(),
        decisions.len(),
        decisions[0].dedup_id
    ));
    let push_session_title = format!("{} updates", decisions.len());
    Some(NotificationBatch {
        decisions,
        desktop_message,
        push_event_id,
        push_event: first_event,
        push_agent,
        push_session_title,
    })
}

fn sanitize_push_id(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, ':' | '.' | '_' | '-') {
                ch
            } else {
                '_'
            }
        })
        .take(256)
        .collect();
    if cleaned.is_empty() {
        "notice".to_owned()
    } else {
        cleaned
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{QuotaWindow, Surface};

    fn base_time() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-11-13T10:00:00Z")
            .expect("valid time")
            .with_timezone(&Utc)
    }

    fn make_session(
        id: &str,
        agent: AgentId,
        title: Option<&str>,
        state: SessionState,
        last_event_at: DateTime<Utc>,
    ) -> Session {
        Session {
            id: id.to_owned(),
            agent,
            surface: Surface::Cli,
            project_id: "proj-1".to_owned(),
            parent_session_id: None,
            model: Some("claude-sonnet-4".to_owned()),
            title: title.map(str::to_owned),
            state,
            pid: Some(4242),
            started_at: last_event_at - Duration::minutes(10),
            last_event_at,
        }
    }

    fn make_quota(
        provider: &str,
        used_pct: f64,
        observed_at: DateTime<Utc>,
        resets_at: Option<DateTime<Utc>>,
    ) -> QuotaSnapshot {
        QuotaSnapshot {
            provider: provider.to_owned(),
            limit_id: "primary".to_owned(),
            window: QuotaWindow::FiveHour,
            used_pct,
            resets_at,
            plan: None,
            basis: TrustLevel::Api,
            observed_at,
        }
    }

    fn at(state: SessionState, t: DateTime<Utc>) -> Session {
        make_session(
            "claude-code:stop",
            AgentId::ClaudeCode,
            Some("Secret task"),
            state,
            t,
        )
    }

    #[test]
    fn no_notification_while_working_or_on_tool_steps() {
        let mut engine = NotificationEngine::new();
        let settings = NotificationSettings::default();
        let t0 = base_time();
        for (i, state) in [
            SessionState::Thinking,
            SessionState::Tool,
            SessionState::Thinking,
            SessionState::Tool,
        ]
        .into_iter()
        .enumerate()
        {
            let t = t0 + Duration::minutes(i as i64 * 3);
            assert!(
                engine
                    .evaluate(&[at(state, t)], &[], &settings, t)
                    .is_none()
            );
        }
    }

    #[test]
    fn process_exit_while_working_notifies_once() {
        let mut engine = NotificationEngine::new();
        let settings = NotificationSettings::default();
        let t0 = base_time();
        assert!(
            engine
                .evaluate(&[at(SessionState::Tool, t0)], &[], &settings, t0)
                .is_none()
        );
        let t1 = t0 + Duration::seconds(30);
        let batch = engine
            .evaluate(&[at(SessionState::Ended, t1)], &[], &settings, t1)
            .expect("exit while working notifies");
        assert_eq!(batch.decisions.len(), 1);
        assert_eq!(batch.push_event, NotificationEventKind::SessionStopped);
        assert!(!batch.desktop_message.is_empty());
        let t2 = t1 + Duration::seconds(10);
        assert!(
            engine
                .evaluate(&[at(SessionState::Ended, t1)], &[], &settings, t2)
                .is_none()
        );
    }

    #[test]
    fn turn_end_then_exit_together_notifies_once() {
        let mut engine = NotificationEngine::new();
        let settings = NotificationSettings::default();
        let t0 = base_time();
        engine.evaluate(&[at(SessionState::Thinking, t0)], &[], &settings, t0);
        let done = t0 + Duration::minutes(5);
        let first = engine
            .evaluate(
                &[at(SessionState::Idle, done)],
                &[],
                &settings,
                done + Duration::seconds(10),
            )
            .expect("turn finished");
        assert_eq!(first.push_event, NotificationEventKind::TurnFinished);
        let exit = done + Duration::seconds(11);
        assert!(
            engine
                .evaluate(&[at(SessionState::Ended, exit)], &[], &settings, exit)
                .is_none(),
            "exit right after a reported turn end must not notify again"
        );
    }

    #[test]
    fn exit_during_turn_settle_reports_only_the_stop() {
        let mut engine = NotificationEngine::new();
        let settings = NotificationSettings::default();
        let t0 = base_time();
        engine.evaluate(&[at(SessionState::Thinking, t0)], &[], &settings, t0);
        let done = t0 + Duration::minutes(5);
        assert!(
            engine
                .evaluate(&[at(SessionState::Idle, done)], &[], &settings, done)
                .is_none()
        );
        let exit = done + Duration::seconds(2);
        let batch = engine
            .evaluate(&[at(SessionState::Ended, exit)], &[], &settings, exit)
            .expect("one stop");
        assert_eq!(batch.decisions.len(), 1);
        assert_eq!(batch.push_event, NotificationEventKind::SessionStopped);
        let later = exit + Duration::seconds(30);
        assert!(
            engine
                .evaluate(&[at(SessionState::Ended, exit)], &[], &settings, later)
                .is_none()
        );
    }

    #[test]
    fn error_needs_attention_and_respects_preference() {
        let t0 = base_time();
        let mut engine = NotificationEngine::new();
        let settings = NotificationSettings::default();
        engine.evaluate(&[at(SessionState::Tool, t0)], &[], &settings, t0);
        let t1 = t0 + Duration::seconds(5);
        let batch = engine
            .evaluate(&[at(SessionState::Error, t1)], &[], &settings, t1)
            .expect("error notifies");
        assert_eq!(batch.push_event, NotificationEventKind::NeedsAttention);

        let mut quiet = NotificationEngine::new();
        let off = NotificationSettings {
            approval_waiting: false,
            ..NotificationSettings::default()
        };
        quiet.evaluate(&[at(SessionState::Tool, t0)], &[], &off, t0);
        assert!(
            quiet
                .evaluate(&[at(SessionState::Error, t1)], &[], &off, t1)
                .is_none()
        );
    }

    #[test]
    fn idle_session_that_exits_stays_silent_and_stale_exit_is_ignored() {
        let mut engine = NotificationEngine::new();
        let settings = NotificationSettings::default();
        let t0 = base_time();
        engine.evaluate(&[at(SessionState::Idle, t0)], &[], &settings, t0);
        let t1 = t0 + Duration::minutes(1);
        assert!(
            engine
                .evaluate(&[at(SessionState::Ended, t1)], &[], &settings, t1)
                .is_none()
        );

        let mut late = NotificationEngine::new();
        late.evaluate(&[at(SessionState::Tool, t0)], &[], &settings, t0);
        let t2 = t0 + Duration::hours(1);
        assert!(
            late.evaluate(&[at(SessionState::Ended, t0)], &[], &settings, t2)
                .is_none()
        );
    }

    #[test]
    fn turn_finished_requires_duration_strictly_above_threshold_and_settle() {
        let mut engine = NotificationEngine::new();
        let settings = NotificationSettings::default(); // default 2 minutes
        let t0 = base_time();

        let thinking = make_session(
            "claude-code:s1",
            AgentId::ClaudeCode,
            Some("Fix auth bug"),
            SessionState::Thinking,
            t0,
        );
        assert!(engine.evaluate(&[thinking], &[], &settings, t0).is_none());

        // Turn lasting exactly 120 seconds (not > 2 minutes) must not notify.
        let idle_short = make_session(
            "claude-code:s1",
            AgentId::ClaudeCode,
            Some("Fix auth bug"),
            SessionState::Idle,
            t0 + Duration::seconds(120),
        );
        assert!(
            engine
                .evaluate(&[idle_short], &[], &settings, t0 + Duration::seconds(130))
                .is_none()
        );

        // Turn 2: > 2 minutes (150s), wait 8s settle before firing.
        let t1 = t0 + Duration::minutes(10);
        let thinking2 = make_session(
            "claude-code:s1",
            AgentId::ClaudeCode,
            Some("Fix auth bug"),
            SessionState::Thinking,
            t1,
        );
        assert!(engine.evaluate(&[thinking2], &[], &settings, t1).is_none());

        let idle_long = make_session(
            "claude-code:s1",
            AgentId::ClaudeCode,
            Some("Fix auth bug"),
            SessionState::Idle,
            t1 + Duration::seconds(150),
        );
        // Before 8s settle -> suppressed.
        assert!(
            engine
                .evaluate(
                    std::slice::from_ref(&idle_long),
                    &[],
                    &settings,
                    t1 + Duration::seconds(152),
                )
                .is_none()
        );
        // After 8s settle -> fires!
        let batch = engine
            .evaluate(&[idle_long], &[], &settings, t1 + Duration::seconds(160))
            .expect("long turn must notify");
        assert_eq!(batch.decisions.len(), 1);
        assert_eq!(
            batch.decisions[0].payload.event,
            NotificationEventKind::TurnFinished
        );
        assert_eq!(batch.decisions[0].payload.session_title, "Fix auth bug");
        assert_eq!(batch.decisions[0].payload.agent, "Claude Code");
    }

    #[test]
    fn approval_waiting_triggers_and_hook_dedupes_subsequent_poll() {
        let mut engine = NotificationEngine::new();
        let settings = NotificationSettings::default();
        let t0 = base_time();
        let waiting = make_session(
            "codex:s2",
            AgentId::Codex,
            Some("Run migration"),
            SessionState::AwaitingApproval,
            t0,
        );

        // Phone-approval / PermissionRequest hook fires first.
        let hook_batch = engine
            .record_approval_waiting_hint(&waiting, &settings, t0 + Duration::seconds(1))
            .expect("hook should notify immediately");
        assert_eq!(
            hook_batch.decisions[0].payload.event,
            NotificationEventKind::ApprovalWaiting
        );

        // Subsequent daemon poll seeing the same AwaitingApproval state must not duplicate.
        assert!(
            engine
                .evaluate(&[waiting], &[], &settings, t0 + Duration::seconds(10))
                .is_none()
        );
    }

    #[test]
    fn limit_low_triggers_only_on_quota_state_low() {
        let mut engine = NotificationEngine::new();
        let settings = NotificationSettings::default();
        let t0 = base_time();
        let reset = t0 + Duration::hours(2);

        let ok = make_quota("claude-code", 50.0, t0, Some(reset));
        assert!(engine.evaluate(&[], &[ok], &settings, t0).is_none());

        // 85% used => 15% remaining => QuotaState::Low (< 20%)
        let low = make_quota("claude-code", 85.0, t0 + Duration::seconds(30), Some(reset));
        let batch = engine
            .evaluate(&[], &[low], &settings, t0 + Duration::seconds(35))
            .expect("low quota notifies");
        assert_eq!(
            batch.decisions[0].payload.event,
            NotificationEventKind::LimitLow
        );
        assert_eq!(batch.decisions[0].payload.agent, "Claude Code");
    }

    #[test]
    fn limit_reset_requires_reset_passed_then_fresh_reading() {
        let mut engine = NotificationEngine::new();
        let settings = NotificationSettings {
            limit_low: false,
            ..NotificationSettings::default()
        };
        let t0 = base_time();
        let reset_at = t0 + Duration::minutes(5);

        // Initial reading drop without reset_passed must NOT fire limit_reset.
        let high = make_quota("codex", 90.0, t0, Some(reset_at));
        assert!(engine.evaluate(&[], &[high], &settings, t0).is_none());
        let dropped_without_reset =
            make_quota("codex", 10.0, t0 + Duration::minutes(1), Some(reset_at));
        assert!(
            engine
                .evaluate(
                    &[],
                    &[dropped_without_reset],
                    &settings,
                    t0 + Duration::minutes(1),
                )
                .is_none()
        );

        // Now reset_at passes -> QuotaState::ResetUnconfirmed observed.
        let awaiting = make_quota("codex", 95.0, t0 + Duration::minutes(2), Some(reset_at));
        assert!(
            engine
                .evaluate(
                    &[],
                    &[awaiting],
                    &settings,
                    reset_at + Duration::seconds(10)
                )
                .is_none()
        );

        // Fresh reading arrives after reset_at with new window reset timestamp.
        let fresh = make_quota(
            "codex",
            5.0,
            reset_at + Duration::seconds(20),
            Some(reset_at + Duration::hours(5)),
        );
        let batch = engine
            .evaluate(&[], &[fresh], &settings, reset_at + Duration::seconds(25))
            .expect("reset_passed followed by fresh reading notifies");
        assert_eq!(
            batch.decisions[0].payload.event,
            NotificationEventKind::LimitReset
        );
    }

    #[test]
    fn rate_limits_same_session_and_event_to_five_minutes() {
        let mut engine = NotificationEngine::new();
        let settings = NotificationSettings::default();
        let t0 = base_time();

        let waiting1 = make_session(
            "claude-code:r1",
            AgentId::ClaudeCode,
            Some("Deploy app"),
            SessionState::AwaitingApproval,
            t0,
        );
        assert!(engine.evaluate(&[waiting1], &[], &settings, t0).is_some());

        // Transition back to Thinking, then back to AwaitingApproval within 5 min -> suppressed.
        let busy = make_session(
            "claude-code:r1",
            AgentId::ClaudeCode,
            Some("Deploy app"),
            SessionState::Thinking,
            t0 + Duration::seconds(60),
        );
        assert!(
            engine
                .evaluate(&[busy], &[], &settings, t0 + Duration::seconds(60))
                .is_none()
        );

        let waiting2 = make_session(
            "claude-code:r1",
            AgentId::ClaudeCode,
            Some("Deploy app"),
            SessionState::AwaitingApproval,
            t0 + Duration::seconds(120),
        );
        assert!(
            engine
                .evaluate(&[waiting2], &[], &settings, t0 + Duration::seconds(120))
                .is_none(),
            "second notification within 5 minutes must be rate-limited"
        );

        // Clear back to Thinking, then enter AwaitingApproval after 5 minutes -> allowed.
        let busy2 = make_session(
            "claude-code:r1",
            AgentId::ClaudeCode,
            Some("Deploy app"),
            SessionState::Thinking,
            t0 + Duration::seconds(290),
        );
        assert!(
            engine
                .evaluate(&[busy2], &[], &settings, t0 + Duration::seconds(290))
                .is_none()
        );

        let waiting3 = make_session(
            "claude-code:r1",
            AgentId::ClaudeCode,
            Some("Deploy app"),
            SessionState::AwaitingApproval,
            t0 + Duration::seconds(301),
        );
        assert!(
            engine
                .evaluate(&[waiting3], &[], &settings, t0 + Duration::seconds(301))
                .is_some(),
            "notification after 5 minutes must pass rate limit"
        );
    }

    #[test]
    fn groups_bursts_and_redacts_secrets_without_chat_content() {
        let mut engine = NotificationEngine::new();
        let settings = NotificationSettings::default();
        let t0 = base_time();

        let s1 = make_session(
            "claude-code:b1",
            AgentId::ClaudeCode,
            Some("Use API_KEY=super-secret-value\nwith assistant output"),
            SessionState::AwaitingApproval,
            t0,
        );
        let s2 = make_session(
            "codex:b2",
            AgentId::Codex,
            Some("Bearer tok_live_1234567890abcdef"),
            SessionState::AwaitingApproval,
            t0,
        );

        let batch = engine
            .evaluate(&[s1, s2], &[], &settings, t0 + Duration::seconds(2))
            .expect("burst notifies");
        assert_eq!(batch.decisions.len(), 2);
        assert!(batch.desktop_message.len() <= MAX_DESKTOP_CHARS);
        assert!(!batch.desktop_message.contains("super-secret-value"));
        assert!(!batch.desktop_message.contains("tok_live_1234567890abcdef"));
        assert!(!batch.desktop_message.contains('\n'));
        assert!(batch.desktop_message.contains("2 "));
    }

    #[test]
    fn disabled_events_are_suppressed() {
        let mut engine = NotificationEngine::new();
        let settings = NotificationSettings {
            approval_waiting: false,
            turn_finished: false,
            turn_finished_min_minutes: 2,
            limit_low: false,
            limit_reset: false,
        };
        let t0 = base_time();
        let waiting = make_session(
            "codex:off",
            AgentId::Codex,
            Some("Secret title"),
            SessionState::AwaitingApproval,
            t0,
        );
        let low = make_quota("claude-code", 90.0, t0, Some(t0 + Duration::hours(1)));
        assert!(
            engine
                .evaluate(&[waiting], &[low], &settings, t0 + Duration::seconds(2))
                .is_none()
        );
    }
}
