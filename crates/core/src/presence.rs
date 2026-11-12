//! Source-independent session presence rules for every agent.
use serde::{Deserialize, Serialize};

/// Grace duration for recent observations and brief idle gaps (15 minutes).
pub const RECENT_ACTIVITY_GRACE_MS: i64 = 900_000;
/// Maximum age of an open tool/model execution without new log lines (2 hours).
pub const OPEN_EXECUTION_LIMIT_MS: i64 = 7_200_000;
/// Maximum age of a transcript-derived approval request without fresh observations.
/// Matches the open-execution limit (2 hours): without a live host or hook confirming
/// the dialog, an older request degrades to inactive rather than staying "Needs you".
pub const APPROVAL_WAIT_LIMIT_MS: i64 = OPEN_EXECUTION_LIMIT_MS;

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

/// Where a session appears on the Home board.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HomeGroup {
    /// Waiting for the user, such as an approval request.
    NeedsYou,
    /// Executing or recently idle.
    Running,
    /// Not currently active.
    Recent,
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
    /// Home board group for this presence.
    #[must_use]
    pub const fn home_group(self) -> HomeGroup {
        match self {
            Self::AwaitingApproval => HomeGroup::NeedsYou,
            Self::Running | Self::RecentlyIdle => HomeGroup::Running,
            Self::Inactive | Self::Ended => HomeGroup::Recent,
        }
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
    let unresolved = age_millis.is_some_and(|age| (0..=APPROVAL_WAIT_LIMIT_MS).contains(&age));
    let executing = age_millis.is_some_and(|age| (0..=OPEN_EXECUTION_LIMIT_MS).contains(&age));
    match state {
        "ended" => SessionPresence::Ended,
        "awaiting_approval" if unresolved => SessionPresence::AwaitingApproval,
        "thinking" | "tool" if recent || (open_execution && executing) => SessionPresence::Running,
        "idle" if recent => SessionPresence::RecentlyIdle,
        _ => SessionPresence::Inactive,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn home_group_maps_every_presence() {
        assert_eq!(
            SessionPresence::AwaitingApproval.home_group(),
            HomeGroup::NeedsYou
        );
        assert_eq!(SessionPresence::Running.home_group(), HomeGroup::Running);
        assert_eq!(
            SessionPresence::RecentlyIdle.home_group(),
            HomeGroup::Running
        );
        assert_eq!(SessionPresence::Inactive.home_group(), HomeGroup::Recent);
        assert_eq!(SessionPresence::Ended.home_group(), HomeGroup::Recent);
    }

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
            classify_presence("awaiting_approval", Some(APPROVAL_WAIT_LIMIT_MS), false),
            SessionPresence::AwaitingApproval
        );
        assert_eq!(
            classify_presence("awaiting_approval", Some(APPROVAL_WAIT_LIMIT_MS + 1), false),
            SessionPresence::Inactive
        );
        assert_eq!(
            classify_presence("awaiting_approval", Some(3_600_000), false),
            SessionPresence::AwaitingApproval
        );
        assert_eq!(
            classify_presence("awaiting_approval", Some(86_400_000), false),
            SessionPresence::Inactive
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
            classify_presence("tool", Some(13 * 3_600_000), true),
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
