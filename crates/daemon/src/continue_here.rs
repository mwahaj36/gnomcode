//! "Continue here" for imported provider sessions (roadmap F2, ADR 0053).
//!
//! An imported session was started outside Gnomcode. Continuing it reopens the same native
//! conversation through the agent's own resume command, in the working folder the session
//! recorded, hosted in a Gnomcode terminal. No permission-skipping flag is ever passed: the
//! command comes from `gnomcode_pty::resume_session`, which only adds `resume <id>` /
//! `--resume <id>`.

use gnomcode_store::Database;
use serde_json::{Value, json};
use std::path::PathBuf;
use thiserror::Error;
use tokio::sync::Mutex;

/// Why a session could not be continued.
#[derive(Debug, Error)]
pub(crate) enum ContinueHereError {
    /// The id is empty, too long or unknown.
    #[error("Session not found. Reload the session list and try again.")]
    NotFound,
    /// The agent has no supported native resume command.
    // Kept for agent names stored by older builds.
    #[error("{0} sessions cannot be resumed from Gnomcode yet. Start a new conversation instead.")]
    UnsupportedAgent(String),
    /// The session was started on a non-CLI surface (e.g. Antigravity 2.0 IDE or desktop app).
    #[error(
        "Sessions started in {0} cannot be resumed from Gnomcode. Start a new conversation here instead."
    )]
    UnsupportedSurface(String),
    /// The session never recorded a working folder, or the folder is gone.
    #[error(
        "This session has no recorded working folder on this computer, so Gnomcode cannot resume it safely. Start a new conversation in the project instead."
    )]
    NoFolder,
    /// The native resume id is missing or not shaped like the agent's ids.
    #[error(
        "This session has no usable native resume id, so Gnomcode cannot resume it. Start a new conversation instead."
    )]
    NoResumeId,
    /// The store could not be read or written.
    #[error("Conversation metadata could not be saved. Reload and retry.")]
    Store,
    /// The agent terminal could not start.
    #[error(
        "The agent could not start. Check that it is installed and signed in on this computer."
    )]
    Launch,
}

impl ContinueHereError {
    pub(crate) fn status(&self) -> axum::http::StatusCode {
        use axum::http::StatusCode;
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::UnsupportedAgent(_)
            | Self::UnsupportedSurface(_)
            | Self::NoFolder
            | Self::NoResumeId => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Store => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Launch => StatusCode::BAD_GATEWAY,
        }
    }
}

/// Everything needed to reopen one imported session.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ResumePlan {
    pub(crate) agent: String,
    pub(crate) folder: PathBuf,
    /// Agent-native id without the `<agent>:` prefix.
    pub(crate) native_id: String,
}

/// Validates an imported session and returns how to resume it. Pure: starts nothing.
pub(crate) fn plan(db: &Database, id: &str) -> Result<ResumePlan, ContinueHereError> {
    if id.is_empty() || id.len() > 128 {
        return Err(ContinueHereError::NotFound);
    }
    let target = db
        .get_session_focus_target(id)
        .map_err(|_| ContinueHereError::Store)?
        .ok_or(ContinueHereError::NotFound)?;
    if !matches!(
        target.agent.as_str(),
        "codex" | "claude-code" | "antigravity"
    ) {
        return Err(ContinueHereError::UnsupportedAgent(target.agent));
    }
    if target.surface != "cli" {
        let label = gnomcode_core::surface_label(&target.agent, &target.surface)
            .unwrap_or(&target.surface)
            .to_owned();
        return Err(ContinueHereError::UnsupportedSurface(label));
    }
    let folder = target
        .repo_root
        .filter(|path| !path.trim().is_empty())
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
        .ok_or(ContinueHereError::NoFolder)?;
    let native_id = id
        .strip_prefix(&format!("{}:", target.agent))
        .filter(|raw| gnomcode_pty::resume_command(&target.agent, &folder, raw).is_ok())
        .ok_or(ContinueHereError::NoResumeId)?
        .to_owned();
    Ok(ResumePlan {
        agent: target.agent,
        folder,
        native_id,
    })
}

/// Records the new hosted terminal as the managed home of the imported conversation.
pub(crate) fn adopt(
    db: &Database,
    id: &str,
    host: &str,
    plan: &ResumePlan,
) -> Result<(), ContinueHereError> {
    db.reattach_conversation(id, host, Some(id))
        .and_then(|()| {
            db.save_conversation_launch(host, &plan.agent, &plan.folder.to_string_lossy(), None)
        })
        .map_err(|_| ContinueHereError::Store)
}

