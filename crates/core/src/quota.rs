//! Quota state and pace derived from a snapshot (SPEC 3.9).
//!
//! Pure: `now` is always a parameter, so results are deterministic and testable.

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;

use crate::{QuotaSnapshot, QuotaWindow, TrustLevel};

/// Remaining percentage below which a window is reported as low.
pub const LOW_REMAINING_PCT: f64 = 20.0;
/// Readings older than this are shown as last seen (SPEC 3.9).
pub const STALE_AFTER: Duration = Duration::minutes(2);
/// Pace is not reported before this fraction of a window has elapsed.
const MIN_PACE_ELAPSED: f64 = 0.05;
/// Usage within this many percentage points of even pace counts as on pace.
const ON_PACE_BAND_PCT: f64 = 5.0;

/// Display state of one quota window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuotaState {
    /// Plenty of capacity left.
    Ok,
    /// Less than [`LOW_REMAINING_PCT`] left.
    Low,
    /// Nothing left until the window resets.
    Exhausted,
    /// The recorded reset time has passed and no newer reading exists yet.
    ResetUnconfirmed,
    /// The reading cannot describe capacity.
    Unknown,
}

impl QuotaState {
    /// Ordering used to pick the worst window: `Exhausted > Low > Unknown > ResetUnconfirmed > Ok`.
    #[must_use]
    pub const fn severity(self) -> u8 {
        match self {
            Self::Exhausted => 4,
            Self::Low => 3,
            Self::Unknown => 2,
            Self::ResetUnconfirmed => 1,
            Self::Ok => 0,
        }
    }
}

/// Whether usage is ahead of, behind, or at even pace for the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PaceVerdict {
    /// Within the on-pace band of even use.
    OnPace,
    /// Using faster than even pace.
    Ahead,
    /// Using slower than even pace (capacity in reserve).
    Behind,
}

/// Pace of usage across a window.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct QuotaPace {
    /// Remaining percentage expected at `now` if usage were even across the window.
    pub pace_remaining_pct: f64,
    /// Actual remaining minus expected remaining; negative means using faster than pace.
    pub delta_pct: f64,
    /// Verdict derived from `delta_pct`.
    pub verdict: PaceVerdict,
    /// When the window would run out at the current rate, if before the reset and the observation is fresh.
    pub runs_out_at: Option<DateTime<Utc>>,
    /// Whether the underlying observation is older than [`STALE_AFTER`].
    pub stale: bool,
    /// Why `runs_out_at` was withheld when `stale` is true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stale_reason: Option<String>,
}

/// Length of a quota window.
#[must_use]
pub fn window_length(window: QuotaWindow) -> Duration {
    match window {
        QuotaWindow::FiveHour => Duration::hours(5),
        QuotaWindow::Weekly => Duration::days(7),
        QuotaWindow::Monthly => Duration::days(30),
    }
}

/// Returns true when the recorded reset timestamp is at or before `now`.
#[must_use]
pub fn reset_passed(s: &QuotaSnapshot, now: DateTime<Utc>) -> bool {
    s.resets_at.is_some_and(|reset| reset <= now)
}

/// Effective trust basis for a snapshot; assumed resets are always [`TrustLevel::Estimated`].
#[must_use]
pub fn effective_basis(s: &QuotaSnapshot, now: DateTime<Utc>) -> TrustLevel {
    if reset_passed(s, now) && s.basis != TrustLevel::Unknown && s.used_pct.is_finite() {
        TrustLevel::Estimated
    } else {
        s.basis
    }
}

/// Effective used percentage for a snapshot; assumed resets report `0.0` until confirmed.
#[must_use]
pub fn effective_used_pct(s: &QuotaSnapshot, now: DateTime<Utc>) -> f64 {
    if reset_passed(s, now) && s.basis != TrustLevel::Unknown && s.used_pct.is_finite() {
        0.0
    } else {
        s.used_pct
    }
}

