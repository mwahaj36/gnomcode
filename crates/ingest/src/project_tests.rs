//! Recomputing stored project identity (one test per bad name class seen in real data).

use super::{Project, reproject_all};
use chrono::Utc;
use gnomcode_core::{AgentId, Session, SessionState, Surface};
use gnomcode_store::{Database, SourceCheckpoint};
use std::collections::BTreeMap;

/// Stores `(stored key, folder)` projects the way earlier versions did, one session each, then
/// recomputes them and returns project name -> session count.
fn recompute(legacy: &[(&str, &str)]) -> BTreeMap<String, usize> {
    let db = recomputed_db(legacy);
    let (rows, total) = db
        .list_sessions_filtered(None, None, None, 0, 100)
        .expect("sessions");
    assert_eq!(total, legacy.len(), "no session is lost");
    let mut counts = BTreeMap::new();
    for row in rows {
        *counts.entry(row.project_name).or_default() += 1;
    }
    counts
}

/// Session `codex:s{i}` belongs to `legacy[i]`.
fn recomputed_db(legacy: &[(&str, &str)]) -> Database {
    let mut db = Database::open_in_memory().expect("db");
    let now = Utc::now();
    let projects: Vec<Project> = legacy
        .iter()
        .map(|(key, root)| Project {
            id: format!("legacy-{key}"),
            canonical_key: (*key).to_owned(),
            repo_root: (*root).to_owned(),
            name: key.rsplit('/').next().unwrap_or(key).to_owned(),
            git_branch: None,
        })
        .collect();
    let sessions: Vec<Session> = projects
        .iter()
        .enumerate()
        .map(|(i, project)| Session {
            id: format!("codex:s{i}"),
            agent: AgentId::Codex,
            surface: Surface::Cli,
            project_id: project.id.clone(),
            parent_session_id: None,
            model: None,
            title: None,
            state: SessionState::Idle,
            pid: None,
            started_at: now,
            last_event_at: now,
        })
        .collect();
    let checkpoint = SourceCheckpoint {
        file_path: "fixture".into(),
        last_read_offset: 0,
        file_size: 0,
        fingerprint: "fixture".into(),
        last_ingested_at: now,
    };
    db.commit_batch(&projects, &sessions, &[], &[], &[], &checkpoint)
        .expect("legacy rows");
    reproject_all(&mut db).expect("recompute");
    db
}

fn resume_folder(db: &Database, index: usize) -> Option<String> {
    db.get_session_focus_target(&format!("codex:s{index}"))
        .expect("target")
        .expect("session")
        .repo_root
}

#[test]
fn resume_uses_each_sessions_own_folder_never_the_merged_projects_last_one() {
    let bas = "C:\\Users\\W\\Documents\\Codex\\2026-10-21\\bas";
    let co = "C:\\Users\\W\\Documents\\Codex\\2026-10-21\\co";
    let temp = "C:/Users/W/AppData/Local/Temp/audit";
    let db = recomputed_db(&[
        ("c:/u/codex/2026-10-21/bas", bas),
        ("c:/u/codex/2026-10-21/co", co),
        ("c:/u/appdata/local/temp/audit", temp),
        ("c:", "C:/"),
    ]);
    assert_eq!(resume_folder(&db, 0).as_deref(), Some(bas));
    assert_eq!(resume_folder(&db, 1).as_deref(), Some(co));
    assert_eq!(resume_folder(&db, 2).as_deref(), Some(temp));
    assert_eq!(
        resume_folder(&db, 3),
        None,
        "unassigned sessions have no folder to resume in"
    );
}

#[test]
fn sessions_in_a_repository_keep_their_subfolder_and_fall_back_to_the_repository_root() {
    let base = tempfile::Builder::new()
        .prefix("repo")
        .tempdir_in(env!("CARGO_MANIFEST_DIR"))
        .expect("repo dir");
    let root = base.path().join("app");
    std::fs::create_dir_all(root.join(".git")).expect("git dir");
    std::fs::create_dir_all(root.join("crates").join("daemon")).expect("subfolder");
    let root = root.to_string_lossy().into_owned();
    let sub = format!(
        "{root}{}crates{}daemon",
        std::path::MAIN_SEPARATOR,
        std::path::MAIN_SEPARATOR
    );
    let key = |text: &str| text.replace('\\', "/").to_lowercase();
    let db = recomputed_db(&[(&key(&root), &root), (&key(&sub), &sub)]);
    assert_eq!(resume_folder(&db, 1).as_deref(), Some(sub.as_str()));
    assert_eq!(resume_folder(&db, 0).as_deref(), Some(root.as_str()));
}

#[test]
fn new_batches_record_the_sessions_own_folder_even_when_the_project_merges_folders() {
    use super::{record_cwds, resolve_batch_projects};
    use gnomcode_adapters::{FileCtx, SourceAdapter, codex::CodexAdapter};
    let mut db = Database::open_in_memory().expect("db");
    let folders = [
        "C:\\Users\\W\\Documents\\Codex\\2026-10-21\\bas",
        "C:\\Users\\W\\Documents\\Codex\\2026-11-09\\cle",
    ];
    for (index, folder) in folders.iter().enumerate() {
        let meta = serde_json::json!({"type":"session_meta","timestamp":"2026-11-02T00:00:00Z","payload":{"id":format!("chat{index}"),"cwd":folder}});
        let log = format!("{meta}\n");
        let mut batch = CodexAdapter::new().parse(&FileCtx::default(), log.as_bytes());
        let cwds = resolve_batch_projects(&mut batch);
        let checkpoint = SourceCheckpoint {
            file_path: format!("chat{index}"),
            last_read_offset: 0,
            file_size: 0,
            fingerprint: "fixture".into(),
            last_ingested_at: Utc::now(),
        };
        db.commit_batch(&batch.projects, &batch.sessions, &[], &[], &[], &checkpoint)
            .expect("commit");
        record_cwds(&db, &cwds);
    }
    assert_eq!(
        db.list_projects().expect("projects").len(),
        1,
        "one Codex chats project"
    );
    for (index, folder) in folders.iter().enumerate() {
        let own = db
            .get_session_focus_target(&format!("codex:chat{index}"))
            .expect("target")
            .expect("session")
            .repo_root;
        assert_eq!(own.as_deref(), Some(*folder));
    }
}

