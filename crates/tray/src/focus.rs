//! Native window focus and desktop notification triggers (`SPEC §3.10`, `§3.13`).
//!
//! Provides:
//! - Window focus bring-to-front dispatcher (Windows Win32 `SetForegroundWindow`, macOS AppleScript, Linux wmctrl/KWin fallback).
//! - Desktop notification triggers for awaiting approval, quota warnings, and quota resets.

use gnomcode_core::AgentId;
use serde::{Deserialize, Serialize};

/// Desktop notification event payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationPayload {
    /// Notification title text.
    pub title: String,
    /// Notification body message.
    pub body: String,
    /// Associated agent ID, if applicable.
    pub agent: Option<AgentId>,
    /// Associated session ID, if applicable.
    pub session_id: Option<String>,
}

/// Generates a notification payload when an agent enters `awaiting_approval` state.
#[must_use]
pub fn notify_approval_needed(
    agent: AgentId,
    session_id: &str,
    tool_name: Option<&str>,
) -> NotificationPayload {
    let action = tool_name.map_or_else(|| "action".to_owned(), |t| format!("tool `{t}`"));
    NotificationPayload {
        title: format!("{} needs your approval", agent.as_str()),
        body: format!("Session {session_id} is waiting for permission to run {action}."),
        agent: Some(agent),
        session_id: Some(session_id.to_owned()),
    }
}

/// Generates a notification payload when a rate-limited quota pool resets.
#[must_use]
pub fn notify_quota_reset(provider: &str, limit_id: &str) -> NotificationPayload {
    NotificationPayload {
        title: format!("Quota reset: {provider}"),
        body: format!("The `{limit_id}` quota pool has reset. Rate-limited sessions can resume."),
        agent: None,
        session_id: None,
    }
}

/// Generates a notification payload when an agent reaches or exceeds the context saturation threshold.
#[must_use]
pub fn notify_context_saturation(
    agent: AgentId,
    project: &str,
    fill_pct: u32,
) -> NotificationPayload {
    NotificationPayload {
        title: format!("{} context saturation", agent.as_str()),
        body: format!(
            "Session in `{project}` reached {fill_pct}% estimated context load. Safety reserve active."
        ),
        agent: Some(agent),
        session_id: None,
    }
}

pub use gnomcode_platform::{bring_to_front, send_desktop_notification};

#[cfg(test)]
mod tests {
    use super::{notify_approval_needed, notify_context_saturation, notify_quota_reset};
    use gnomcode_core::AgentId;

    #[test]
    fn context_saturation_notification_formats_cleanly_without_emojis() {
        let n = notify_context_saturation(AgentId::ClaudeCode, "my-repo", 92);
        assert_eq!(n.title, "claude-code context saturation");
        assert!(n.body.contains("92% estimated"));
    }

    #[test]
    fn approval_notification_formats_cleanly_without_emojis() {
        let n = notify_approval_needed(AgentId::ClaudeCode, "s1", Some("Bash"));
        assert_eq!(n.title, "claude-code needs your approval");
        assert!(n.body.contains("tool `Bash`"));
    }

    #[test]
    fn quota_reset_notification_formats_cleanly() {
        let n = notify_quota_reset("openai", "codex");
        assert_eq!(n.title, "Quota reset: openai");
        assert!(n.body.contains("codex"));
    }
}
