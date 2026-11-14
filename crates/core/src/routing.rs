//! Conservative quota-based failover selection.
use crate::{QuotaSnapshot, RoutingPolicy};
use chrono::{DateTime, Utc};
/// Selects the candidate with the most known headroom, respecting policy and freshness.
#[must_use]
pub fn select_failover(
    source: (&str, &str),
    policy: RoutingPolicy,
    candidates: &[(&str, &str)],
    quotas: &[QuotaSnapshot],
    now: DateTime<Utc>,
) -> Option<usize> {
    candidates
        .iter()
        .enumerate()
        .filter_map(|(index, (agent, model))| {
            if (*agent == source.0 && (*model == source.1 || *model == "@default"))
                || !policy.allows_automatic(Some(source.0), Some(agent))
            {
                return None;
            }
            let relevant: Vec<_> = quotas
                .iter()
                .filter(|q| q.provider == *agent && (q.limit_id == *model || q.limit_id == *agent))
                .collect();
            if relevant.is_empty()
                || relevant.iter().any(|q| {
                    q.basis == crate::TrustLevel::Unknown
                        || q.observed_at > now
                        || now - q.observed_at > chrono::Duration::minutes(5)
                        || !(0.0..100.0).contains(&q.used_pct)
                })
            {
                return None;
            }
            // A same-provider change needs explicit evidence of a different model pool.
            if *agent == source.0 && !relevant.iter().any(|q| q.limit_id == *model) {
                return None;
            }
            let worst = relevant
                .iter()
                .fold(0.0_f64, |used, q| used.max(q.used_pct));
            Some((index, worst))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(index, _)| index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{QuotaWindow, TrustLevel};
    fn quota(provider: &str, pool: &str, used: f64, now: DateTime<Utc>) -> QuotaSnapshot {
        QuotaSnapshot {
            provider: provider.into(),
            limit_id: pool.into(),
            window: QuotaWindow::FiveHour,
            used_pct: used,
            resets_at: None,
            plan: None,
            basis: TrustLevel::Exact,
            observed_at: now,
        }
    }
    #[test]
    fn failover_requires_fresh_headroom_and_respects_provider_policy() {
        let now = Utc::now();
        let candidates = [("codex", "gpt-other"), ("claude-code", "claude-other")];
        let quotas = [
            quota("codex", "gpt-other", 30.0, now),
            quota("claude-code", "claude-other", 10.0, now),
        ];
        assert_eq!(
            select_failover(
                ("codex", "gpt-source"),
                RoutingPolicy::Automatic,
                &candidates,
                &quotas,
                now
            ),
            Some(1)
        );
        assert_eq!(
            select_failover(
                ("codex", "gpt-source"),
                RoutingPolicy::DenyCrossProvider,
                &candidates,
                &quotas,
                now
            ),
            Some(0)
        );
        assert_eq!(
            select_failover(
                ("codex", "gpt-source"),
                RoutingPolicy::AskFirst,
                &candidates,
                &quotas,
                now
            ),
            None
        );
        assert_eq!(
            select_failover(
                ("codex", "gpt-source"),
                RoutingPolicy::Automatic,
                &candidates,
                &quotas,
                now + chrono::Duration::minutes(6)
            ),
            None
        );
    }
    /// Simulates a Claude Code session whose 5h window hit its limit and asserts the
    /// handoff decision for each routing policy.
    #[test]
    fn limit_reached_handoff_decision_per_policy() {
        let now = Utc::now();
        let source = ("claude-code", "claude-opus");
        let candidates = [
            ("claude-code", "claude-code"),
            ("claude-code", "claude-haiku"),
            ("codex", "gpt-other"),
            ("antigravity", "gemini-pool"),
        ];
        let quotas = [
            quota("claude-code", "claude-opus", 100.0, now),
            quota("claude-code", "claude-haiku", 40.0, now),
            quota("codex", "gpt-other", 60.0, now),
            quota("antigravity", "gemini-pool", 20.0, now),
        ];
        let decide = |policy| select_failover(source, policy, &candidates, &quotas, now);
        assert_eq!(decide(RoutingPolicy::AskFirst), None);
        assert_eq!(decide(RoutingPolicy::Automatic), Some(3));
        assert_eq!(decide(RoutingPolicy::DenyCrossProvider), Some(1));
        // Same provider without a separate model pool never qualifies.
        assert_eq!(
            select_failover(
                source,
                RoutingPolicy::DenyCrossProvider,
                &candidates[..1],
                &quotas,
                now
            ),
            None
        );
    }

    #[test]
    fn unknown_or_exhausted_shared_windows_never_imply_model_headroom() {
        let now = Utc::now();
        let candidates = [("codex", "gpt-other")];
        let mut quotas = vec![
            quota("codex", "gpt-other", 10.0, now),
            quota("codex", "codex", 100.0, now),
        ];
        assert_eq!(
            select_failover(
                ("codex", "gpt-source"),
                RoutingPolicy::Automatic,
                &candidates,
                &quotas,
                now
            ),
            None
        );
        quotas.pop();
        quotas[0].basis = TrustLevel::Unknown;
        assert_eq!(
            select_failover(
                ("codex", "gpt-source"),
                RoutingPolicy::Automatic,
                &candidates,
                &quotas,
                now
            ),
            None
        );
        assert_eq!(
            select_failover(
                ("codex", "gpt-source"),
                RoutingPolicy::Automatic,
                &candidates,
                &[],
                now
            ),
            None
        );
    }
}
