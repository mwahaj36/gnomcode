//! Guardrails execution and the Resume Queue. Gnomcode is free: there is no licence.

use gnomcode_core::AgentId;
use gnomcode_core::rules::{RuleAction, evaluate_guardrails};
use gnomcode_platform::gnomcode_home;
use gnomcode_store::Database;

use std::fs;
use std::time::{Duration, SystemTime};

/// Evaluates persisted rules after ingestion and records alerts.
/// Claude hard stops are read by the fail-open PreToolUse hook.
pub fn run_guardrails(db: &Database) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let block_path = gnomcode_home().join("blocked-sessions.json");
    let rules = db.list_rules()?;
    let sessions = db.list_active_sessions(500)?;
    let mut blocked = Vec::new();

    for session in sessions {
        let (cost, turns) = db.session_guardrail_totals(&session.id)?;
        for rule in &rules {
            if let Some(action) =
                evaluate_guardrails(&session, cost, turns, std::slice::from_ref(rule))
            {
                let action_name = match action {
                    RuleAction::Warn => "warn",
                    RuleAction::HardStop => "hard_stop",
                };
                let is_new = db.insert_alert_once(&session.id, &rule.id, action_name)?;
                if is_new {
                    db.automation_notice(&format!("guardrail:{}:{}",session.id,rule.id),&session.id,"A guardrail was triggered. Open Automations to review the rule and session.")?;
                    tracing::info!(
                        "Guardrail triggered for {}: rule {} ({})",
                        session.id,
                        rule.id,
                        action_name
                    );
                }
                if action == RuleAction::HardStop && session.agent == AgentId::ClaudeCode {
                    blocked.push(session.id.clone());
                }
            }
        }
    }

    // Persist blocked sessions list with fresh timestamp for fail-open safety
    fs::write(block_path, serde_json::to_vec(&blocked)?)?;
    Ok(())
}

/// Retires context rotations for providers that manage their own context.
/// Explicit user-requested model and provider switches remain available.
pub fn run_context_relay(db: &Database) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    for relay in db.list_context_relays()? {
        if relay.status == "pending" {
            db.update_context_relay_status(&relay.id, "dismissed")?;
        }
    }
    Ok(())
}

/// Executes a Context Relay handoff:
/// 1. Writes zero-knowledge .gnomcode/HANDOFF.md checkpoint in project directory.
/// 2. Spawns fresh agent session in an interactive terminal window without permission escalation.
pub fn execute_relay_handoff(
    entry: &gnomcode_store::ContextRelayEntry,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let repo_dir = std::path::Path::new(&entry.cwd);
    let gnomcode_dir = repo_dir.join(".gnomcode");
    let _ = std::fs::create_dir_all(&gnomcode_dir);

    let handoff_path = gnomcode_dir.join("HANDOFF.md");
    let content = format!(
        "# Gnomcode Context Relay Checkpoint\n\n\
        - **Source Agent:** {}\n\
        - **Target Agent:** {}\n\
        - **Project:** {}\n\
        - **Saturation:** {}% context consumed (Safety reserve active)\n\
        - **Timestamp:** {}\n\n\
        ## Continuity Instructions for Fresh Session\n\
        This workspace was handed off by Gnomcode Context Relay because the previous session reached the context saturation limit.\n\
        1. Inspect modified workspace files and recent git status (`git status`).\n\
        2. Verify existing test suites (`cargo test` / `npm test`).\n\
        3. Continue the task from this clean context window without repeating saturated tokens.\n",
        entry.source_agent,
        entry.target_agent,
        entry.project_name,
        entry.fill_pct,
        entry.created_at,
    );
    std::fs::write(&handoff_path, content)?;

    // Spawn the fresh target agent session in an interactive terminal or focus editor
    if !gnomcode_platform::spawn_agent_in_terminal(&entry.target_agent, &entry.cwd) {
        return Err("agent session could not launch".into());
    }

    let title = format!("[RELAY] Handoff Complete: {}", entry.target_agent);
    let body = format!(
        "Checkpoint created in .gnomcode/HANDOFF.md for {}. Clean session initialized.",
        entry.project_name
    );
    let _ = gnomcode_platform::send_desktop_notification(&title, &body);

    Ok(())
}

/// Checks whether a session is currently blocked by a guardrail rule.
/// Used for 50ms loopback IPC query.
pub fn is_session_blocked(session_id: &str) -> bool {
    let block_path = gnomcode_home().join("blocked-sessions.json");
    let Ok(meta) = fs::metadata(&block_path) else {
        return false;
    };

    // Stale check: if file is older than 30s, fail open
    if let Ok(mtime) = meta.modified()
        && let Ok(elapsed) = SystemTime::now().duration_since(mtime)
        && elapsed > Duration::from_secs(30)
    {
        return false;
    }

    let Ok(raw) = fs::read(block_path) else {
        return false;
    };
    let Ok(blocked) = serde_json::from_slice::<Vec<String>>(&raw) else {
        return false;
    };
    blocked.iter().any(|s| s == session_id)
}

