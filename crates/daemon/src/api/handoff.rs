//! Model switch, continue and legacy resume handlers.

use super::*;

/// Request payload to switch a session to a target model with zero-token handoff.
#[derive(Debug, Deserialize)]
pub struct SwitchSessionRequest {
    /// Target model to switch to.
    pub target_model: String,
    /// Optional agent surface identifier.
    pub agent: Option<String>,
}

pub(super) async fn switch_session_handler(
    headers: HeaderMap,
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(payload): Json<SwitchSessionRequest>,
) -> Result<axum::response::Response, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let _control = crate::CONVERSATION_CONTROL.lock().await;
    let prepared = crate::prepare_model_change(
        &state.db,
        &id,
        &payload.target_model,
        payload.agent.as_deref(),
    )
    .await;
    let prepared = match prepared {
        Ok(value) => value,
        Err(error) => return Ok(handoff_error_response(error)),
    };
    let record = match crate::execute_switch(prepared).await {
        Ok(value) => value,
        Err(error) => return Ok(handoff_error_response(error)),
    };
    crate::remote::persist_model_change(&state.db, &id, &record)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({
        "ok": true,
        "source_session_id": id,
        "session_id": id,
        "agent": record.agent,
        "model": record.model
    }))
    .into_response())
}

pub(super) fn handoff_error_response(error: crate::HandoffError) -> axum::response::Response {
    let (status, message) = match error {
        crate::HandoffError::ImportedSession => (
            StatusCode::FORBIDDEN,
            "This provider session is view-only. Start a Gnomcode conversation to send messages or switch models.",
        ),
        crate::HandoffError::Busy => (
            StatusCode::CONFLICT,
            "Wait for the current response or approval to finish before sending or changing models.",
        ),
        crate::HandoffError::NotReady => (
            StatusCode::CONFLICT,
            "The agent is starting or needs native approval. Open Agent terminal, finish setup, then retry. Your draft is preserved.",
        ),
        crate::HandoffError::MissingSession => (
            StatusCode::NOT_FOUND,
            "The source session or workspace is no longer available. Reload conversations and retry.",
        ),
        crate::HandoffError::UnsupportedTarget => (
            StatusCode::BAD_REQUEST,
            "The selected model is not supported by that agent.",
        ),
        crate::HandoffError::Platform(_) => (
            StatusCode::CONFLICT,
            "The workspace could not be inspected. Check folder access and Git status.",
        ),
        crate::HandoffError::Store(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Conversation metadata could not be read. Reload and retry.",
        ),
        crate::HandoffError::Launch(_) => (
            StatusCode::BAD_GATEWAY,
            "The target agent could not start or accept the handoff. Check its installation and desktop login.",
        ),
    };
    (status, Json(json!({"message":message}))).into_response()
}

/// Request payload to resume an idle or suspended session.
pub(super) async fn continue_session_handler(
    headers: HeaderMap,
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<axum::response::Response, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    Ok(match crate::resume_conversation(&state.db, &id).await {
        Ok(_) => Json(json!({"ok":true,"session_id":id})).into_response(),
        Err(error) => handoff_error_response(error),
    })
}

/// Request payload to resume an idle or suspended session.
#[derive(Debug, Deserialize, Default)]
pub struct ResumeSessionRequest {
    /// Optional resume prompt instruction.
    pub prompt: Option<String>,
    /// Optional maximum run count.
    pub max_runs: Option<u32>,
}

pub(super) async fn resume_session_handler(
    headers: HeaderMap,
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(payload): Json<ResumeSessionRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    if payload
        .prompt
        .as_deref()
        .is_some_and(|prompt| prompt != "continue")
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    let db = state.db.lock().await;

    let target = db
        .get_session_focus_target(&id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    if db
        .conversation_launch(&id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .is_none()
    {
        return Err(StatusCode::FORBIDDEN);
    }
    drop(target);
    drop(db);
    let reply = crate::automation::command(
        &state.db,
        json!({"type":"resume_on_reset","session_id":id,"enabled":true}),
    )
    .await;
    if reply["type"] == "error" {
        return Err(StatusCode::CONFLICT);
    }
    Ok(Json(reply))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use gnomcode_store::Database;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tower::ServiceExt as _;

    #[tokio::test]
    async fn switch_session_endpoint_compiles_handoff_packet() {
        let app = build_router(AppState {
            db: Arc::new(Mutex::new(Database::open_in_memory().expect("database"))),
            bearer_token: "switch-token".to_owned(),
            orchestrator: None,
            remote: Arc::new(crate::remote::RemoteManager::new()),
        });

        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/sessions/switch-test-s1/switch")
            .header("Host", "127.0.0.1")
            .header("Authorization", "Bearer switch-token")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"target_model":"o3"}"#))
            .expect("request");

        let res = app.oneshot(req).await.expect("res");
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn resume_session_rejects_unknown_session() {
        let app = build_router(AppState {
            db: Arc::new(Mutex::new(Database::open_in_memory().expect("database"))),
            bearer_token: "resume-token".to_owned(),
            orchestrator: None,
            remote: Arc::new(crate::remote::RemoteManager::new()),
        });

        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/sessions/resume-test-s1/resume")
            .header("Host", "127.0.0.1")
            .header("Authorization", "Bearer resume-token")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"prompt":"continue","max_runs":2}"#))
            .expect("request");

        let res = app.oneshot(req).await.expect("res");
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn imported_session_cannot_be_armed_through_the_legacy_resume_endpoint() {
        use gnomcode_core::{AgentId, Project, Session, SessionState, Surface};
        let mut db = Database::open_in_memory().expect("database");
        let project = Project::from_origin(None, "/workspace/resume", None);
        let session = Session {
            id: "codex:resume-test".to_owned(),
            agent: AgentId::Codex,
            surface: Surface::Cli,
            project_id: project.id.clone(),
            parent_session_id: None,
            model: None,
            title: None,
            state: SessionState::Idle,
            pid: None,
            started_at: chrono::Utc::now(),
            last_event_at: chrono::Utc::now(),
        };
        let checkpoint = gnomcode_store::SourceCheckpoint {
            file_path: "/fixture".to_owned(),
            last_read_offset: 0,
            file_size: 0,
            fingerprint: "unit".to_owned(),
            last_ingested_at: chrono::Utc::now(),
        };
        db.commit_batch(&[project], &[session], &[], &[], &[], &checkpoint)
            .expect("fixture");
        let db = Arc::new(Mutex::new(db));
        let app = build_router(AppState {
            db: db.clone(),
            bearer_token: "resume-token".to_owned(),
            orchestrator: None,
            remote: Arc::new(crate::remote::RemoteManager::new()),
        });
        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/sessions/codex:resume-test/resume")
            .header("Host", "127.0.0.1")
            .header("Authorization", "Bearer resume-token")
            .header("Content-Type", "application/json")
            .body(Body::from("{}"))
            .expect("request");
        assert_eq!(
            app.oneshot(req).await.expect("response").status(),
            StatusCode::FORBIDDEN
        );
        let db = db.lock().await;
        let queue = db.list_resume_queue().expect("queue");
        assert!(queue.is_empty());
    }
}
