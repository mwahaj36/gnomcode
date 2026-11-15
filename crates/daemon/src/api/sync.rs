//! Reconcile and sync-status handlers.

use super::*;

pub(super) async fn reconcile_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let orchestrator = state.orchestrator.ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    match orchestrator
        .reconcile_now()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        true => Ok((StatusCode::ACCEPTED, Json(json!({ "status": "completed" })))),
        false => Ok((
            StatusCode::CONFLICT,
            Json(json!({ "status": "in_progress" })),
        )),
    }
}

pub(super) async fn sync_status_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    Ok(Json(crate::sync::cloud_sync_status().await))
}

pub(super) async fn usage_sync_status_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let db = Arc::clone(&state.db);
    let local_usage_count =
        tokio::task::spawn_blocking(move || db.blocking_lock().local_usage_record_count())
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let status = crate::sync::usage_sync_status().await;
    let mut value = serde_json::to_value(status).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    value["local_usage_count"] = json!(local_usage_count);
    Ok(Json(value))
}

pub(super) async fn usage_sync_restore_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    validate_auth(&headers, &state.bearer_token)
        .map_err(|s| (s, Json(json!({ "ok": false, "message": "Unauthorized." }))))?;
    let summary = crate::sync::restore_usage_history(&state.db)
        .await
        .map_err(|(s, msg)| (s, Json(json!({ "ok": false, "message": msg }))))?;
    Ok(Json(summary))
}

pub(super) async fn usage_sync_delete_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    validate_auth(&headers, &state.bearer_token)
        .map_err(|s| (s, Json(json!({ "ok": false, "message": "Unauthorized." }))))?;
    crate::sync::delete_cloud_usage_history(&state.db)
        .await
        .map_err(|(s, msg)| (s, Json(json!({ "ok": false, "message": msg }))))?;
    Ok(Json(
        json!({ "ok": true, "message": "Cloud usage history deleted." }),
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
    async fn reconcile_rejects_non_loopback_host_before_starting() {
        let db = Database::open_in_memory().expect("in-memory db");
        let app = build_router(AppState {
            db: Arc::new(Mutex::new(db)),
            bearer_token: "secret-test-token".to_owned(),
            orchestrator: None,
            remote: Arc::new(crate::remote::RemoteManager::new()),
        });
        let request = Request::builder()
            .method("POST")
            .uri("/api/v1/reconcile")
            .header("Host", "attacker.example")
            .header("Authorization", "Bearer secret-test-token")
            .body(Body::empty())
            .expect("request");
        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}
