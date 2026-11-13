//! End-to-end failover test: Claude Code exhausted -> select Codex -> compile handoff -> continue in Codex.

use chrono::{Duration, Utc};
use gnomcode_core::routing::select_failover;
use gnomcode_core::{
    AgentId, ChatEvent, ChatRole, Project, QuotaSnapshot, QuotaWindow, RoutingPolicy, Session,
    SessionState, Surface, TrustLevel,
};
use gnomcode_daemon::prepare_handoff;
use gnomcode_store::{Database, SourceCheckpoint};
use tempfile::tempdir;

#[test]
fn claude_exhausted_selects_codex_and_continues_conversation_with_handoff() {
    let dir = tempdir().expect("tempdir");
    let repo = dir.path().join("repo");
    std::fs::create_dir_all(&repo).expect("mkdir repo");
    std::fs::write(repo.join("README.md"), "# Demo\n").expect("write readme");

    let mut db = Database::open(&dir.path().join("failover.db")).expect("open db");
    let now = Utc::now();
    let repo_str = repo.to_string_lossy().into_owned();
    let project = Project::from_origin(Some(&repo_str), &repo_str, Some("main"));

    let claude_sid = "claude-code:failover-src";
    let claude_sess = Session {
        id: claude_sid.to_owned(),
        agent: AgentId::ClaudeCode,
        surface: Surface::Cli,
        project_id: project.id.clone(),
        parent_session_id: None,
        model: Some("claude-opus-5-5".to_owned()),
        title: Some("Fix failing tests".to_owned()),
        state: SessionState::Error,
        pid: None,
        started_at: now - Duration::minutes(10),
        last_event_at: now,
    };
    let chat = vec![
        ChatEvent {
            event_id: format!("{claude_sid}:u1"),
            session_id: claude_sid.to_owned(),
            turn_index: 1,
            role: ChatRole::User,
            content: "Fix the failing unit test in src/lib.rs".to_owned(),
            tool_name: None,
            status: None,
            created_at: now - Duration::minutes(5),
        },
        ChatEvent {
            event_id: format!("{claude_sid}:a1"),
            session_id: claude_sid.to_owned(),
            turn_index: 2,
            role: ChatRole::Assistant,
            content: "I found the bug in src/lib.rs and will patch it next.".to_owned(),
            tool_name: None,
            status: None,
            created_at: now - Duration::minutes(4),
        },
    ];
    let quotas = vec![
        QuotaSnapshot {
            provider: "claude-code".to_owned(),
            limit_id: "claude".to_owned(),
            window: QuotaWindow::FiveHour,
            used_pct: 100.0,
            resets_at: Some(now + Duration::hours(3)),
            plan: None,
            basis: TrustLevel::Exact,
            observed_at: now,
        },
        QuotaSnapshot {
            provider: "codex".to_owned(),
            limit_id: "codex".to_owned(),
            window: QuotaWindow::FiveHour,
            used_pct: 10.0,
            resets_at: Some(now + Duration::hours(4)),
            plan: None,
            basis: TrustLevel::Exact,
            observed_at: now,
        },
    ];
    let cp = SourceCheckpoint {
        file_path: "claude.jsonl".to_owned(),
        last_read_offset: 10,
        file_size: 10,
        fingerprint: "fp1".to_owned(),
        last_ingested_at: now,
    };
    db.commit_batch_with_chat(&[project], &[claude_sess], &[], &[], &quotas, &cp, &chat)
        .expect("commit claude + quotas");

    // 1. Routing selects Codex over exhausted Claude Code.
    let latest_quotas = db.get_latest_quota_snapshots().expect("latest quotas");
    let candidates = [("claude-code", "claude-opus-5-5"), ("codex", "gpt-5.4")];
    let selected = select_failover(
        ("claude-code", "claude-opus-5-5"),
        RoutingPolicy::Automatic,
        &candidates,
        &latest_quotas,
        now,
    )
    .expect("codex failover candidate");
    let target = candidates[selected];
    assert_eq!(target.0, "codex");
    assert_eq!(target.1, "gpt-5.4");

    // 2. Compile handoff packet from the exhausted Claude session.
    let packet = prepare_handoff(&db, claude_sid, target.1).expect("handoff packet");
    assert!(
        packet
            .prompt_summary
            .contains("Fix the failing unit test in src/lib.rs")
    );

    // 3. Continue conversation on Codex host while preserving conversation identity.
    db.bind_conversation_host("host-claude-1", claude_sid)
        .expect("bind claude host");
    db.continue_conversation(claude_sid, "host-codex-2")
        .expect("continue on codex");
    let root_id = db.conversation_id(claude_sid).expect("root");
    assert_eq!(root_id, "host-claude-1");
}
