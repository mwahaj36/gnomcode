//! Tool permission decision handler.

use super::*;

/// Request payload to approve or deny a pending tool execution or action.
#[derive(Debug, Deserialize)]
pub struct PermissionRequest {
    /// Decision: `approve` or `deny`.
    pub action: String,
    /// Optional tool call identifier.
    pub tool_call_id: Option<String>,
    /// Optional summary text.
    pub summary: Option<String>,
}

pub(super) async fn permissions_post_handler(
    headers: HeaderMap,
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(payload): Json<PermissionRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let action = payload.action.to_ascii_lowercase();
    if action != "approve" && action != "deny" {
        return Err(StatusCode::BAD_REQUEST);
    }

    let summary = payload
        .summary
        .unwrap_or_else(|| format!("User {}d action", action));
    let redacted_content = gnomcode_core::redact::redact_chat_content(&summary);

    let db = state.db.lock().await;
    let evt = gnomcode_core::ChatEvent {
        event_id: format!(
            "perm-{}-{}",
            id,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ),
        session_id: id.clone(),
        turn_index: 0,
        role: gnomcode_core::ChatRole::System,
        content: redacted_content,
        tool_name: payload.tool_call_id,
        status: Some(action.clone()),
        created_at: chrono::Utc::now(),
    };
    let _ = db.insert_chat_event(&evt);

    Ok(Json(
        json!({ "ok": true, "session_id": id, "action": action }),
    ))
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
    async fn permissions_endpoint_records_approval_decision() {
        let app = build_router(AppState {
            db: Arc::new(Mutex::new(Database::open_in_memory().expect("database"))),
            bearer_token: "perm-token".to_owned(),
            orchestrator: None,
            remote: Arc::new(crate::remote::RemoteManager::new()),
        });

        // 1. Approve
        let req_app = Request::builder()
            .method("POST")
            .uri("/api/v1/sessions/test-s1/permissions")
            .header("Host", "127.0.0.1")
            .header("Authorization", "Bearer perm-token")
            .header("Content-Type", "application/json")
            .body(Body::from(
                r#"{"action":"approve","tool_call_id":"tool-123"}"#,
            ))
            .expect("request");
        let res_app = app.clone().oneshot(req_app).await.expect("res");
        assert_eq!(res_app.status(), StatusCode::OK);

        // 2. Deny
        let req_deny = Request::builder()
            .method("POST")
            .uri("/api/v1/sessions/test-s1/permissions")
            .header("Host", "127.0.0.1")
            .header("Authorization", "Bearer perm-token")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"action":"deny","tool_call_id":"tool-123"}"#))
            .expect("request");
        let res_deny = app.clone().oneshot(req_deny).await.expect("res");
        assert_eq!(res_deny.status(), StatusCode::OK);

        // 3. Invalid action (escalation) -> 400 Bad Request
        let req_bad = Request::builder()
            .method("POST")
            .uri("/api/v1/sessions/test-s1/permissions")
            .header("Host", "127.0.0.1")
            .header("Authorization", "Bearer perm-token")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"action":"escalate_permissions"}"#))
            .expect("request");
        let res_bad = app.oneshot(req_bad).await.expect("res");
        assert_eq!(res_bad.status(), StatusCode::BAD_REQUEST);
    }
}
