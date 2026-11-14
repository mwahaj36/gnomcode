//! Antigravity session titles stay on the first prompt across incremental ingests.

use gnomcode_adapters::{FileCtx, SourceAdapter, antigravity::AntigravityAdapter};
use gnomcode_store::{Database, SourceCheckpoint};
use std::path::PathBuf;

const CONVERSATION: &str = "00000000-0000-4000-8000-000000000001";

fn fixture(dir: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/antigravity")
            .join(dir)
            .join("multiturn.jsonl"),
    )
    .expect("fixture")
}

fn ingest(db: &mut Database, data_dir: &str, offset: u64, chunk: &[u8]) {
    let ctx = FileCtx {
        source_path: format!(
            "/h/.gemini/{data_dir}/brain/{CONVERSATION}/.system_generated/logs/transcript.jsonl"
        ),
        source_offset: offset,
        fallback_cwd: Some("/workspace".into()),
        ..FileCtx::default()
    };
    let batch = AntigravityAdapter::new().parse(&ctx, chunk);
    let checkpoint = SourceCheckpoint {
        file_path: ctx.source_path.clone(),
        last_read_offset: offset + chunk.len() as u64,
        file_size: offset + chunk.len() as u64,
        fingerprint: String::new(),
        last_ingested_at: chrono::Utc::now(),
    };
    db.commit_batch_with_chat(
        &batch.projects,
        &batch.sessions,
        &batch.usage_records,
        &batch.activity_events,
        &batch.quota_snapshots,
        &checkpoint,
        &batch.chat_events,
    )
    .expect("commit");
}

fn title(dir: &tempfile::TempDir) -> Option<String> {
    rusqlite::Connection::open(dir.path().join("t.db"))
        .expect("open")
        .query_row(
            "SELECT title FROM session WHERE id = ?1",
            [format!("antigravity:{CONVERSATION}")],
            |row| row.get(0),
        )
        .expect("session")
}

#[test]
fn every_surface_keeps_its_first_prompt_as_title() {
    for (dir, data_dir) in [
        ("cli-1.5.8", "antigravity-cli"),
        ("ide-1.107.0", "antigravity-ide"),
        ("app-2.31.2", "antigravity"),
    ] {
        let bytes = fixture(dir);
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut db = Database::open(&tmp.path().join("t.db")).expect("db");
        // One ingest per line, like a watcher tailing the file as the conversation grows.
        let mut offset = 0;
        for line in bytes.split_inclusive(|b| *b == b'\n') {
            ingest(&mut db, data_dir, offset, line);
            offset += line.len() as u64;
        }
        assert_eq!(
            title(&tmp).as_deref(),
            Some("Build the widget feed parser"),
            "{dir}"
        );
    }
}

#[test]
fn a_tail_that_starts_at_a_later_prompt_does_not_replace_the_title() {
    let bytes = fixture("cli-1.5.8");
    let split = bytes
        .split_inclusive(|b| *b == b'\n')
        .take(3)
        .map(<[u8]>::len)
        .sum::<usize>();
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut db = Database::open(&tmp.path().join("t.db")).expect("db");
    ingest(&mut db, "antigravity-cli", 0, &bytes[..split]);
    ingest(&mut db, "antigravity-cli", split as u64, &bytes[split..]);
    assert_eq!(title(&tmp).as_deref(), Some("Build the widget feed parser"));
}

#[test]
fn drifted_titles_are_restored_from_the_first_prompt_on_migration() {
    let bytes = fixture("ide-1.107.0");
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join("t.db");
    {
        let mut db = Database::open(&path).expect("db");
        ingest(&mut db, "antigravity-ide", 0, &bytes);
    }
    let conn = rusqlite::Connection::open(&path).expect("open");
    conn.execute_batch(
        "UPDATE session SET title = 'im going to sleep continue working';
         DELETE FROM schema_migrations WHERE version = 16;",
    )
    .expect("drift");
    drop(conn);
    drop(Database::open(&path).expect("reopen"));
    assert_eq!(title(&tmp).as_deref(), Some("Build the widget feed parser"));
}