/// Remaining percentage, or `None` when the reading cannot describe capacity.
#[must_use]
pub fn remaining_pct(s: &QuotaSnapshot, now: DateTime<Utc>) -> Option<f64> {
    if s.basis == TrustLevel::Unknown || !s.used_pct.is_finite() {
        return None;
    }
    if reset_passed(s, now) {
        return Some(100.0);
    }
    Some((100.0 - s.used_pct).clamp(0.0, 100.0))
}

/// Display state for a snapshot.
#[must_use]
pub fn quota_state(s: &QuotaSnapshot, now: DateTime<Utc>) -> QuotaState {
    if reset_passed(s, now) && s.basis != TrustLevel::Unknown && s.used_pct.is_finite() {
        return QuotaState::ResetUnconfirmed;
    }
    match remaining_pct(s, now) {
        None => QuotaState::Unknown,
        Some(left) if left <= 0.0 => QuotaState::Exhausted,
        Some(left) if left < LOW_REMAINING_PCT => QuotaState::Low,
        Some(_) => QuotaState::Ok,
    }
}

/// Pace for a snapshot, when it can be computed honestly.
#[must_use]
pub fn quota_pace(s: &QuotaSnapshot, now: DateTime<Utc>) -> Option<QuotaPace> {
    if !matches!(quota_state(s, now), QuotaState::Ok | QuotaState::Low)
        || !matches!(s.basis, TrustLevel::Exact | TrustLevel::Api)
    {
        return None;
    }
    let resets_at = s.resets_at?;
    let remaining = remaining_pct(s, now)?;
    let len = window_length(s.window);
    let start = resets_at - len;
    let elapsed = ((now - start).num_seconds() as f64 / len.num_seconds() as f64).clamp(0.0, 1.0);
    if elapsed < MIN_PACE_ELAPSED {
        return None;
    }
    let pace_remaining_pct = 100.0 * (1.0 - elapsed);
    let delta_pct = remaining - pace_remaining_pct;
    let verdict = if delta_pct < -ON_PACE_BAND_PCT {
        PaceVerdict::Ahead
    } else if delta_pct > ON_PACE_BAND_PCT {
        PaceVerdict::Behind
    } else {
        PaceVerdict::OnPace
    };
    let stale = is_stale(s, now);
    let stale_reason = stale.then(|| "Observation is older than 2 minutes.".to_owned());
    let runs_out_at = (!stale && verdict == PaceVerdict::Ahead && s.used_pct > 0.0)
        .then(|| {
            let rate = s.used_pct / (now - start).num_seconds() as f64;
            now + Duration::seconds((remaining / rate) as i64)
        })
        .filter(|eta| *eta < resets_at);
    Some(QuotaPace {
        pace_remaining_pct,
        delta_pct,
        verdict,
        runs_out_at,
        stale,
        stale_reason,
    })
}

/// Whether the reading is older than [`STALE_AFTER`].
#[must_use]
pub fn is_stale(s: &QuotaSnapshot, now: DateTime<Utc>) -> bool {
    now - s.observed_at > STALE_AFTER
}

