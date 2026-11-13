//! HTTP and E2EE Remote surface for worktree sessions (roadmap F1, ADR 0051).
//!
//! Local: `POST /api/v1/sessions/{id}/worktree/{merge,discard,open-pr}`; launch accepts
//! `worktree` and `worktree_branch`. Remote: `worktree_merge`, `worktree_discard`,
//! `worktree_open_pr` (reply `worktree_action_ack`) and `session_changes`.

use crate::api::{AppState, validate_auth};
use crate::worktree_session::{self as wt, WorktreeError, WorktreeRecord};
use axum::Json;
use axum::extract::{Path as AxumPath, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use gnomcode_store::Database;
use serde_json::{Value, json};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Merge, discard and pull-request actions change the same repository; run one at a time.
static ACTIONS: Mutex<()> = Mutex::const_new(());

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    Merge,
    Discard,
    OpenPr,
}

impl Action {
    fn name(self) -> &'static str {
        match self {
            Self::Merge => "merge",
            Self::Discard => "discard",
            Self::OpenPr => "open_pr",
        }
    }
}

/// What an action did.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Outcome {
    pub(crate) message: String,
    pub(crate) url: Option<String>,
    pub(crate) merged_commit: Option<String>,
}

impl Outcome {
    fn body(&self) -> Value {
        let mut body = json!({"ok":true,"message":self.message});
        if let Some(url) = &self.url {
            body["url"] = json!(url);
        }
        if let Some(commit) = &self.merged_commit {
            body["merged_commit"] = json!(commit);
        }
        body
    }
}

/// Finds the worktree record for a session id, a conversation id or its current host.
async fn find(
    db: &Mutex<Database>,
    root: &Path,
    id: &str,
) -> Result<WorktreeRecord, WorktreeError> {
    if id.is_empty() || id.len() > 128 {
        return Err(WorktreeError::NotFound);
    }
    let mut candidates = vec![id.to_owned()];
    {
        let guard = db.lock().await;
        if let Ok(conversation) = guard.conversation_id(id) {
            candidates.push(conversation);
        }
        if let Ok(Some((host, _, _))) = guard.conversation_target(id) {
            candidates.push(host);
        }
    }
    candidates
        .iter()
        .find_map(|candidate| wt::load_record(root, candidate))
        .ok_or(WorktreeError::NotFound)
}

/// Points the conversation back at the original repository once the worktree is gone, so a
/// later resume does not look for a folder that no longer exists.
async fn restore_launch_repo(db: &Mutex<Database>, id: &str, record: &WorktreeRecord) {
    let guard = db.lock().await;
    if let (Ok(Some((host, _, _))), Ok(Some(launch))) =
        (guard.conversation_target(id), guard.conversation_launch(id))
    {
        let _ = guard.save_conversation_launch(
            &host,
            &launch.agent,
            &record.repo,
            launch.model.as_deref(),
        );
    }
}

/// Runs one action. `busy` reports whether an agent is still inside the worktree.
pub(crate) async fn perform(
    db: &Mutex<Database>,
    root: &Path,
    id: &str,
    action: Action,
    gh: &str,
    busy: fn(&WorktreeRecord) -> bool,
) -> Result<Outcome, WorktreeError> {
    let _one_at_a_time = ACTIONS.lock().await;
    let record = find(db, root, id).await?;
    if action != Action::OpenPr && busy(&record) {
        return Err(WorktreeError::InUse);
    }
    let outcome = match action {
        Action::Merge => {
            let merged = wt::merge(root, &record).await?;
            restore_launch_repo(db, id, &record).await;
            let mut message = match &merged.commit {
                Some(_) => format!(
                    "Merged {} into {} and removed the worktree.",
                    record.branch, merged.base_branch
                ),
                None => "No changes were made in this worktree; it was removed.".to_owned(),
            };
            if let Some(warning) = merged.cleanup_warning {
                message.push_str(&format!(" {warning}"));
            }
            Outcome {
                message,
                merged_commit: merged.commit,
                ..Outcome::default()
            }
        }
        Action::Discard => {
            wt::discard(root, &record).await?;
            restore_launch_repo(db, id, &record).await;
            Outcome {
                message: format!("Discarded {} and removed the worktree.", record.branch),
                ..Outcome::default()
            }
        }
        Action::OpenPr => {
            let url = wt::open_pr(&record, gh).await?;
            Outcome {
                message: "Pull request ready:".to_owned(),
                url: Some(url),
                ..Outcome::default()
            }
        }
    };
    Ok(outcome)
}

async fn http_action(
    headers: HeaderMap,
    state: AppState,
    id: String,
    action: Action,
) -> Result<Response, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    Ok(
        match perform(
            &state.db,
            &wt::managed_root(),
            &id,
            action,
            "gh",
            wt::in_use,
        )
        .await
        {
            Ok(outcome) => Json(outcome.body()).into_response(),
            Err(error) => (
                error.status(),
                Json(json!({"ok":false,"message":error.to_string()})),
            )
                .into_response(),
        },
    )
}

