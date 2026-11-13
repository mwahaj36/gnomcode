//! Read-only per-session git status and diff for local API and E2EE Remote.

use crate::api::{AppState, validate_auth};
use axum::Json;
use axum::extract::{Path as AxumPath, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use gnomcode_store::Database;
use serde::Serialize;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

const MAX_GIT_BYTES: u64 = 1_048_576;
const GIT_TIMEOUT_SECS: u64 = 10;

#[derive(Debug, Clone, Serialize)]
struct SessionChanges {
    repo: String,
    diff: String,
    status: String,
    lossy: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChangesError {
    NotGitRepo,
    TooLarge,
    Timeout,
    Io,
}

impl ChangesError {
    fn status_code(self) -> StatusCode {
        match self {
            Self::NotGitRepo => StatusCode::BAD_REQUEST,
            Self::TooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            Self::Timeout => StatusCode::GATEWAY_TIMEOUT,
            Self::Io => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::NotGitRepo => "Workspace is not a git repository",
            Self::TooLarge => "Workspace diff exceeds 1 MiB limit",
            Self::Timeout => "Git status or diff timed out",
            Self::Io => "Could not inspect workspace changes",
        }
    }
}

fn resolve_session_workspace(db: &Database, session_id: &str) -> Result<PathBuf, &'static str> {
    if session_id.trim().is_empty() || session_id.len() > 128 {
        return Err("Invalid session");
    }
    let hosted = gnomcode_pty::list_active_sessions();
    if let Some(record) = hosted.iter().find(|r| r.session_id == session_id) {
        let path = PathBuf::from(&record.repo_path);
        if path.is_dir() {
            return Ok(path);
        }
    }
    let sessions = db.list_active_sessions(usize::MAX).unwrap_or_default();
    let native_id = crate::remote::conversation_session_id(session_id, &sessions, &hosted);
    if let Some(record) = hosted.iter().find(|r| {
        sessions
            .iter()
            .find(|s| s.id == native_id)
            .is_some_and(|s| Some(r.child_pid) == s.pid || r.session_id == s.id)
    }) {
        let path = PathBuf::from(&record.repo_path);
        if path.is_dir() {
            return Ok(path);
        }
    }
    let target = db
        .get_session_focus_target(&native_id)
        .map_err(|_| "Session workspace is unavailable")?
        .ok_or("Session not found")?;
    let folder = target
        .repo_root
        .filter(|path| !path.trim().is_empty())
        .ok_or("Session has no workspace folder")?;
    let path = PathBuf::from(folder);
    if path.is_dir() {
        Ok(path)
    } else {
        Err("Session workspace folder is unavailable")
    }
}

async fn run_git_readonly(repo: &Path, args: &[&str]) -> Result<(String, bool), ChangesError> {
    use tokio::io::AsyncReadExt as _;
    let mut command = tokio::process::Command::new("git");
    command
        .args([
            "--no-pager",
            "-c",
            "core.quotepath=off",
            "-c",
            "core.fsmonitor=false",
        ])
        .args(args)
        .current_dir(repo)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_LITERAL_PATHSPECS", "1")
        .env("LC_ALL", "C")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(0x0800_0000);
    let mut child = command.spawn().map_err(|_| ChangesError::Io)?;
    let stdout = child.stdout.take().ok_or(ChangesError::Io)?;
    tokio::time::timeout(std::time::Duration::from_secs(GIT_TIMEOUT_SECS), async {
        let mut bytes = Vec::new();
        stdout
            .take(MAX_GIT_BYTES + 1)
            .read_to_end(&mut bytes)
            .await
            .map_err(|_| ChangesError::Io)?;
        if bytes.len() as u64 > MAX_GIT_BYTES {
            let _ = child.kill().await;
            return Err(ChangesError::TooLarge);
        }
        let status = child.wait().await.map_err(|_| ChangesError::Io)?;
        if !status.success() {
            return Err(ChangesError::NotGitRepo);
        }
        Ok(match String::from_utf8(bytes) {
            Ok(text) => (text, false),
            Err(error) => (String::from_utf8_lossy(error.as_bytes()).into_owned(), true),
        })
    })
    .await
    .map_err(|_| ChangesError::Timeout)?
}

async fn collect_changes(workspace: &Path) -> Result<SessionChanges, ChangesError> {
    let canonical = workspace
        .canonicalize()
        .map_err(|_| ChangesError::NotGitRepo)?;
    if !canonical.is_dir() {
        return Err(ChangesError::NotGitRepo);
    }
    let (diff, diff_lossy) = run_git_readonly(
        &canonical,
        &["diff", "--no-ext-diff", "--no-textconv", "HEAD"],
    )
    .await?;
    let (status_raw, status_lossy) =
        run_git_readonly(&canonical, &["status", "--porcelain=v1", "-z"]).await?;
    let status = status_raw.replace('\0', "\n");
    Ok(SessionChanges {
        repo: gnomcode_platform::workspaces::display_path(&canonical),
        diff,
        status,
        lossy: diff_lossy || status_lossy,
    })
}

/// Returns read-only git status and diff for a session's resolved workspace.
pub(crate) async fn get_handler(
    headers: HeaderMap,
    AxumPath(session_id): AxumPath<String>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let workspace = {
        let db = state.db.lock().await;
        resolve_session_workspace(&db, &session_id).map_err(|_| StatusCode::NOT_FOUND)?
    };
    let changes = collect_changes(&workspace)
        .await
        .map_err(ChangesError::status_code)?;
    Ok(Json(json!({
        "session_id": session_id,
        "repo": changes.repo,
        "diff": changes.diff,
        "status": changes.status,
        "lossy": changes.lossy,
    })))
}

/// Handles E2EE Remote `changes` requests for a session's workspace.
pub(crate) async fn remote_command(db: &Arc<Mutex<Database>>, request: &Value) -> Value {
    let Some(session_id) = request.get("session_id").and_then(Value::as_str) else {
        return json!({"type": "error", "message": "Missing session"});
    };
    let workspace = {
        let db = db.lock().await;
        match resolve_session_workspace(&db, session_id) {
            Ok(path) => path,
            Err(message) => return json!({"type": "error", "message": message}),
        }
    };
    match collect_changes(&workspace).await {
        Ok(changes) => json!({
            "type": "changes",
            "session_id": session_id,
            "repo": changes.repo,
            "diff": changes.diff,
            "status": changes.status,
            "lossy": changes.lossy,
        }),
        Err(err) => json!({"type": "error", "message": err.message()}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gnomcode_core::{AgentId, Project, Session, SessionState, Surface};

    fn seeded_db_with_session(session_id: &str, repo: &Path) -> Arc<Mutex<Database>> {
        let mut db = Database::open_in_memory().expect("in-memory db");
        let project = Project::from_origin(None, &repo.to_string_lossy(), None);
        let now = chrono::Utc::now();
        let session = Session {
            id: session_id.to_owned(),
            agent: AgentId::Codex,
            surface: Surface::Cli,
            project_id: project.id.clone(),
            parent_session_id: None,
            pid: None,
            title: Some("Changes Test".to_owned()),
            model: None,
            state: SessionState::Idle,
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
        db.commit_batch(&[project], &[session], &[], &[], &[], &checkpoint)
            .expect("commit");
        db.record_session_cwd(session_id, &repo.to_string_lossy())
            .expect("cwd");
        Arc::new(Mutex::new(db))
    }

    #[tokio::test]
    async fn remote_changes_returns_git_status_and_diff_and_handles_non_utf8_lossily() {
        let repo = tempfile::tempdir().expect("repo");
        let git = |args: &[&str]| {
            assert!(
                std::process::Command::new("git")
                    .args(args)
                    .current_dir(repo.path())
                    .status()
                    .expect("git")
                    .success()
            );
        };
        git(&["init"]);
        git(&["config", "user.email", "test@example.com"]);
        git(&["config", "user.name", "Test"]);
        std::fs::write(repo.path().join("tracked.txt"), "one\n").expect("write");
        git(&["add", "tracked.txt"]);
        git(&["commit", "-m", "init"]);

        std::fs::write(repo.path().join("tracked.txt"), b"one\n\xff\xfe\n").expect("modify");
        std::fs::write(repo.path().join("untracked.txt"), "hello\n").expect("untracked");

        let db = seeded_db_with_session("codex:changes-test", repo.path());
        let response = remote_command(
            &db,
            &json!({"type": "changes", "session_id": "codex:changes-test"}),
        )
        .await;
        assert_eq!(response["type"], "changes");
        assert_eq!(response["lossy"], true);
        let status = response["status"].as_str().expect("status");
        assert!(status.contains("M tracked.txt"));
        assert!(status.contains("?? untracked.txt"));
        let diff = response["diff"].as_str().expect("diff");
        assert!(diff.contains("tracked.txt"));
    }

    #[tokio::test]
    async fn remote_changes_rejects_non_git_workspace() {
        let dir = tempfile::tempdir().expect("non-git");
        let db = seeded_db_with_session("codex:nongit-test", dir.path());
        let response = remote_command(
            &db,
            &json!({"type": "changes", "session_id": "codex:nongit-test"}),
        )
        .await;
        assert_eq!(response["type"], "error");
    }
}
