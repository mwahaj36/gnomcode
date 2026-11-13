//! Handoff quality scoring (roadmap F3, ADR 0055).
//!
//! Pure and deterministic: given the text a target agent receives and what that text should
//! carry, count how much of it survived. All scores here are keyword-coverage proxies over
//! synthetic cases, so every figure is labelled `estimated`, never `exact`.

use crate::TrustLevel;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Weight of goal coverage in the score (percent).
const GOALS_WEIGHT: f64 = 40.0;
/// Weight of file coverage in the score (percent).
const FILES_WEIGHT: f64 = 25.0;
/// Weight of open-task coverage in the score (percent).
const TASKS_WEIGHT: f64 = 25.0;
/// Weight of staying inside the length budget (percent).
const LENGTH_WEIGHT: f64 = 10.0;
/// A case passes at or above this score, inside its budget and with no leaked secret.
pub const PASS_THRESHOLD_PCT: f64 = 80.0;

/// What a handoff for one case must carry.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandoffExpectation {
    /// Phrases naming the user's goals.
    #[serde(default)]
    pub goals: Vec<String>,
    /// File paths that matter to the work.
    #[serde(default)]
    pub files: Vec<String>,
    /// Phrases naming unfinished work.
    #[serde(default)]
    pub open_tasks: Vec<String>,
    /// Strings that must never appear (secrets that redaction should remove).
    #[serde(default)]
    pub forbidden: Vec<String>,
}

/// How many expected items one category found.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Coverage {
    /// Items found in the handoff.
    pub hit: u32,
    /// Items expected.
    pub total: u32,
    /// Items that were not found.
    pub missing: Vec<String>,
}

/// Score of one handoff text.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct HandoffScore {
    /// Goal coverage.
    pub goals: Coverage,
    /// File coverage.
    pub files: Coverage,
    /// Open-task coverage.
    pub open_tasks: Coverage,
    /// Characters in the handoff text.
    pub chars: usize,
    /// Allowed characters.
    pub budget_chars: usize,
    /// Forbidden strings that appeared.
    pub leaked: Vec<String>,
    /// Weighted score, 0 to 100, one decimal.
    pub score_pct: f64,
    /// Whether the case passes.
    pub passed: bool,
}

fn normalize(text: &str) -> String {
    text.to_lowercase()
        .replace('\\', "/")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn coverage(haystack: &str, wanted: &[String]) -> Coverage {
    let missing: Vec<String> = wanted
        .iter()
        .filter(|item| !haystack.contains(&normalize(item)))
        .cloned()
        .collect();
    let total = u32::try_from(wanted.len()).unwrap_or(u32::MAX);
    Coverage {
        hit: total - u32::try_from(missing.len()).unwrap_or(total),
        total,
        missing,
    }
}

fn share(found: &Coverage, weight: f64) -> f64 {
    if found.total == 0 {
        weight
    } else {
        weight * f64::from(found.hit) / f64::from(found.total)
    }
}

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

/// Scores one handoff text against an expectation and a length budget.
///
/// Phrases match case-insensitively with whitespace collapsed and backslashes read as slashes.
#[must_use]
pub fn score_handoff(text: &str, expect: &HandoffExpectation, budget_chars: usize) -> HandoffScore {
    let haystack = normalize(text);
    let goals = coverage(&haystack, &expect.goals);
    let files = coverage(&haystack, &expect.files);
    let open_tasks = coverage(&haystack, &expect.open_tasks);
    let chars = text.chars().count();
    let within_budget = chars <= budget_chars;
    let leaked: Vec<String> = expect
        .forbidden
        .iter()
        .filter(|secret| !secret.is_empty() && text.contains(secret.as_str()))
        .cloned()
        .collect();
    let score_pct = round1(
        share(&goals, GOALS_WEIGHT)
            + share(&files, FILES_WEIGHT)
            + share(&open_tasks, TASKS_WEIGHT)
            + if within_budget { LENGTH_WEIGHT } else { 0.0 },
    );
    HandoffScore {
        goals,
        files,
        open_tasks,
        chars,
        budget_chars,
        passed: score_pct >= PASS_THRESHOLD_PCT && within_budget && leaked.is_empty(),
        leaked,
        score_pct,
    }
}

/// One-line, human-readable breakdown of a score.
#[must_use]
pub fn describe(score: &HandoffScore) -> String {
    let mut line = format!(
        "goals {}/{}, files {}/{}, open tasks {}/{}, {}/{} chars",
        score.goals.hit,
        score.goals.total,
        score.files.hit,
        score.files.total,
        score.open_tasks.hit,
        score.open_tasks.total,
        score.chars,
        score.budget_chars
    );
    if !score.leaked.is_empty() {
        line.push_str(", secret leaked");
    }
    line
}

/// Result of one evaluated case, shaped for `GET /api/v1/evals/handoff` (ADR 0038, 0055).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalCaseResult {
    /// Stable case identifier.
    pub id: String,
    /// Short case title.
    pub title: String,
    /// Agent the conversation came from.
    pub source_agent: String,
    /// Agent that receives the handoff.
    pub target_agent: String,
    /// Synthetic repository name.
    pub repo_name: String,
    /// `passed` or `failed`.
    pub outcome: String,
    /// Score of the handoff the target agent receives.
    pub score_pct: f64,
    /// Always `estimated`: keyword coverage over synthetic cases.
    pub trust_level: TrustLevel,
    /// Time to compile and score, in milliseconds.
    pub duration_ms: u64,
    /// One-line breakdown.
    pub summary: String,
    /// Full breakdown of the delivered handoff.
    pub details: HandoffScore,
    /// Score of the zero-token packet summary for the same conversation.
    pub packet_score_pct: f64,
}

