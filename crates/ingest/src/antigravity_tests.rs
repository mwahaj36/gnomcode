//! End-to-end Antigravity ingestion across the CLI, IDE and 2.0-app data directories.

use super::{ingest_files, scan_log_files};
use gnomcode_core::AgentId;
use gnomcode_platform::AgentRoot;
use gnomcode_store::Database;
use std::fs;
use std::path::{Path, PathBuf};

const IDE: &str = "1de00000-0000-4000-8000-000000000001";
const APP: &str = "a9900000-0000-4000-8000-000000000002";
const CLI: &str = "c1100000-0000-4000-8000-000000000003";

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/antigravity")
}

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len() / 2)
        .map(|i| u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).expect("hex"))
        .collect()
}

/// Writes `conversations/<id>.db` from a surface's trajectory fixture.
fn write_trajectory(path: &Path, fixture: &str) {
    let value: serde_json::Value = serde_json::from_slice(
        &fs::read(fixtures().join(fixture).join("trajectory.json")).unwrap(),
    )
    .unwrap();
    fs::create_dir_all(path.parent().expect("directory")).expect("conversations");
    let conn = rusqlite::Connection::open(path).expect("trajectory");
    conn.execute_batch(
        "CREATE TABLE gen_metadata(idx integer PRIMARY KEY, data blob, size integer NOT NULL DEFAULT 0);
         CREATE TABLE steps(idx integer PRIMARY KEY, metadata blob);",
    )
    .expect("schema");
    for g in value["generations"].as_array().expect("generations") {
        add_generation(
            &conn,
            g["idx"].as_i64().expect("idx"),
            &unhex(g["blob"].as_str().expect("blob")),
        );
    }
    for (i, step) in value["steps"].as_array().expect("steps").iter().enumerate() {
        conn.execute(
            "INSERT INTO steps(idx, metadata) VALUES (?1, ?2)",
            rusqlite::params![i as i64, unhex(step.as_str().expect("step"))],
        )
        .expect("step");
    }
}

fn add_generation(conn: &rusqlite::Connection, idx: i64, blob: &[u8]) {
    conn.execute(
        "INSERT INTO gen_metadata(idx, data) VALUES (?1, ?2)",
        rusqlite::params![idx, blob],
    )
    .expect("generation");
}

fn write_transcript(data: &Path, id: &str, fixture: &str) {
    let logs = data
        .join("brain")
        .join(id)
        .join(".system_generated")
        .join("logs");
    fs::create_dir_all(&logs).expect("logs");
    fs::copy(
        fixtures().join(fixture).join("transcript.jsonl"),
        logs.join("transcript.jsonl"),
    )
    .expect("transcript");
}

fn tokens(db: &Database) -> Vec<(String, String, u64, u64, u64)> {
    let (mut rows, total) = db
        .list_sessions_filtered(None, None, None, 0, 100)
        .expect("rows");
    assert_eq!(rows.len(), total);
    rows.sort_by(|a, b| a.id.cmp(&b.id));
    rows.into_iter()
        .map(|r| {
            (
                r.id,
                r.surface,
                r.input_tokens,
                r.output_tokens,
                r.cache_read_tokens,
            )
        })
        .collect()
}

fn usage_rows(db: &Database, session: &str) -> u64 {
    db.session_guardrail_totals(session).expect("count").1
}

fn pass(db: &mut Database, roots: &[AgentRoot]) {
    let discovered = scan_log_files(roots);
    ingest_files(db, roots, &discovered, false).expect("ingest");
}

