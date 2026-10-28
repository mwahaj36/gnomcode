//! Source-independent session presence rules for every agent.
use serde::{Deserialize, Serialize};

/// Grace duration for recent observations and brief idle gaps.
pub const RECENT_ACTIVITY_GRACE_MS: i64 = 120_000;

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
    let recent = age_millis.is_some_and(|age| age <= RECENT_ACTIVITY_GRACE_MS);
    match state {
        "ended" => SessionPresence::Ended,
        "awaiting_approval" => SessionPresence::AwaitingApproval,
        "thinking" | "tool" if recent || open_execution => SessionPresence::Running,
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
            classify_presence("idle", Some(120_000), false),
            SessionPresence::RecentlyIdle
        );
        assert_eq!(
            classify_presence("idle", Some(120_001), true),
            SessionPresence::Inactive
        );
        assert_eq!(
            classify_presence("thinking", Some(120_001), false),
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
    }
}
