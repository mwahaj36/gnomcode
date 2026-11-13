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
