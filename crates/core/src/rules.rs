//! Rule evaluation engine and guardrails (`SPEC §3.8`, `§3.14`).
//!
//! Evaluates thresholds for:
//! - Spend caps (stop session when cost exceeds limit)
//! - Turn count caps
//! - Context window bloat warnings
//! - Auto-resume triggers when rate-limited quota resets

use crate::{AgentId, QuotaSnapshot, Session};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Action to perform when a guardrail rule triggers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleAction {
    /// Notify the user via desktop toast.
    Warn,
    /// Hard stop session execution via hook or signal.
    HardStop,
}

/// User-configured guardrail rule (`SPEC §3.8`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GuardrailRule {
    /// Unique rule identifier.
    pub id: String,
    /// Optional project scope filter.
    pub project_id: Option<String>,
    /// Optional agent scope filter.
    pub agent: Option<AgentId>,
    /// Spend cap threshold in USD.
    pub max_spend_usd: Option<f64>,
    /// Maximum turns permitted before trigger.
    pub max_turns: Option<u64>,
    /// Action to take on breach.
    pub action: RuleAction,
}

/// Armed entry in the Resume Queue (`SPEC §3.14`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumeQueueEntry {
    /// Session ID queued for auto-resume.
    pub session_id: String,
    /// Agent identifier.
    pub agent: AgentId,
    /// Working directory for headless execution fallback.
    pub cwd: String,
    /// Prompt delivered on resume (default `"continue"`).
    pub resume_prompt: String,
    /// Maximum permitted auto-resume iterations (stops loops, default 1).
    pub max_runs: u32,
    /// Number of auto-resumes executed so far.
    pub runs_completed: u32,
    /// UTC timestamp after which the entry is eligible to resume.
    pub eligible_after: DateTime<Utc>,
}

/// Evaluates a session against configured guardrail rules.
#[must_use]
pub fn evaluate_guardrails(
    session: &Session,
    accumulated_cost_usd: f64,
    turn_count: u64,
    rules: &[GuardrailRule],
) -> Option<RuleAction> {
    for rule in rules {
        if let Some(ref pid) = rule.project_id {
            if pid != &session.project_id {
                continue;
            }
        }
        if let Some(agent) = rule.agent {
            if agent != session.agent {
                continue;
            }
        }

        if let Some(max_cost) = rule.max_spend_usd {
            if accumulated_cost_usd >= max_cost {
                return Some(rule.action);
            }
        }

        if let Some(max_turns) = rule.max_turns {
            if turn_count >= max_turns {
                return Some(rule.action);
            }
        }
    }
    None
}

/// Determines whether an armed resume queue entry is ready to execute.
#[must_use]
pub fn is_resume_eligible(
    entry: &ResumeQueueEntry,
    now: DateTime<Utc>,
    quota_snapshot: Option<&QuotaSnapshot>,
) -> bool {
    if entry.runs_completed >= entry.max_runs {
        return false;
    }

    if now < entry.eligible_after {
        return false;
    }

    // If quota snapshot is provided, confirm it is no longer locked at 100%
    if let Some(q) = quota_snapshot {
        if q.used_pct >= 100.0 {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::{
        GuardrailRule, ResumeQueueEntry, RuleAction, evaluate_guardrails, is_resume_eligible,
    };
    use crate::{AgentId, QuotaSnapshot, QuotaWindow, Session, SessionState, Surface, TrustLevel};
    use chrono::{Duration, Utc};

    #[test]
    fn guardrail_triggers_hard_stop_on_spend_cap_breach() {
        let s = Session {
            id: "claude-code:s1".to_owned(),
            agent: AgentId::ClaudeCode,
            surface: Surface::Cli,
            project_id: "p1".to_owned(),
            parent_session_id: None,
            model: None,
            title: None,
            state: SessionState::Idle,
            pid: None,
            started_at: Utc::now(),
            last_event_at: Utc::now(),
        };

        let rules = [GuardrailRule {
            id: "cap-1".to_owned(),
            project_id: None,
            agent: Some(AgentId::ClaudeCode),
            max_spend_usd: Some(5.0),
            max_turns: None,
            action: RuleAction::HardStop,
        }];

        assert_eq!(evaluate_guardrails(&s, 2.5, 10, &rules), None);
        assert_eq!(
            evaluate_guardrails(&s, 5.0, 15, &rules),
            Some(RuleAction::HardStop)
        );
    }

    #[test]
    fn resume_queue_enforces_max_runs_and_reset_time() {
        let now = Utc::now();
        let mut entry = ResumeQueueEntry {
            session_id: "codex:s1".to_owned(),
            agent: AgentId::Codex,
            cwd: "/workspace".to_owned(),
            resume_prompt: "continue".to_owned(),
            max_runs: 1,
            runs_completed: 0,
            eligible_after: now + Duration::seconds(60),
        };

        // Not yet eligible (future reset time)
        assert!(!is_resume_eligible(&entry, now, None));

        // Eligible past reset time with quota available
        let reset_now = now + Duration::seconds(120);
        let q = QuotaSnapshot {
            provider: "openai".to_owned(),
            limit_id: "codex".to_owned(),
            window: QuotaWindow::FiveHour,
            used_pct: 0.0,
            resets_at: None,
            plan: None,
            basis: TrustLevel::Exact,
            observed_at: reset_now,
        };
        assert!(is_resume_eligible(&entry, reset_now, Some(&q)));

        // Once runs_completed reaches max_runs, it stops permanently
        entry.runs_completed = 1;
        assert!(!is_resume_eligible(&entry, reset_now, Some(&q)));
    }
}
