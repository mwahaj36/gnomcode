//! Source-independent session presence rules for every agent.
use serde::{Deserialize, Serialize};

/// Grace duration for recent observations and brief idle gaps (15 minutes).
pub const RECENT_ACTIVITY_GRACE_MS: i64 = 900_000;
/// Maximum age of an unresolved source observation without live process evidence.
pub const STALE_EXECUTION_LIMIT_MS: i64 = 86_400_000;

/// What the last source observation establishes about a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionPresence {
    /// An open execution or recent running observation.
    Running,
    /// Explicit unresolved approval request.
    AwaitingApproval,
    /// Idle observation within the two-minute grace period.
    RecentlyIdle,
    /// No current execution evidence.
    #[default]
    Inactive,
    /// Explicit source-reported end.
    Ended,
}
impl SessionPresence {
    /// Whether this session belongs in ongoing lists and badges.
    #[must_use]
    pub const fn is_ongoing(self) -> bool {
        matches!(
            self,
            Self::Running | Self::AwaitingApproval | Self::RecentlyIdle
        )
    }
    /// Whether the source indicates execution or an approval wait.
    #[must_use]
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Running | Self::AwaitingApproval)
    }
}
/// Classifies source observations without assuming silence means completion.
/// Open execution means an unmatched start event consistent with the stored state.
#[must_use]
pub fn classify_presence(
    state: &str,
    age_millis: Option<i64>,
    open_execution: bool,
) -> SessionPresence {
    let recent = age_millis.is_some_and(|age| (0..=RECENT_ACTIVITY_GRACE_MS).contains(&age));
    let unresolved = age_millis.is_some_and(|age| (0..=STALE_EXECUTION_LIMIT_MS).contains(&age));
    match state {
        "ended" => SessionPresence::Ended,
        "awaiting_approval" if unresolved => SessionPresence::AwaitingApproval,
        "thinking" | "tool" if recent || (open_execution && unresolved) => SessionPresence::Running,
        "idle" if recent => SessionPresence::RecentlyIdle,
        _ => SessionPresence::Inactive,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rules_apply_without_provider_specific_exceptions() {
        assert_eq!(
            classify_presence("idle", Some(900_000), false),
            SessionPresence::RecentlyIdle
        );
        assert_eq!(
            classify_presence("idle", Some(900_001), true),
            SessionPresence::Inactive
        );
        assert_eq!(
            classify_presence("thinking", Some(900_001), false),
            SessionPresence::Inactive
        );
        assert_eq!(
            classify_presence("tool", Some(3_600_000), true),
            SessionPresence::Running
        );
        assert_eq!(
            classify_presence("awaiting_approval", Some(86_400_000), false),
            SessionPresence::AwaitingApproval
        );
        assert_eq!(
            classify_presence("ended", Some(0), true),
            SessionPresence::Ended
        );
        assert_eq!(
            classify_presence("unknown", None, false),
            SessionPresence::Inactive
        );
        assert_eq!(
            classify_presence("tool", Some(15 * 86_400_000), true),
            SessionPresence::Inactive
        );
        assert_eq!(
            classify_presence("awaiting_approval", Some(15 * 86_400_000), false),
            SessionPresence::Inactive
        );
    }
}