/// `POST /api/v1/sessions/{id}/worktree/merge`.
pub(crate) async fn merge_handler(
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
    State(state): State<AppState>,
) -> Result<Response, StatusCode> {
    http_action(headers, state, id, Action::Merge).await
}

/// `POST /api/v1/sessions/{id}/worktree/discard`.
pub(crate) async fn discard_handler(
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
    State(state): State<AppState>,
) -> Result<Response, StatusCode> {
    http_action(headers, state, id, Action::Discard).await
}

/// `POST /api/v1/sessions/{id}/worktree/open-pr`.
pub(crate) async fn open_pr_handler(
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
    State(state): State<AppState>,
) -> Result<Response, StatusCode> {
    http_action(headers, state, id, Action::OpenPr).await
}

/// E2EE Remote worktree requests and the `session_changes` alias.
pub(crate) async fn remote_command(db: &Arc<Mutex<Database>>, request: &Value) -> Value {
    let kind = request
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let Some(id) = request.get("session_id").and_then(Value::as_str) else {
        return json!({"type":"error","message":"Choose a session first."});
    };
    let action = match kind {
        "worktree_merge" => Action::Merge,
        "worktree_discard" => Action::Discard,
        "worktree_open_pr" => Action::OpenPr,
        "session_changes" => return session_changes(db, request, id).await,
        _ => return json!({"type":"error","message":"Unknown worktree request."}),
    };
    match perform(db, &wt::managed_root(), id, action, "gh", wt::in_use).await {
        Ok(outcome) => {
            let mut reply = outcome.body();
            reply["type"] = json!("worktree_action_ack");
            reply["session_id"] = json!(id);
            reply["action"] = json!(action.name());
            reply
        }
        Err(error) => json!({"type":"error","message":error.to_string()}),
    }
}

/// `session_changes` is the name the worktree review UI uses for the `changes` request.
async fn session_changes(db: &Arc<Mutex<Database>>, request: &Value, id: &str) -> Value {
    let mut reply = crate::session_changes::remote_command(db, request).await;
    if reply["type"] == "changes" {
        reply["type"] = json!("session_changes");
        if let Ok(record) = find(db, &wt::managed_root(), id).await {
            reply["branch"] = json!(record.branch);
        }
    }
    reply
}

/// Creates the worktree for a launch, or `None` when the request did not ask for one.
pub(crate) async fn prepare_launch(
    repo: &Path,
    session_id: &str,
    wanted: bool,
    branch: Option<&str>,
) -> Result<Option<WorktreeRecord>, WorktreeError> {
    if !wanted {
        return Ok(None);
    }
    wt::create(&wt::managed_root(), repo, session_id, branch)
        .await
        .map(Some)
}

/// Removes a worktree whose agent failed to start.
pub(crate) async fn abort_launch(record: &WorktreeRecord) {
    wt::abort_launch(&wt::managed_root(), record).await;
}

/// Worktree fields merged into launch replies and session rows.
pub(crate) fn fields(record: &WorktreeRecord) -> Value {
    json!({
        "worktree": true,
        "worktree_branch": record.branch,
        "worktree_path": gnomcode_platform::workspaces::display_path(Path::new(&record.path)),
    })
}