/// Resumes an imported session in its recorded folder and adopts it. Idempotent: a session that
/// is already managed is resumed through the normal path.
pub(crate) async fn continue_here(
    db: &Mutex<Database>,
    id: &str,
) -> Result<Value, ContinueHereError> {
    let _control = crate::CONVERSATION_CONTROL.lock().await;
    let managed = {
        let guard = db.lock().await;
        guard
            .conversation_launch(id)
            .map_err(|_| ContinueHereError::Store)?
            .is_some()
    };
    if managed {
        let record = crate::ensure_conversation_running(db, id)
            .await
            .map_err(|_| ContinueHereError::Launch)?;
        return Ok(ack(id, &record.session_id));
    }
    let plan = plan(&*db.lock().await, id)?;
    let mut random = [0u8; 8];
    getrandom::fill(&mut random).map_err(|_| ContinueHereError::Launch)?;
    let host = format!("resume-{}", u64::from_be_bytes(random));
    let record = gnomcode_pty::resume_session(&host, &plan.agent, &plan.folder, &plan.native_id)
        .await
        .map_err(|_| ContinueHereError::Launch)?;
    adopt(&*db.lock().await, id, &record.session_id, &plan)?;
    Ok(ack(id, &record.session_id))
}

fn ack(id: &str, host: &str) -> Value {
    json!({"ok":true,"session_id":id,"new_session_id":id,"host_session_id":host})
}

/// E2EE Remote `{ "type": "continue_here", "session_id": "<id>" }`.
pub(crate) async fn remote_command(db: &Mutex<Database>, request: &Value) -> Value {
    let Some(id) = request.get("session_id").and_then(Value::as_str) else {
        return json!({"type":"error","message":"Choose a session to continue."});
    };
    match continue_here(db, id).await {
        Ok(mut reply) => {
            reply["type"] = json!("continue_here_ack");
            reply
        }
        Err(error) => json!({"type":"error","message":error.to_string()}),
    }
}

