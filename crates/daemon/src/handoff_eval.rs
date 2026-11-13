//! Repeatable handoff quality evals (roadmap F3, ADR 0055).
//!
//! Each synthetic case under `fixtures/evals/handoff/` is a short conversation plus what a
//! handoff must keep. The runner builds the same context the daemon delivers to a new agent
//! (`continuation_prompt` over the newest events, after the same read-time cleaning) and the
//! zero-token packet summary, then scores both with `gnomcode_core::handoff_eval`. Fixtures are
//! synthetic: no real user data.

use crate::api::{AppState, validate_auth};
use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use chrono::{DateTime, TimeZone as _, Utc};
use gnomcode_core::handoff_eval::{
    EvalCaseResult, EvalReport, HandoffExpectation, build_report, describe, score_handoff,
};
use gnomcode_core::{ChatEvent, ChatRole, TrustLevel, handoff_summary, redact};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// History cap in `continuation_prompt` (16000) plus room for its framing and JSON escaping.
const DEFAULT_BUDGET_CHARS: usize = 18_000;
/// Upper bound on events one case may expand to, so a bad fixture cannot exhaust memory.
const MAX_EVENTS_PER_CASE: usize = 2_000;

const EMBEDDED: [&str; 9] = [
    include_str!("../../../fixtures/evals/handoff/dark-mode-css.json"),
    include_str!("../../../fixtures/evals/handoff/antigravity-to-claude.json"),
    include_str!("../../../fixtures/evals/handoff/login-redirect.json"),
    include_str!("../../../fixtures/evals/handoff/long-session.json"),
    include_str!("../../../fixtures/evals/handoff/orm-migration.json"),
    include_str!("../../../fixtures/evals/handoff/pricing-table.json"),
    include_str!("../../../fixtures/evals/handoff/secret-in-prompt.json"),
    include_str!("../../../fixtures/evals/handoff/small-budget.json"),
    include_str!("../../../fixtures/evals/handoff/tool-heavy.json"),
];