/// Totals across all cases.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalSummary {
    /// Cases run.
    pub total_cases: u32,
    /// Cases that passed.
    pub passed_cases: u32,
    /// Share of passing cases, one decimal.
    pub pass_rate_pct: f64,
    /// Mean case score, one decimal.
    pub mean_score_pct: f64,
    /// Always `estimated`.
    pub trust_level: TrustLevel,
}

/// Totals for one target provider.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderSummary {
    /// Receiving agent.
    pub target_agent: String,
    /// Cases for this provider.
    pub cases: u32,
    /// Passing cases.
    pub passed_cases: u32,
    /// Share of passing cases, one decimal.
    pub pass_rate_pct: f64,
    /// Mean case score, one decimal.
    pub mean_score_pct: f64,
}

/// Complete report: totals, per-provider totals and every case.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalReport {
    /// When the report was produced.
    pub updated_at: DateTime<Utc>,
    /// Totals.
    pub summary: EvalSummary,
    /// Totals per receiving provider, sorted by name.
    pub by_provider: Vec<ProviderSummary>,
    /// Case results in input order.
    pub cases: Vec<EvalCaseResult>,
}

fn rates(cases: &[&EvalCaseResult]) -> (u32, u32, f64, f64) {
    let total = u32::try_from(cases.len()).unwrap_or(u32::MAX);
    let passed = u32::try_from(cases.iter().filter(|case| case.outcome == "passed").count())
        .unwrap_or(u32::MAX);
    if total == 0 {
        return (0, 0, 0.0, 0.0);
    }
    let mean = cases.iter().map(|case| case.score_pct).sum::<f64>() / f64::from(total);
    (
        total,
        passed,
        round1(100.0 * f64::from(passed) / f64::from(total)),
        round1(mean),
    )
}