/// `POST /api/v1/sessions/{id}/continue-here`.
pub(crate) async fn post_handler(
    headers: axum::http::HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::extract::State(state): axum::extract::State<crate::api::AppState>,
) -> Result<axum::response::Response, axum::http::StatusCode> {
    use axum::response::IntoResponse;
    crate::api::validate_auth(&headers, &state.bearer_token)?;
    Ok(match continue_here(&state.db, &id).await {
        Ok(reply) => axum::Json(reply).into_response(),
        Err(error) => (
            error.status(),
            axum::Json(json!({"ok":false,"message":error.to_string()})),
        )
            .into_response(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use gnomcode_core::{AgentId, Project, Session, SessionState, Surface};
    use std::path::Path;

    fn db_with_session(id: &str, agent: AgentId, cwd: Option<&Path>) -> Database {
        let mut db = Database::open_in_memory().expect("database");
        let now = chrono::Utc::now();
        let project = Project::from_origin(None, "scratch-project", None);
        let session = Session {
            id: id.into(),
            agent,
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
        db.commit_batch(&[project], &[session], &[], &[], &[], &checkpoint)
            .expect("session");
        if let Some(cwd) = cwd {
            db.record_session_cwd(id, &cwd.to_string_lossy())
                .expect("cwd");
        }
        db
    }

    #[test]
    fn plan_resumes_in_the_recorded_folder_with_the_native_id() {
        let folder = tempfile::tempdir().expect("folder");
        let db = db_with_session("codex:abc-123", AgentId::Codex, Some(folder.path()));
        assert_eq!(
            plan(&db, "codex:abc-123").expect("plan"),
            ResumePlan {
                agent: "codex".into(),
                folder: folder.path().to_path_buf(),
                native_id: "abc-123".into(),
            }
        );
    }

    #[test]
    fn plan_refuses_without_a_recorded_folder_or_a_real_one() {
        let db = db_with_session("codex:abc", AgentId::Codex, None);
        assert!(matches!(
            plan(&db, "codex:abc"),
            Err(ContinueHereError::NoFolder)
        ));
        let gone = std::env::temp_dir().join("gnomcode-continue-here-missing-folder");
        let db = db_with_session("codex:abc", AgentId::Codex, Some(&gone));
        assert!(matches!(
            plan(&db, "codex:abc"),
            Err(ContinueHereError::NoFolder)
        ));
    }

    #[test]
    fn plan_refuses_unusable_resume_ids_and_unknown_sessions() {
        let folder = tempfile::tempdir().expect("folder");
        for bad in ["codex:--yolo", "codex:a b", "codex:", "codex:a/../b"] {
            let db = db_with_session(bad, AgentId::Codex, Some(folder.path()));
            assert!(
                matches!(plan(&db, bad), Err(ContinueHereError::NoResumeId)),
                "{bad}"
            );
        }
        let db = db_with_session("no-prefix", AgentId::Codex, Some(folder.path()));
        assert!(matches!(
            plan(&db, "no-prefix"),
            Err(ContinueHereError::NoResumeId)
        ));
        assert!(matches!(
            plan(&db, "missing"),
            Err(ContinueHereError::NotFound)
        ));
        assert!(matches!(plan(&db, ""), Err(ContinueHereError::NotFound)));
    }

    #[test]
    fn plan_accepts_antigravity_conversations_in_plan_mode() {
        let folder = tempfile::tempdir().expect("folder");
        let db = db_with_session("antigravity:x1", AgentId::Antigravity, Some(folder.path()));
        let plan = plan(&db, "antigravity:x1").expect("plan");
        assert_eq!(plan.native_id, "x1");
        let command = gnomcode_pty::resume_command("antigravity", folder.path(), &plan.native_id)
            .expect("cmd");
        assert!(command.arguments.join(" ").ends_with("--mode plan"));
    }

    #[test]
    fn plan_rejects_non_cli_external_surfaces() {
        use gnomcode_core::{AgentId, Project, Session, SessionState, Surface};
        let folder = tempfile::tempdir().expect("folder");
        let project = Project::from_origin(None, &folder.path().to_string_lossy(), None);
        let now = chrono::Utc::now();
        let session = Session {
            id: "antigravity:ide-1".into(),
            agent: AgentId::Antigravity,
            surface: Surface::Ide,
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
        let mut db = Database::open_in_memory().expect("db");
        db.commit_batch(&[project], &[session], &[], &[], &[], &checkpoint)
            .expect("commit");
        assert!(matches!(
            plan(&db, "antigravity:ide-1"),
            Err(ContinueHereError::UnsupportedSurface(_))
        ));
    }

    #[test]
    fn adopt_makes_the_imported_session_managed_and_stable() {
        let folder = tempfile::tempdir().expect("folder");
        let db = db_with_session("claude-code:u1", AgentId::ClaudeCode, Some(folder.path()));
        let plan = plan(&db, "claude-code:u1").expect("plan");
        assert!(
            db.conversation_launch("claude-code:u1")
                .expect("q")
                .is_none()
        );
        adopt(&db, "claude-code:u1", "resume-1", &plan).expect("adopt");
        let launch = db
            .conversation_launch("claude-code:u1")
            .expect("q")
            .expect("managed");
        assert_eq!(launch.agent, "claude-code");
        assert_eq!(launch.repo, folder.path().to_string_lossy());
        assert_eq!(
            db.conversation_id("claude-code:u1").expect("id"),
            "claude-code:u1"
        );
    }

    #[test]
    fn resume_command_never_carries_permission_skipping_flags() {
        let folder = tempfile::tempdir().expect("folder");
        for agent in ["codex", "claude-code", "antigravity"] {
            let command = gnomcode_pty::resume_command(agent, folder.path(), "abc").expect("cmd");
            let all = command.arguments.join(" ");
            assert!(
                !all.contains("dangerously") && !all.contains("yolo"),
                "{all}"
            );
        }
    }

    #[tokio::test]
    async fn endpoint_requires_auth_and_explains_refusals() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use std::sync::Arc;
        use tower::ServiceExt as _;
        let folder = tempfile::tempdir().expect("folder");
        let db = db_with_session("codex:nofolder", AgentId::Codex, None);
        let app = crate::api::build_router(crate::api::AppState {
            db: Arc::new(Mutex::new(db)),
            bearer_token: "ch-token".into(),
            orchestrator: None,
            remote: Arc::new(crate::remote::RemoteManager::new()),
        });
        let call = |id: &str, auth: bool| {
            let mut request = Request::builder()
                .method("POST")
                .uri(format!("/api/v1/sessions/{id}/continue-here"))
                .header("Host", "127.0.0.1");
            if auth {
                request = request.header("Authorization", "Bearer ch-token");
            }
            request.body(Body::empty()).expect("request")
        };
        let response = app.clone().oneshot(call("codex:nofolder", false)).await;
        assert_eq!(
            response.expect("response").status(),
            StatusCode::UNAUTHORIZED
        );
        let response = app
            .clone()
            .oneshot(call("codex:nofolder", true))
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .expect("body");
        let body: Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(body["ok"], false);
        assert!(
            body["message"]
                .as_str()
                .is_some_and(|m| m.contains("working folder"))
        );
        let response = app.oneshot(call("missing", true)).await.expect("response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        drop(folder);
    }

    #[tokio::test]
    async fn remote_reports_clear_errors() {
        let db = Mutex::new(db_with_session("codex:nofolder", AgentId::Codex, None));
        let reply = remote_command(&db, &json!({"type":"continue_here"})).await;
        assert_eq!(reply["type"], "error");
        let reply = remote_command(
            &db,
            &json!({"type":"continue_here","session_id":"codex:nofolder"}),
        )
        .await;
        assert_eq!(reply["type"], "error");
        assert!(
            reply["message"]
                .as_str()
                .is_some_and(|m| m.contains("working folder"))
        );
    }
}