/// Whether `model` (`Claude Opus 5.5 (High)`, `claude-opus-5-5`) belongs to the pool member `name`
/// (`Claude Opus`, `Gemini Pro`): every word of `name` appears in `model` in the same order, so
/// version numbers and effort suffixes between them are ignored. Pure and case-insensitive.
#[must_use]
pub fn model_matches_pool_name(model: &str, name: &str) -> bool {
    fn words(text: &str) -> Vec<String> {
        text.split(|c: char| !c.is_alphanumeric())
            .filter(|word| !word.is_empty())
            .map(str::to_lowercase)
            .collect()
    }
    let wanted = words(name);
    if wanted.is_empty() {
        return false;
    }
    let mut have = words(model).into_iter();
    wanted.iter().all(|want| have.any(|word| word == *want))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn pool_names_match_models_by_ordered_words() {
        assert!(model_matches_pool_name(
            "Claude Opus 5.5 (High)",
            "Claude Opus"
        ));
        assert!(model_matches_pool_name("claude-opus-5-5", "Claude Opus"));
        assert!(model_matches_pool_name(
            "Gemini 3.1 Pro (High)",
            "Gemini Pro"
        ));
        assert!(model_matches_pool_name("GPT-OSS 120B (Medium)", "GPT-OSS"));
        assert!(!model_matches_pool_name("Claude Sonnet 5", "Claude Opus"));
        assert!(!model_matches_pool_name("Pro Gemini", "Gemini Pro"));
        assert!(!model_matches_pool_name("anything", " "));
    }

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 12, 12, 0, 0)
            .single()
            .unwrap_or_default()
    }

    fn snap(
        window: QuotaWindow,
        used: f64,
        resets_in: Duration,
        basis: TrustLevel,
    ) -> QuotaSnapshot {
        QuotaSnapshot {
            provider: "codex".into(),
            limit_id: "codex".into(),
            window,
            used_pct: used,
            resets_at: Some(now() + resets_in),
            plan: None,
            basis,
            observed_at: now(),
        }
    }

    #[test]
    fn state_ok_when_plenty_left() {
        let s = snap(
            QuotaWindow::Weekly,
            30.0,
            Duration::days(3),
            TrustLevel::Exact,
        );
        assert_eq!(quota_state(&s, now()), QuotaState::Ok);
        assert_eq!(remaining_pct(&s, now()), Some(70.0));
    }

    #[test]
    fn state_low_below_twenty_left() {
        let s = snap(
            QuotaWindow::Weekly,
            85.0,
            Duration::days(3),
            TrustLevel::Exact,
        );
        assert_eq!(quota_state(&s, now()), QuotaState::Low);
    }

    #[test]
    fn state_exhausted_at_full_use() {
        let s = snap(
            QuotaWindow::Weekly,
            100.0,
            Duration::days(3),
            TrustLevel::Exact,
        );
        assert_eq!(quota_state(&s, now()), QuotaState::Exhausted);
    }

    #[test]
    fn state_reset_unconfirmed_after_reset_time() {
        let s = snap(
            QuotaWindow::FiveHour,
            50.0,
            -Duration::minutes(1),
            TrustLevel::Exact,
        );
        assert_eq!(quota_state(&s, now()), QuotaState::ResetUnconfirmed);
        assert_eq!(remaining_pct(&s, now()), Some(100.0));
        assert!((effective_used_pct(&s, now()) - 0.0).abs() < 1e-9);
        assert_eq!(effective_basis(&s, now()), TrustLevel::Estimated);
    }

    #[test]
    fn state_reset_unconfirmed_at_exact_reset_boundary() {
        let s = snap(
            QuotaWindow::FiveHour,
            99.0,
            Duration::zero(),
            TrustLevel::Api,
        );
        assert_eq!(quota_state(&s, now()), QuotaState::ResetUnconfirmed);
        assert_eq!(remaining_pct(&s, now()), Some(100.0));
        assert_eq!(effective_basis(&s, now()), TrustLevel::Estimated);
    }

    #[test]
    fn state_unknown_for_unknown_basis() {
        let s = snap(
            QuotaWindow::Weekly,
            10.0,
            Duration::days(3),
            TrustLevel::Unknown,
        );
        assert_eq!(quota_state(&s, now()), QuotaState::Unknown);
    }

    #[test]
    fn state_unknown_for_nan_used() {
        let s = snap(
            QuotaWindow::Weekly,
            f64::NAN,
            Duration::days(3),
            TrustLevel::Exact,
        );
        assert_eq!(quota_state(&s, now()), QuotaState::Unknown);
    }

    #[test]
    fn pace_none_for_estimated_basis() {
        let s = snap(
            QuotaWindow::Weekly,
            50.0,
            Duration::hours(84),
            TrustLevel::Estimated,
        );
        assert_eq!(quota_pace(&s, now()), None);
    }

    #[test]
    fn pace_none_in_first_five_percent_of_window() {
        // 10 minutes into a 5 hour window is 3.3% elapsed.
        let s = snap(
            QuotaWindow::FiveHour,
            5.0,
            Duration::minutes(290),
            TrustLevel::Exact,
        );
        assert_eq!(quota_pace(&s, now()), None);
    }

    #[test]
    fn pace_on_pace_when_usage_even() {
        let s = snap(
            QuotaWindow::Weekly,
            50.0,
            Duration::hours(84),
            TrustLevel::Exact,
        );
        let pace = quota_pace(&s, now()).expect("pace");
        assert_eq!(pace.verdict, PaceVerdict::OnPace);
        assert!((pace.pace_remaining_pct - 50.0).abs() < 1e-9);
        assert!(pace.delta_pct.abs() < 1e-9);
        assert_eq!(pace.runs_out_at, None);
    }

    #[test]
    fn pace_ahead_with_run_out_before_reset() {
        let s = snap(
            QuotaWindow::FiveHour,
            80.0,
            Duration::minutes(150),
            TrustLevel::Exact,
        );
        let pace = quota_pace(&s, now()).expect("pace");
        assert_eq!(pace.verdict, PaceVerdict::Ahead);
        let eta = pace.runs_out_at.expect("run out");
        assert!(eta > now());
        assert!(eta < s.resets_at.expect("reset"));
    }

    #[test]
    fn pace_behind_when_in_reserve() {
        let s = snap(
            QuotaWindow::Weekly,
            20.0,
            Duration::hours(84),
            TrustLevel::Exact,
        );
        let pace = quota_pace(&s, now()).expect("pace");
        assert_eq!(pace.verdict, PaceVerdict::Behind);
        assert!((pace.delta_pct - 30.0).abs() < 1e-9);
        assert_eq!(pace.runs_out_at, None);
    }

    #[test]
    fn pace_ahead_late_in_window_still_runs_out_before_reset() {
        // Under a linear projection, a window that is ahead of pace always runs out before its
        // reset, so the "ahead without run-out" branch is purely defensive. Assert the invariant
        // at the extreme the plan names (90% elapsed, 96% used).
        let s = snap(
            QuotaWindow::Weekly,
            96.0,
            Duration::hours(17),
            TrustLevel::Exact,
        );
        let pace = quota_pace(&s, now()).expect("pace");
        assert_eq!(pace.verdict, PaceVerdict::Ahead);
        assert!(
            pace.runs_out_at
                .is_some_and(|eta| eta < s.resets_at.unwrap_or_default())
        );
    }

    #[test]
    fn stale_after_two_minutes() {
        let s = snap(
            QuotaWindow::Weekly,
            10.0,
            Duration::days(3),
            TrustLevel::Exact,
        );
        assert!(!is_stale(&s, now() + Duration::minutes(2)));
        assert!(is_stale(
            &s,
            now() + Duration::minutes(2) + Duration::seconds(1)
        ));
    }

    #[test]
    fn pace_stale_omits_runs_out_projection_with_reason() {
        let mut s = snap(
            QuotaWindow::FiveHour,
            99.0,
            Duration::minutes(66),
            TrustLevel::Exact,
        );
        s.observed_at = now() - Duration::hours(2);
        let pace = quota_pace(&s, now()).expect("pace");
        assert!(pace.stale);
        assert!(pace.stale_reason.is_some());
        assert_eq!(pace.runs_out_at, None);
    }

    #[test]
    fn severity_orders_exhausted_first() {
        let order = [
            QuotaState::Exhausted,
            QuotaState::Low,
            QuotaState::Unknown,
            QuotaState::ResetUnconfirmed,
            QuotaState::Ok,
        ];
        let severities: Vec<u8> = order.iter().map(|s| s.severity()).collect();
        assert_eq!(severities, vec![4, 3, 2, 1, 0]);
    }
}