/// Stored worktree records by session id, for decorating session lists.
pub(crate) fn index() -> std::collections::HashMap<String, WorktreeRecord> {
    wt::record_index(&wt::managed_root())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sh(dir: &Path, args: &[&str]) -> String {
        let out = std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .expect("git");
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_owned()
    }

    fn repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("repo");
        sh(dir.path(), &["init", "-q", "-b", "main"]);
        sh(dir.path(), &["config", "user.email", "t@example.com"]);
        sh(dir.path(), &["config", "user.name", "Test"]);
        sh(dir.path(), &["config", "commit.gpgsign", "false"]);
        std::fs::write(dir.path().join("a.txt"), "one\n").expect("a");
        sh(dir.path(), &["add", "-A"]);
        sh(dir.path(), &["commit", "-q", "-m", "init"]);
        dir
    }

    fn db() -> Arc<Mutex<Database>> {
        Arc::new(Mutex::new(Database::open_in_memory().expect("database")))
    }

    #[tokio::test]
    async fn actions_refuse_while_an_agent_is_inside_and_unknown_sessions_are_404() {
        let repo = repo();
        let root = tempfile::tempdir().expect("root");
        let db = db();
        let record = wt::create(root.path(), repo.path(), "claude-9", None)
            .await
            .expect("create");
        std::fs::write(Path::new(&record.path).join("w.txt"), "x").expect("w");
        for action in [Action::Merge, Action::Discard] {
            let error = perform(&db, root.path(), "claude-9", action, "gh", |_| true)
                .await
                .expect_err("busy");
            assert!(matches!(error, WorktreeError::InUse));
        }
        assert!(Path::new(&record.path).is_dir(), "nothing was removed");
        let missing = perform(&db, root.path(), "nope", Action::Merge, "gh", |_| false).await;
        assert!(matches!(missing, Err(WorktreeError::NotFound)));
        assert_eq!(WorktreeError::NotFound.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn merge_reports_the_commit_and_discard_reports_the_branch() {
        let repo = repo();
        let root = tempfile::tempdir().expect("root");
        let db = db();
        let one = wt::create(root.path(), repo.path(), "codex-9", None)
            .await
            .expect("create");
        std::fs::write(Path::new(&one.path).join("w.txt"), "x").expect("w");
        let merged = perform(&db, root.path(), "codex-9", Action::Merge, "gh", |_| false)
            .await
            .expect("merge");
        assert!(merged.message.contains("Merged gnomcode/codex-9 into main"));
        assert_eq!(
            merged.merged_commit,
            Some(sh(repo.path(), &["rev-parse", "HEAD"]))
        );
        assert_eq!(merged.body()["ok"], true);
        let two = wt::create(root.path(), repo.path(), "codex-10", None)
            .await
            .expect("create");
        let discarded = perform(&db, root.path(), "codex-10", Action::Discard, "gh", |_| {
            false
        })
        .await
        .expect("discard");
        assert!(discarded.message.contains(&two.branch));
        assert!(!Path::new(&two.path).exists());
    }

    #[tokio::test]
    async fn remote_replies_use_the_ack_shape_the_phone_expects() {
        let db = db();
        let reply = remote_command(&db, &json!({"type":"worktree_merge"})).await;
        assert_eq!(reply["type"], "error");
        let reply =
            remote_command(&db, &json!({"type":"worktree_discard","session_id":"nope"})).await;
        assert_eq!(reply["type"], "error");
        assert!(
            reply["message"]
                .as_str()
                .is_some_and(|m| m.contains("no worktree"))
        );
        let reply =
            remote_command(&db, &json!({"type":"worktree_open_pr","session_id":"nope"})).await;
        assert_eq!(reply["type"], "error");
    }

    #[tokio::test]
    async fn http_requires_auth() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt as _;
        let app = crate::api::build_router(AppState {
            db: db(),
            bearer_token: "wt-token".into(),
            orchestrator: None,
            remote: Arc::new(crate::remote::RemoteManager::new()),
        });
        for action in ["merge", "discard", "open-pr"] {
            let request = Request::builder()
                .method("POST")
                .uri(format!("/api/v1/sessions/s1/worktree/{action}"))
                .header("Host", "127.0.0.1")
                .body(Body::empty())
                .expect("request");
            let response = app.clone().oneshot(request).await.expect("response");
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{action}");
        }
        let request = Request::builder()
            .method("POST")
            .uri("/api/v1/sessions/s1/worktree/merge")
            .header("Host", "127.0.0.1")
            .header("Authorization", "Bearer wt-token")
            .body(Body::empty())
            .expect("request");
        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn launch_with_worktree_refuses_non_git_folders_without_starting_an_agent() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt as _;
        let plain = tempfile::tempdir().expect("plain");
        let body = json!({"agent":"claude","repo_path":plain.path(),"worktree":true}).to_string();
        let app = crate::api::build_router(AppState {
            db: db(),
            bearer_token: "wt-token".into(),
            orchestrator: None,
            remote: Arc::new(crate::remote::RemoteManager::new()),
        });
        let request = Request::builder()
            .method("POST")
            .uri("/api/v1/sessions/launch")
            .header("Host", "127.0.0.1")
            .header("Authorization", "Bearer wt-token")
            .header("Content-Type", "application/json")
            .body(Body::from(body))
            .expect("request");
        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let bytes = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .expect("body");
        let reply: Value = serde_json::from_slice(&bytes).expect("json");
        assert_eq!(reply["ok"], false);
        assert!(
            reply["message"]
                .as_str()
                .is_some_and(|m| m.contains("not inside a git repository"))
        );
        let reply = crate::remote::dispatch(
            &db(),
            json!({"type":"launch","agent":"claude","repo_path":plain.path(),"worktree":true}),
        )
        .await;
        assert_eq!(reply["type"], "error");
        assert!(
            reply["message"]
                .as_str()
                .is_some_and(|m| m.contains("not inside a git repository"))
        );
    }

    #[test]
    fn launch_fields_expose_branch_and_display_path() {
        let record = WorktreeRecord {
            session_id: "s".into(),
            repo: "r".into(),
            path: "p".into(),
            cwd: "p".into(),
            branch: "gnomcode/s".into(),
            base_branch: "main".into(),
            base_commit: "abc".into(),
            created_at: String::new(),
        };
        let fields = fields(&record);
        assert_eq!(fields["worktree"], true);
        assert_eq!(fields["worktree_branch"], "gnomcode/s");
        assert_eq!(fields["worktree_path"], "p");
    }
}