/// Failure while loading or storing eval data.
#[derive(Debug, Error)]
pub enum EvalError {
    /// A fixture file is not valid.
    #[error("invalid eval fixture {name}: {reason}")]
    Fixture {
        /// File or case name.
        name: String,
        /// What is wrong.
        reason: String,
    },
    /// Reading or writing failed.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// Storing the report failed.
    #[error("could not store report: {0}")]
    Store(#[from] gnomcode_platform::Error),
}

#[derive(Debug, Deserialize)]
struct FixtureEvent {
    role: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    tool_name: Option<String>,
    #[serde(default)]
    status: Option<String>,
    /// Emits this event several times, to model a long conversation compactly.
    #[serde(default)]
    repeat: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct FixtureCase {
    id: String,
    title: String,
    source_agent: String,
    target_agent: String,
    repo_name: String,
    #[serde(default)]
    budget_chars: Option<usize>,
    #[serde(default)]
    touched_files: Vec<String>,
    events: Vec<FixtureEvent>,
    expect: HandoffExpectation,
}

fn parse_case(name: &str, text: &str) -> Result<FixtureCase, EvalError> {
    serde_json::from_str(text).map_err(|error| EvalError::Fixture {
        name: name.to_owned(),
        reason: error.to_string(),
    })
}

fn chat_events(case: &FixtureCase) -> Result<Vec<ChatEvent>, EvalError> {
    let bad = |reason: String| EvalError::Fixture {
        name: case.id.clone(),
        reason,
    };
    let session_id = format!("{}:eval-{}", case.source_agent, case.id);
    let mut events = Vec::new();
    for item in &case.events {
        let role = ChatRole::parse_role(&item.role)
            .ok_or_else(|| bad(format!("unknown role {}", item.role)))?;
        for _ in 0..item.repeat.unwrap_or(1).max(1) {
            if events.len() >= MAX_EVENTS_PER_CASE {
                return Err(bad("too many events".into()));
            }
            let index = events.len();
            let created_at: DateTime<Utc> = Utc
                .timestamp_opt(1_700_000_000 + i64::try_from(index).unwrap_or(0), 0)
                .single()
                .unwrap_or_default();
            events.push(ChatEvent {
                event_id: format!("{}-{index}", case.id),
                session_id: session_id.clone(),
                turn_index: i64::try_from(index).unwrap_or(0),
                role,
                // The store redacts chat content when it is read back; do the same here.
                content: if role == ChatRole::Tool {
                    String::new()
                } else {
                    redact::redact_chat_content(&item.content)
                },
                tool_name: item.tool_name.clone(),
                status: item.status.clone(),
                created_at,
            });
        }
    }
    Ok(events)
}

fn run_case(case: &FixtureCase) -> Result<EvalCaseResult, EvalError> {
    let started = std::time::Instant::now();
    let mut events = chat_events(case)?;
    // Production fetches the first retained request independently of the newest page.
    let original = events
        .iter()
        .find(|event| event.role == ChatRole::User)
        .cloned();
    if events.len() > crate::HANDOFF_CONTEXT_EVENTS {
        events.drain(..events.len() - crate::HANDOFF_CONTEXT_EVENTS);
    }
    if let Some(first) = original
        && !events.iter().any(|event| event.event_id == first.event_id)
    {
        events.insert(0, first);
    }
    let events = gnomcode_adapters::codex::clean_stored_conversation(
        gnomcode_adapters::antigravity::clean_stored_conversation(events),
    );
    let budget = case.budget_chars.unwrap_or(DEFAULT_BUDGET_CHARS);
    let delivered = crate::continuation_prompt(&events, "Continue the task.");
    let score = score_handoff(&delivered, &case.expect, budget);
    let packet = format!(
        "{}\nFiles: {}",
        handoff_summary(&events),
        case.touched_files.join(", ")
    );
    let packet_score = score_handoff(&packet, &case.expect, budget);
    Ok(EvalCaseResult {
        id: case.id.clone(),
        title: case.title.clone(),
        source_agent: case.source_agent.clone(),
        target_agent: case.target_agent.clone(),
        repo_name: case.repo_name.clone(),
        outcome: if score.passed { "passed" } else { "failed" }.to_owned(),
        score_pct: score.score_pct,
        trust_level: TrustLevel::Estimated,
        duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        summary: describe(&score),
        details: score,
        packet_score_pct: packet_score.score_pct,
    })
}

fn run_cases(cases: &[FixtureCase]) -> Result<EvalReport, EvalError> {
    let results = cases.iter().map(run_case).collect::<Result<Vec<_>, _>>()?;
    Ok(build_report(Utc::now(), results))
}

/// Runs the eval set built into this binary.
///
/// # Errors
/// Returns an error when a built-in fixture is invalid (a build defect, covered by tests).
pub fn run_embedded() -> Result<EvalReport, EvalError> {
    let cases = EMBEDDED
        .iter()
        .enumerate()
        .map(|(index, text)| parse_case(&format!("embedded-{index}"), text))
        .collect::<Result<Vec<_>, _>>()?;
    run_cases(&cases)
}

/// Runs every `*.json` case in `dir`, sorted by file name.
///
/// # Errors
/// Returns an error when the directory cannot be read or a fixture is invalid.
pub fn run_dir(dir: &Path) -> Result<EvalReport, EvalError> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    paths.sort();
    let cases = paths
        .iter()
        .map(|path| parse_case(&path.display().to_string(), &std::fs::read_to_string(path)?))
        .collect::<Result<Vec<_>, _>>()?;
    run_cases(&cases)
}

/// Where the latest report is stored.
#[must_use]
pub fn report_path() -> PathBuf {
    gnomcode_platform::gnomcode_home()
        .join("evals")
        .join("handoff-latest.json")
}

/// Stores a report atomically with owner-only access.
///
/// # Errors
/// Returns an error when the report cannot be written.
pub fn save_report(path: &Path, report: &EvalReport) -> Result<(), EvalError> {
    let bytes = serde_json::to_vec_pretty(report).map_err(|error| EvalError::Fixture {
        name: "report".into(),
        reason: error.to_string(),
    })?;
    gnomcode_platform::write_private_file(path, &bytes)?;
    Ok(())
}

/// Reads the stored report, or `None` when it is absent or unreadable.
#[must_use]
pub fn load_report(path: &Path) -> Option<EvalReport> {
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

/// The stored report, or a freshly run built-in report that is stored for next time.
///
/// # Errors
/// Returns an error only when a fresh run is needed and fails.
pub fn latest_at(path: &Path) -> Result<EvalReport, EvalError> {
    if let Some(report) = load_report(path) {
        return Ok(report);
    }
    let report = run_embedded()?;
    // A report that cannot be stored is still returned: serving it must not depend on disk.
    let _ = save_report(path, &report);
    Ok(report)
}

/// `GET /api/v1/evals/handoff`: the latest report (ADR 0038, 0055).
pub(crate) async fn get_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<EvalReport>, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let report = tokio::task::spawn_blocking(|| latest_at(&report_path()))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(report))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_set_matches_the_fixture_directory() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/evals/handoff");
        let on_disk = std::fs::read_dir(&dir)
            .expect("fixtures")
            .filter_map(Result::ok)
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
            .count();
        assert_eq!(on_disk, EMBEDDED.len(), "add new fixtures to EMBEDDED");
    }

    #[test]
    fn embedded_run_is_repeatable_and_covers_every_provider() {
        let first = run_embedded().expect("run");
        let second = run_embedded().expect("run");
        let scores = |r: &EvalReport| -> Vec<(String, f64, f64)> {
            r.cases
                .iter()
                .map(|c| (c.id.clone(), c.score_pct, c.packet_score_pct))
                .collect()
        };
        assert_eq!(scores(&first), scores(&second));
        assert_eq!(first.summary.total_cases, 9);
        let providers: Vec<&str> = first
            .by_provider
            .iter()
            .map(|p| p.target_agent.as_str())
            .collect();
        assert_eq!(providers, ["antigravity", "claude-code", "codex"]);
        assert!(
            first
                .cases
                .iter()
                .all(|c| c.trust_level == TrustLevel::Estimated)
        );
    }

    #[test]
    fn a_normal_handoff_keeps_goals_files_and_open_tasks() {
        let report = run_embedded().expect("run");
        let case = report
            .cases
            .iter()
            .find(|c| c.id == "login-redirect")
            .expect("case");
        assert_eq!(case.outcome, "passed", "{}", case.summary);
        assert_eq!(case.details.goals.hit, case.details.goals.total);
    }

    #[test]
    fn redaction_keeps_pasted_secrets_out_of_the_handoff() {
        let report = run_embedded().expect("run");
        let case = report
            .cases
            .iter()
            .find(|c| c.id == "secret-in-prompt")
            .expect("case");
        assert!(case.details.leaked.is_empty(), "{:?}", case.details.leaked);
    }

    #[test]
    fn a_goal_buried_under_a_long_session_is_preserved() {
        let report = run_embedded().expect("run");
        let case = report
            .cases
            .iter()
            .find(|c| c.id == "long-session")
            .expect("case");
        assert_eq!(case.outcome, "passed", "{}", case.summary);
        assert!(case.details.goals.missing.is_empty());
        assert_eq!(report.summary.passed_cases, 9);
    }

    #[test]
    fn run_dir_reads_custom_cases_and_rejects_bad_ones() {
        let dir = tempfile::tempdir().expect("dir");
        std::fs::write(dir.path().join("a.json"), EMBEDDED[0]).expect("write");
        assert_eq!(run_dir(dir.path()).expect("run").summary.total_cases, 1);
        std::fs::write(dir.path().join("b.json"), "{ not json").expect("write");
        assert!(matches!(
            run_dir(dir.path()),
            Err(EvalError::Fixture { .. })
        ));
        let unknown_role = EMBEDDED[0].replace("\"role\": \"user\"", "\"role\": \"robot\"");
        std::fs::write(dir.path().join("b.json"), unknown_role).expect("write");
        assert!(matches!(
            run_dir(dir.path()),
            Err(EvalError::Fixture { .. })
        ));
    }

    #[test]
    fn latest_runs_once_then_serves_the_stored_report() {
        let dir = tempfile::tempdir().expect("dir");
        let path = dir.path().join("evals").join("handoff-latest.json");
        assert!(load_report(&path).is_none());
        let first = latest_at(&path).expect("first");
        assert!(path.is_file(), "report must be stored");
        let second = latest_at(&path).expect("second");
        assert_eq!(first.updated_at, second.updated_at);
        std::fs::write(&path, "corrupt").expect("corrupt");
        assert!(latest_at(&path).is_ok(), "a corrupt report is re-run");
    }

    #[tokio::test]
    async fn endpoint_requires_auth() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt as _;
        let app = crate::api::build_router(AppState {
            db: std::sync::Arc::new(tokio::sync::Mutex::new(
                gnomcode_store::Database::open_in_memory().expect("database"),
            )),
            bearer_token: "eval-token".into(),
            orchestrator: None,
            remote: std::sync::Arc::new(crate::remote::RemoteManager::new()),
        });
        let request = Request::builder()
            .uri("/api/v1/evals/handoff")
            .header("Host", "127.0.0.1")
            .body(Body::empty())
            .expect("request");
        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