/// Builds the report from case results. An empty input yields an empty, well-formed report.
#[must_use]
pub fn build_report(updated_at: DateTime<Utc>, cases: Vec<EvalCaseResult>) -> EvalReport {
    let all: Vec<&EvalCaseResult> = cases.iter().collect();
    let (total_cases, passed_cases, pass_rate_pct, mean_score_pct) = rates(&all);
    let mut providers: Vec<&str> = cases
        .iter()
        .map(|case| case.target_agent.as_str())
        .collect();
    providers.sort_unstable();
    providers.dedup();
    let by_provider = providers
        .into_iter()
        .map(|name| {
            let group: Vec<&EvalCaseResult> = cases
                .iter()
                .filter(|case| case.target_agent == name)
                .collect();
            let (cases, passed_cases, pass_rate_pct, mean_score_pct) = rates(&group);
            ProviderSummary {
                target_agent: name.to_owned(),
                cases,
                passed_cases,
                pass_rate_pct,
                mean_score_pct,
            }
        })
        .collect();
    EvalReport {
        updated_at,
        summary: EvalSummary {
            total_cases,
            passed_cases,
            pass_rate_pct,
            mean_score_pct,
            trust_level: TrustLevel::Estimated,
        },
        by_provider,
        cases,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expect() -> HandoffExpectation {
        HandoffExpectation {
            goals: vec!["fix the login redirect".into(), "add retry".into()],
            files: vec!["src/auth/login.rs".into()],
            open_tasks: vec!["update the migration".into()],
            forbidden: vec!["sk-live-123".into()],
        }
    }

    #[test]
    fn full_coverage_inside_budget_scores_100_and_passes() {
        let text = "Goal: Fix the Login Redirect and add   retry. Touched src/auth/login.rs. TODO: update the migration";
        let score = score_handoff(text, &expect(), 1000);
        assert_eq!(score.score_pct, 100.0);
        assert!(score.passed);
        assert_eq!((score.goals.hit, score.goals.total), (2, 2));
        assert!(score.goals.missing.is_empty());
    }

    #[test]
    fn missing_items_are_listed_and_lower_the_weighted_score() {
        let text = "Goal: fix the login redirect. src/auth/login.rs";
        let score = score_handoff(text, &expect(), 1000);
        // goals 1/2 of 40 + files 25 + tasks 0 + length 10
        assert_eq!(score.score_pct, 55.0);
        assert!(!score.passed);
        assert_eq!(score.goals.missing, ["add retry"]);
        assert_eq!(score.open_tasks.missing, ["update the migration"]);
    }

    #[test]
    fn windows_separators_and_case_do_not_hide_a_file() {
        let text =
            r"edited SRC\AUTH\LOGIN.RS and fix the login redirect, add retry, update the migration";
        assert_eq!(score_handoff(text, &expect(), 1000).files.hit, 1);
    }

    #[test]
    fn exceeding_the_budget_loses_the_length_share_and_fails() {
        let text = "fix the login redirect add retry src/auth/login.rs update the migration";
        let score = score_handoff(text, &expect(), 10);
        assert_eq!(score.score_pct, 90.0);
        assert!(!score.passed);
        assert_eq!(score.chars, text.chars().count());
    }

    #[test]
    fn a_leaked_secret_fails_even_with_full_coverage() {
        let text =
            "fix the login redirect add retry src/auth/login.rs update the migration sk-live-123";
        let score = score_handoff(text, &expect(), 1000);
        assert_eq!(score.leaked, ["sk-live-123"]);
        assert!(!score.passed);
    }

    #[test]
    fn empty_expectations_count_as_covered() {
        let score = score_handoff("anything", &HandoffExpectation::default(), 100);
        assert_eq!(score.score_pct, 100.0);
        assert!(score.passed);
    }

    fn result(id: &str, target: &str, score: f64, passed: bool) -> EvalCaseResult {
        EvalCaseResult {
            id: id.into(),
            title: id.into(),
            source_agent: "claude-code".into(),
            target_agent: target.into(),
            repo_name: "demo".into(),
            outcome: if passed { "passed" } else { "failed" }.into(),
            score_pct: score,
            trust_level: TrustLevel::Estimated,
            duration_ms: 1,
            summary: String::new(),
            details: HandoffScore::default(),
            packet_score_pct: 0.0,
        }
    }

    #[test]
    fn report_totals_and_provider_groups_are_computed_in_rust() {
        let report = build_report(
            Utc::now(),
            vec![
                result("a", "codex", 100.0, true),
                result("b", "codex", 50.0, false),
                result("c", "gemini-cli", 90.0, true),
            ],
        );
        assert_eq!(report.summary.total_cases, 3);
        assert_eq!(report.summary.passed_cases, 2);
        assert_eq!(report.summary.pass_rate_pct, 66.7);
        assert_eq!(report.summary.mean_score_pct, 80.0);
        assert_eq!(report.summary.trust_level, TrustLevel::Estimated);
        let names: Vec<&str> = report
            .by_provider
            .iter()
            .map(|p| p.target_agent.as_str())
            .collect();
        assert_eq!(names, ["codex", "gemini-cli"]);
        assert_eq!(report.by_provider[0].pass_rate_pct, 50.0);
        assert_eq!(report.by_provider[0].mean_score_pct, 75.0);
    }

    #[test]
    fn empty_report_is_well_formed() {
        let report = build_report(Utc::now(), Vec::new());
        assert_eq!(report.summary.total_cases, 0);
        assert_eq!(report.summary.pass_rate_pct, 0.0);
        assert!(report.cases.is_empty() && report.by_provider.is_empty());
        let json = serde_json::to_value(&report).expect("json");
        assert_eq!(json["summary"]["trust_level"], "estimated");
    }

    #[test]
    fn describe_names_each_category_and_a_leak() {
        let score = score_handoff("x sk-live-123", &expect(), 5);
        let line = describe(&score);
        assert!(
            line.contains("goals 0/2")
                && line.contains("13/5 chars")
                && line.contains("secret leaked")
        );
    }
}
