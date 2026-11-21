//! Read-only per-session git status and diff for local API and E2EE Remote.

use crate::api::{AppState, validate_auth};
use crate::workspace_files::resolve_session_workspace;
use axum::Json;
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use gnomcode_store::Database;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::Path;
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
    truncated: bool,
}

#[derive(Default, Deserialize)]
pub(crate) struct ChangesQuery {
    path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChangesError {
    NotGitRepo,
    TooLarge,
    Timeout,
    Io,
    InvalidPath,
}

impl ChangesError {
    fn status_code(self) -> StatusCode {
        match self {
            Self::NotGitRepo => StatusCode::BAD_REQUEST,
            Self::TooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            Self::Timeout => StatusCode::GATEWAY_TIMEOUT,
            Self::Io => StatusCode::INTERNAL_SERVER_ERROR,
            Self::InvalidPath => StatusCode::BAD_REQUEST,
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::NotGitRepo => "Workspace is not a git repository",
            Self::TooLarge => "Workspace diff exceeds 1 MiB limit",
            Self::Timeout => "Git status or diff timed out",
            Self::Io => "Could not inspect workspace changes",
            Self::InvalidPath => "Choose a relative file inside the workspace",
        }
    }
}

async fn run_git_readonly(repo: &Path, args: &[&str]) -> Result<(String, bool), ChangesError> {
    use tokio::io::AsyncReadExt as _;
    let mut command = tokio::process::Command::new("git");
    command
        .args([
            "--no-pager",
            "-c",
            "safe.directory=*",
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

async fn collect_changes(
    workspace: &Path,
    path: Option<&str>,
) -> Result<SessionChanges, ChangesError> {
    if let Some(path) = path
        && (path.is_empty()
            || !Path::new(path)
                .components()
                .all(|part| matches!(part, std::path::Component::Normal(_))))
    {
        return Err(ChangesError::InvalidPath);
    }
    let canonical = workspace
        .canonicalize()
        .map_err(|_| ChangesError::NotGitRepo)?;
    if !canonical.is_dir() {
        return Err(ChangesError::NotGitRepo);
    }
    // A managed worktree is reviewed against the commit it was cut from, so committed work shows.
    let base = match crate::worktree_session::diff_base(
        &crate::worktree_session::managed_root(),
        &canonical,
    ) {
        Some(base) => base,
        // A repository with no commits has no HEAD, which `git diff HEAD` rejects; a new
        // project is exactly where someone opens Changes first.
        None if run_git_readonly(&canonical, &["rev-parse", "--verify", "--quiet", "HEAD"])
            .await
            .is_err() =>
        {
            gnomcode_core::EMPTY_TREE_OBJECT.to_owned()
        }
        None => "HEAD".to_owned(),
    };
    let mut args = vec!["diff", "--no-ext-diff", "--no-textconv", &base];
    if let Some(path) = path {
        args.extend(["--", path]);
    }
    let (diff, diff_lossy, truncated) = match run_git_readonly(&canonical, &args).await {
        Ok((diff, lossy)) => (diff, lossy, false),
        // Keep the complete bounded status list so a large review remains navigable.
        Err(ChangesError::TooLarge) => (String::new(), false, true),
        Err(error) => return Err(error),
    };
    let (status_raw, status_lossy) =
        run_git_readonly(&canonical, &["status", "--porcelain=v1", "-z"]).await?;
    let status = status_raw.replace('\0', "\n");
    Ok(SessionChanges {
        repo: gnomcode_platform::workspaces::display_path(&canonical),
        diff,
        status,
        lossy: diff_lossy || status_lossy,
        truncated,
    })
}

/// Returns read-only git status and diff for a session's resolved workspace.
pub(crate) async fn get_handler(
    headers: HeaderMap,
    AxumPath(session_id): AxumPath<String>,
    Query(query): Query<ChangesQuery>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let workspace = {
        let db = state.db.lock().await;
        resolve_session_workspace(&db, &session_id).map_err(|_| StatusCode::NOT_FOUND)?
    };
    let changes = collect_changes(&workspace, query.path.as_deref())
        .await
        .map_err(ChangesError::status_code)?;
    Ok(Json(json!({
        "session_id": session_id,
        "repo": changes.repo,
        "diff": changes.diff,
        "status": changes.status,
        "lossy": changes.lossy,
        "truncated": changes.truncated,
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
    match collect_changes(&workspace, request.get("path").and_then(Value::as_str)).await {
        Ok(changes) => json!({
            "type": "changes",
            "session_id": session_id,
            "repo": changes.repo,
            "diff": changes.diff,
            "status": changes.status,
            "lossy": changes.lossy,
            "truncated": changes.truncated,
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

    #[tokio::test]
    async fn oversized_review_keeps_file_list_and_allows_a_single_file_diff() {
        let repo = tempfile::tempdir().expect("repo");
        for args in [
            vec!["init"],
            vec!["config", "user.email", "test@example.com"],
            vec!["config", "user.name", "Test"],
        ] {
            assert!(
                std::process::Command::new("git")
                    .args(args)
                    .current_dir(repo.path())
                    .output()
                    .expect("git")
                    .status
                    .success()
            );
        }
        std::fs::write(repo.path().join("large.txt"), "base\n").expect("base");
        std::fs::write(repo.path().join("small.txt"), "base\n").expect("base");
        for args in [vec!["add", "."], vec!["commit", "-m", "base"]] {
            assert!(
                std::process::Command::new("git")
                    .args(args)
                    .current_dir(repo.path())
                    .output()
                    .expect("git")
                    .status
                    .success()
            );
        }
        std::fs::write(
            repo.path().join("large.txt"),
            "large change\n".repeat(100_000),
        )
        .expect("large change");
        std::fs::write(repo.path().join("small.txt"), "small change\n").expect("small change");
        let db = seeded_db_with_session("codex:large-review", repo.path());
        let response = remote_command(&db, &json!({"session_id": "codex:large-review"})).await;
        assert_eq!(response["type"], "changes", "{response}");
        assert_eq!(response["truncated"], true);
        assert!(
            response["status"]
                .as_str()
                .expect("status")
                .contains("small.txt")
        );
        let response = remote_command(
            &db,
            &json!({"session_id": "codex:large-review", "path": "small.txt"}),
        )
        .await;
        assert_eq!(response["truncated"], false);
        let diff = response["diff"].as_str().expect("diff");
        assert!(diff.contains("+small change"));
        assert!(!diff.contains("large.txt"));
        let rejected = remote_command(
            &db,
            &json!({"session_id": "codex:large-review", "path": "../outside.txt"}),
        )
        .await;
        assert_eq!(rejected["type"], "error");
    }

    #[tokio::test]
    async fn remote_changes_resolves_switched_conversation_root_id() {
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
        std::fs::write(repo.path().join("README.md"), "base\n").expect("write");
        git(&["add", "README.md"]);
        git(&["commit", "-m", "init"]);
        std::fs::write(repo.path().join("README.md"), "edited\n").expect("edit");

        let db = Database::open_in_memory().expect("in-memory db");
        let repo_str = repo.path().to_string_lossy().into_owned();
        db.save_conversation_launch("claude-root-1", "claude", &repo_str, Some("sonnet"))
            .expect("launch");
        db.continue_conversation("claude-root-1", "switch-target-1")
            .expect("continue");
        let db = Arc::new(Mutex::new(db));

        let response = remote_command(
            &db,
            &json!({"type": "changes", "session_id": "claude-root-1"}),
        )
        .await;
        assert_eq!(response["type"], "changes", "got {response}");
        assert!(
            response["status"]
                .as_str()
                .is_some_and(|s| s.contains("M README.md")),
            "status={response}"
        );
    }
}