#[test]
fn drive_roots_become_unassigned_instead_of_c_and_d() {
    let counts = recompute(&[
        ("c:", "C:/"),
        ("d:", "D:/"),
        ("/unknown", "/unknown"),
        ("ubuntu-22.04", "//wsl.localhost/Ubuntu-22.04/"),
    ]);
    assert_eq!(counts, BTreeMap::from([("unknown".to_owned(), 4)]));
}

#[test]
fn doubled_separators_and_percent_encoding_merge_into_one_project() {
    let counts = recompute(&[
        (
            "c:/users/wahaj/documents/github/notii",
            "c:/Users/Wahaj/Documents/GitHub/notii",
        ),
        (
            "c://users//wahaj//documents//github//notii",
            "C:\\\\Users\\\\Wahaj\\\\Documents\\\\GitHub\\\\notii",
        ),
        (
            "c:/users/wahaj/documents/infosec%20assignment1/a2",
            "c:/Users/Wahaj/Documents/infosec%20assignment1/a2",
        ),
    ]);
    assert_eq!(
        counts,
        BTreeMap::from([("a2".to_owned(), 1), ("notii".to_owned(), 2)])
    );
}

#[test]
fn codex_chat_folders_and_temporary_directories_collapse_to_one_row_each() {
    let counts = recompute(&[
        (
            "c:/u/codex/2026-10-21/bas",
            "C:\\Users\\W\\Documents\\Codex\\2026-10-21\\bas",
        ),
        (
            "c:/u/codex/2026-10-21/co",
            "C:\\Users\\W\\Documents\\Codex\\2026-10-21\\co",
        ),
        (
            "c:/u/codex/2026-11-09/cle",
            "C:\\Users\\W\\Documents\\Codex\\2026-11-09\\cle",
        ),
        (
            "d:/spawnt-ide-test-temp/real/account-retest-1793248674048",
            "D:\\spawnt-ide-test-temp\\real\\account-retest-1793248674048",
        ),
        (
            "c:/u/appdata/local/temp/audit",
            "C:/Users/W/AppData/Local/Temp/audit",
        ),
    ]);
    assert_eq!(
        counts,
        BTreeMap::from([
            ("Codex chats".to_owned(), 3),
            ("Temporary folders".to_owned(), 2)
        ])
    );
}

#[test]
fn subfolders_and_removed_worktrees_belong_to_their_repository() {
    let base = tempfile::Builder::new()
        .prefix("repo")
        .tempdir_in(env!("CARGO_MANIFEST_DIR"))
        .expect("repo dir");
    let root = base.path().join("fleethud");
    std::fs::create_dir_all(root.join(".git")).expect("git dir");
    std::fs::create_dir_all(root.join("crates").join("daemon")).expect("subfolder");
    let root = root.to_string_lossy().into_owned();
    let sep = std::path::MAIN_SEPARATOR;
    let sub = format!("{root}{sep}crates{sep}daemon");
    let worktree = format!("{root}{sep}.claude{sep}worktrees{sep}agent-a1");
    let key = |text: &str| text.replace('\\', "/").to_lowercase();
    let counts = recompute(&[
        (&key(&root), &root),
        (&key(&sub), &sub),
        (&key(&worktree), &worktree),
    ]);
    assert_eq!(counts, BTreeMap::from([("fleethud".to_owned(), 3)]));
}

#[test]
fn a_remote_keyed_project_keeps_its_key_when_the_folder_is_gone() {
    let counts = recompute(&[(
        "github.com/mwahaj36/devsplain",
        "C:\\Users\\Wahaj\\Documents\\GitHub\\devsplain",
    )]);
    assert_eq!(counts, BTreeMap::from([("devsplain".to_owned(), 1)]));
}

#[test]
fn wsl_logs_map_linux_folders_to_the_distro_share() {
    use super::{map_wsl_projects, resolve_batch_projects};
    use gnomcode_adapters::{FileCtx, SourceAdapter, codex::CodexAdapter};
    let meta = serde_json::json!({"type":"session_meta","timestamp":"2026-11-02T00:00:00Z","payload":{"id":"w1","cwd":"/home/wahaj/proj"}});
    let log = format!("{meta}\n");
    let mut batch = CodexAdapter::new().parse(&FileCtx::default(), log.as_bytes());
    map_wsl_projects(&mut batch, "Ubuntu-22.04");
    let cwds = resolve_batch_projects(&mut batch);
    let project = &batch.projects[0];
    assert_eq!(project.name, "proj");
    assert_eq!(
        gnomcode_core::wsl::distro_of(&project.repo_root),
        Some("Ubuntu-22.04")
    );
    assert_eq!(batch.sessions[0].project_id, project.id);
    assert_eq!(cwds[0].1, r"\\wsl.localhost\Ubuntu-22.04\home\wahaj\proj");
    // Same folder spelled through wsl$ is the same project key.
    let other = Project::from_origin(None, r"\\wsl$\Ubuntu-22.04\home\wahaj\proj", None);
    assert_eq!(other.canonical_key, project.canonical_key);
}