#[cfg(test)]
mod remote_entitlement_tests {
    #[test]
    fn relay_handoff_does_not_claim_success_when_agent_cannot_launch() {
        let repo = tempfile::tempdir().expect("repo");
        let entry = gnomcode_store::ContextRelayEntry {
            id: "relay-test".to_owned(),
            session_id: "codex:test".to_owned(),
            source_agent: "codex".to_owned(),
            target_agent: "unsupported-agent".to_owned(),
            project_name: "test".to_owned(),
            cwd: repo.path().to_string_lossy().into_owned(),
            fill_pct: 92,
            status: "pending".to_owned(),
            created_at: chrono::Utc::now().to_rfc3339(),
            executed_at: None,
        };
        assert!(super::execute_relay_handoff(&entry).is_err());
    }

    #[test]
    fn native_compaction_clears_pending_relays_without_erasing_history() {
        let db = gnomcode_store::Database::open_in_memory().expect("db");
        for agent in ["claude-code", "codex", "antigravity"] {
            db.record_context_relay(&gnomcode_store::ContextRelayEntry {
                id: format!("relay-{agent}"),
                session_id: format!("{agent}:test"),
                source_agent: agent.into(),
                target_agent: agent.into(),
                project_name: "test".into(),
                cwd: "test".into(),
                fill_pct: 90,
                status: "pending".into(),
                created_at: chrono::Utc::now().to_rfc3339(),
                executed_at: None,
            })
            .expect("record");
        }
        super::run_context_relay(&db).expect("relay pass");
        let history = db.list_context_relays().expect("history");
        assert_eq!(history.len(), 3);
        assert!(history.iter().all(|row| row.status != "pending"));
    }

    #[test]
    fn context_relay_skips_auto_compacting_ides_and_agents() {
        let mut db = gnomcode_store::Database::open_in_memory().expect("db");
        let project = gnomcode_core::Project::from_origin(None, "/tmp/test", None);

        let now = chrono::Utc::now();
        let ide_sess = gnomcode_core::Session {
            id: "codex:ide-1".into(),
            agent: gnomcode_core::AgentId::Codex,
            surface: gnomcode_core::Surface::Ide,
            project_id: project.id.clone(),
            parent_session_id: None,
            model: Some("gpt-6-luna".into()),
            title: None,
            state: gnomcode_core::SessionState::Thinking,
            pid: None,
            started_at: now,
            last_event_at: now,
        };
        let ide_usage = gnomcode_core::UsageRecord {
            dedup_key: "usage-ide-1".into(),
            session_id: ide_sess.id.clone(),
            timestamp: now,
            model: "gpt-6-luna".into(),
            input_tokens: 95000,
            output_tokens: 5000,
            reasoning_tokens: 0,
            cache_read_tokens: 0,
            cache_write_5m_tokens: 0,
            cache_write_1h_tokens: 0,
            context_tokens: 95000,
            context_window: Some(100000),
            cost_usd: 0.1,
            cache_savings_usd: 0.0,
            cost_basis: gnomcode_core::TrustLevel::Exact,
            pricing_version: "test".into(),
        };

        let agy_sess = gnomcode_core::Session {
            id: "antigravity:cli-1".into(),
            agent: gnomcode_core::AgentId::Antigravity,
            surface: gnomcode_core::Surface::Cli,
            project_id: project.id.clone(),
            parent_session_id: None,
            model: Some("gemini-3.8-flash".into()),
            title: None,
            state: gnomcode_core::SessionState::Thinking,
            pid: None,
            started_at: now,
            last_event_at: now,
        };
        let agy_usage = gnomcode_core::UsageRecord {
            dedup_key: "usage-agy-1".into(),
            session_id: agy_sess.id.clone(),
            timestamp: now,
            model: "gemini-3.8-flash".into(),
            input_tokens: 95000,
            output_tokens: 5000,
            reasoning_tokens: 0,
            cache_read_tokens: 0,
            cache_write_5m_tokens: 0,
            cache_write_1h_tokens: 0,
            context_tokens: 95000,
            context_window: Some(100000),
            cost_usd: 0.1,
            cache_savings_usd: 0.0,
            cost_basis: gnomcode_core::TrustLevel::Exact,
            pricing_version: "test".into(),
        };

        let cp = gnomcode_store::SourceCheckpoint {
            file_path: "/tmp/log.jsonl".into(),
            last_read_offset: 0,
            file_size: 0,
            fingerprint: "test".into(),
            last_ingested_at: now,
        };
        db.commit_batch(
            &[project],
            &[ide_sess, agy_sess],
            &[ide_usage, agy_usage],
            &[],
            &[],
            &cp,
        )
        .expect("commit batch");

        super::run_context_relay(&db).expect("run_context_relay");
        let relays = db.list_context_relays().expect("list relays");
        assert!(
            relays.is_empty(),
            "auto-compacting sessions must not generate context relays"
        );
    }
}
