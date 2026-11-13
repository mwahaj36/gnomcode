//! Conversation listing and chat history handlers.

use super::*;

/// Query parameters for retrieving chat events.
#[derive(Debug, Deserialize)]
pub struct ChatQuery {
    /// Maximum number of events to return.
    pub limit: Option<usize>,
    /// Return events older than this event ID.
    pub before: Option<String>,
}

pub(super) async fn chat_get_handler(
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(query): Query<ChatQuery>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let db = state.db.lock().await;
    let sessions = db.list_active_sessions(usize::MAX).unwrap_or_default();
    let requested = id.clone();
    let id = crate::remote::conversation_session_id(
        &id,
        &sessions,
        &gnomcode_pty::list_active_sessions(),
    );
    match crate::remote::conversation_chat(
        &db,
        &requested,
        &id,
        &sessions,
        query.limit,
        query.before.as_deref(),
    ) {
        Ok(events) => Ok(Json(gnomcode_adapters::codex::clean_stored_conversation(
            gnomcode_adapters::antigravity::clean_stored_conversation(events),
        ))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub(super) async fn chat_delete_handler(
    headers: HeaderMap,
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let _control = crate::CONVERSATION_CONTROL.lock().await;
    let db = state.db.lock().await;
    let sessions = db.list_active_sessions(usize::MAX).unwrap_or_default();
    let hosted = gnomcode_pty::list_active_sessions();
    let native = crate::remote::active_conversation_id(&db, &id, &sessions, &hosted)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let host = sessions
        .iter()
        .find(|session| session.id == native)
        .and_then(|session| crate::remote::session_host(session, &sessions, &hosted))
        .map_or(id.as_str(), |record| record.session_id.as_str());
    match db.delete_session_chat_with_host(&native, host) {
        Ok(deleted_count) => Ok(Json(json!({
            "ok": true,
            "session_id": id,
            "deleted_count": deleted_count
        }))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub(super) async fn list_conversations_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(params): Query<SessionQueryParams>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let db = state.db.lock().await;
    let sessions = db
        .list_active_sessions(usize::MAX)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut rows =
        crate::remote::conversation_sessions(&db, &sessions, &gnomcode_pty::list_active_sessions())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    crate::remote::nest_subagents(&mut rows);
    let page = crate::remote::conversation_page(
        rows,
        &json!({"page":params.page,"limit":params.limit,"agent":params.agent,"status":params.status,"project":params.project,"search":params.search,"source":params.source,"include_subagents":params.include_subagents}),
    );
    if params.page.is_none() {
        Ok(Json(page["sessions"].clone()))
    } else {
        Ok(Json(page))
    }
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
    async fn chat_endpoints_retrieve_and_hard_delete_history() {
        let db = Database::open_in_memory().expect("database");
        let project = gnomcode_core::Project::from_origin(None, "C:\\repo", None);
        let session = gnomcode_core::Session {
            id: "chat-test-session".to_owned(),
            agent: gnomcode_core::AgentId::ClaudeCode,
            surface: gnomcode_core::Surface::Cli,
            project_id: project.id.clone(),
            parent_session_id: None,
            model: Some("claude-3-5-sonnet".to_owned()),
            title: None,
            state: gnomcode_core::SessionState::Idle,
            pid: Some(100),
            started_at: chrono::Utc::now(),
            last_event_at: chrono::Utc::now(),
        };
        let cp = gnomcode_store::SourceCheckpoint {
            file_path: "/path/to/c.jsonl".to_owned(),
            last_read_offset: 0,
            file_size: 0,
            fingerprint: "fp".to_owned(),
            last_ingested_at: chrono::Utc::now(),
        };
        let mut db_mut = db;
        db_mut
            .commit_batch(&[project], &[session], &[], &[], &[], &cp)
            .expect("commit");

        let evt = gnomcode_core::ChatEvent {
            event_id: "evt-test-1".to_owned(),
            session_id: "chat-test-session".to_owned(),
            turn_index: 1,
            role: gnomcode_core::ChatRole::User,
            content: "Please inspect tests".to_owned(),
            tool_name: None,
            status: None,
            created_at: chrono::Utc::now(),
        };
        db_mut.insert_chat_event(&evt).expect("insert chat");

        let app = build_router(AppState {
            db: Arc::new(Mutex::new(db_mut)),
            bearer_token: "chat-token".to_owned(),
            orchestrator: None,
            remote: Arc::new(crate::remote::RemoteManager::new()),
        });

        let titles = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/sessions?range=all")
                    .header("Host", "127.0.0.1")
                    .header("Authorization", "Bearer chat-token")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        let titles: serde_json::Value = serde_json::from_slice(
            &axum::body::to_bytes(titles.into_body(), 131072)
                .await
                .expect("body"),
        )
        .expect("json");
        assert_eq!(titles["sessions"][0]["title"], "Please inspect tests");

        // 1. GET chat events
        let req_get = Request::builder()
            .uri("/api/v1/sessions/chat-test-session/chat")
            .header("Host", "127.0.0.1")
            .header("Authorization", "Bearer chat-token")
            .body(Body::empty())
            .expect("request");

        let res_get = app.clone().oneshot(req_get).await.expect("res");
        assert_eq!(res_get.status(), StatusCode::OK);
        let body = axum::body::to_bytes(res_get.into_body(), 16384)
            .await
            .expect("body");
        let events: Vec<gnomcode_core::ChatEvent> = serde_json::from_slice(&body).expect("json");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].content, "Please inspect tests");

        // 2. DELETE chat events (hard deletion)
        let req_del = Request::builder()
            .method("DELETE")
            .uri("/api/v1/sessions/chat-test-session/chat")
            .header("Host", "127.0.0.1")
            .header("Authorization", "Bearer chat-token")
            .body(Body::empty())
            .expect("request");

        let res_del = app.clone().oneshot(req_del).await.expect("res");
        assert_eq!(res_del.status(), StatusCode::OK);

        // 3. GET chat events after delete -> empty
        let req_get_after = Request::builder()
            .uri("/api/v1/sessions/chat-test-session/chat")
            .header("Host", "127.0.0.1")
            .header("Authorization", "Bearer chat-token")
            .body(Body::empty())
            .expect("request");

        let res_get_after = app.oneshot(req_get_after).await.expect("res");
        assert_eq!(res_get_after.status(), StatusCode::OK);
        let body_after = axum::body::to_bytes(res_get_after.into_body(), 16384)
            .await
            .expect("body");
        let events_after: Vec<gnomcode_core::ChatEvent> =
            serde_json::from_slice(&body_after).expect("json");
        assert!(events_after.is_empty());
    }
}