#[test]
fn every_surface_is_ingested_once_and_a_migrated_copy_is_not_counted_again() {
    const CHILD: &str = "GNOMCODE_TEST_AGY_SURFACES";
    if std::env::var_os(CHILD).is_none() {
        let root = tempfile::tempdir().expect("root");
        let status = std::process::Command::new(std::env::current_exe().expect("test binary"))
            .args([
                "--exact",
                "antigravity_tests::every_surface_is_ingested_once_and_a_migrated_copy_is_not_counted_again",
            ])
            .env(CHILD, root.path())
            .env("HOME", root.path())
            .env("USERPROFILE", root.path())
            .env("APPDATA", root.path())
            .env("LOCALAPPDATA", root.path())
            .env("XDG_CONFIG_HOME", root.path())
            .env("XDG_DATA_HOME", root.path())
            .env("CODEX_HOME", root.path().join("codex"))
            .env("CLAUDE_CONFIG_DIR", root.path().join("claude"))
            .env("GEMINI_HOME", root.path().join("gemini"))
            .status()
            .expect("isolated test");
        assert!(status.success());
        return;
    }
    let gemini = PathBuf::from(std::env::var_os(CHILD).expect("root")).join("gemini");
    let (hub, ide, cli) = (
        gemini.join("antigravity"),
        gemini.join("antigravity-ide"),
        gemini.join("antigravity-cli"),
    );

    // IDE 1.107: transcript without token fields, trajectory next to it, and the copy the 2.0 app
    // made when it migrated the conversation (same id, no transcript of its own).
    write_transcript(&ide, IDE, "ide-1.107.0");
    write_trajectory(
        &ide.join("conversations").join(format!("{IDE}.db")),
        "ide-1.107.0",
    );
    fs::copy(ide.join("conversations").join(format!("{IDE}.db")), {
        fs::create_dir_all(hub.join("conversations")).expect("hub conversations");
        hub.join("conversations").join(format!("{IDE}.db"))
    })
    .expect("migrated copy");
    fs::create_dir_all(hub.join("brain").join(IDE)).expect("hub artifact folder");
    // 2.0 app and CLI conversations: transcripts with token fields plus trajectories.
    write_transcript(&hub, APP, "app-2.31.2");
    write_trajectory(
        &hub.join("conversations").join(format!("{APP}.db")),
        "app-2.31.2",
    );
    write_transcript(&cli, CLI, "cli-1.5.8");
    write_trajectory(
        &cli.join("conversations").join(format!("{CLI}.db")),
        "cli-1.5.8",
    );

    let roots: Vec<AgentRoot> = [&hub, &ide, &cli]
        .into_iter()
        .map(|data| AgentRoot {
            agent: AgentId::Antigravity,
            path: data.join("brain"),
            exists: true,
        })
        .collect();
    let mut db = Database::open_in_memory().expect("db");

    pass(&mut db, &roots);
    let first = tokens(&db);
    let surface = |id: &str| {
        first
            .iter()
            .find(|row| row.0 == format!("antigravity:{id}"))
            .expect("session")
            .1
            .clone()
    };
    assert_eq!(first.len(), 3, "one session per conversation id");
    assert_eq!(
        surface(IDE),
        "ide",
        "a migrated copy keeps the IDE as owner"
    );
    assert_eq!(surface(APP), "desktop_app");
    assert_eq!(surface(CLI), "cli");
    let ide_row = first.iter().find(|row| row.0.ends_with(IDE)).expect("ide");
    assert_eq!((ide_row.2, ide_row.3, ide_row.4), (41_900, 832, 310_000));
    // Transcript usage written before the trajectory was read is replaced, not added to.
    assert_eq!(usage_rows(&db, &format!("antigravity:{IDE}")), 3);
    assert_eq!(usage_rows(&db, &format!("antigravity:{APP}")), 8);
    assert_eq!(usage_rows(&db, &format!("antigravity:{CLI}")), 8);

    // Re-ingesting changes nothing.
    pass(&mut db, &roots);
    pass(&mut db, &roots);
    assert_eq!(tokens(&db), first);

    // A conversation continued in the 2.0 app grows only its hub copy: the new call is counted
    // once and the session stays with the IDE.
    let hub_copy = rusqlite::Connection::open(hub.join("conversations").join(format!("{IDE}.db")))
        .expect("hub copy");
    let blob: Vec<u8> = hub_copy
        .query_row("SELECT data FROM gen_metadata WHERE idx = 0", [], |row| {
            row.get(0)
        })
        .expect("blob");
    add_generation(&hub_copy, 3, &blob);
    drop(hub_copy);
    pass(&mut db, &roots);
    pass(&mut db, &roots);
    assert_eq!(usage_rows(&db, &format!("antigravity:{IDE}")), 4);
    let after = tokens(&db);
    assert_eq!(
        after
            .iter()
            .find(|row| row.0.ends_with(IDE))
            .expect("ide")
            .1,
        "ide"
    );
    assert_eq!(
        after
            .iter()
            .find(|row| row.0.ends_with(IDE))
            .expect("ide")
            .2,
        61_900
    );
    assert_eq!(after.iter().filter(|row| !row.0.ends_with(IDE)).count(), 2);
}
