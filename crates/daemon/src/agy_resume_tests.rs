//! Regression coverage for saved Antigravity conversation capabilities.

#[test]
fn saved_antigravity_native_conversation_advertises_resume() {
    use gnomcode_core::{AgentId, Project, Session, SessionState, Surface};
    let repo = tempfile::tempdir().expect("workspace");
    let root = repo.path().to_string_lossy();
    let project = Project::from_origin(None, &root, None);
    let now = chrono::Utc::now();
    let session = Session {
        id: "antigravity:11111111-2222-4333-8444-555555555555".into(),
        agent: AgentId::Antigravity,
        surface: Surface::Cli,
        project_id: project.id.clone(),
        parent_session_id: None,
        model: None,
        title: None,
        state: SessionState::Idle,
        pid: None,
        started_at: now,
        last_event_at: now,
    };
    let checkpoint = gnomcode_store::SourceCheckpoint {
        file_path: "fixture".into(),
        last_read_offset: 0,
        file_size: 0,
        fingerprint: "fixture".into(),
        last_ingested_at: now,
    };
    let mut db = gnomcode_store::Database::open_in_memory().expect("database");
    db.commit_batch(
        &[project],
        std::slice::from_ref(&session),
        &[],
        &[],
        &[],
        &checkpoint,
    )
    .expect("session");
    db.record_session_cwd(&session.id, &root)
        .expect("workspace");
    let imported = crate::remote::conversation_sessions(&db, std::slice::from_ref(&session), &[])
        .expect("imported conversations");
    let imported = imported
        .iter()
        .find(|row| row["id"] == session.id)
        .expect("imported session");
    assert_eq!(
        imported["can_resume"], false,
        "imported history must remain read-only"
    );
    db.save_conversation_launch("agy-host", "antigravity", &root, None)
        .expect("launch");
    db.bind_conversation_host("agy-host", &session.id)
        .expect("binding");
    let rows = crate::remote::conversation_sessions(&db, &[session], &[]).expect("conversations");
    let saved = rows
        .iter()
        .find(|row| row["id"] == "agy-host")
        .expect("saved conversation");
    assert_eq!(saved["can_resume"], true);
}
