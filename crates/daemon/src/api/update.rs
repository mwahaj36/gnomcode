//! Update status, check and preference handlers.

use super::*;

#[derive(Deserialize)]
pub(super) struct UpdatePreferenceBody {
    enabled: bool,
}

pub(super) async fn update_status_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let enabled = crate::update::is_update_enabled(&state.db).await;
    let mut snapshot = crate::update::shared_state().read().await.clone();
    snapshot.auto_update_enabled = enabled;
    if !enabled {
        snapshot.state = gnomcode_core::update::UpdateStateKind::Disabled;
        snapshot.available_version = None;
        snapshot.downloaded_path = None;
        snapshot.sha256 = None;
    }
    Ok(Json(snapshot))
}

pub(super) async fn update_check_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let status = crate::update::check_now(&state.db).await;
    Ok(Json(status))
}

pub(super) async fn update_settings_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(body): Json<UpdatePreferenceBody>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let db = Arc::clone(&state.db);
    let enabled = body.enabled;
    tokio::task::spawn_blocking(move || db.blocking_lock().set_auto_update_enabled(enabled))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut snapshot = crate::update::shared_state().write().await;
    snapshot.auto_update_enabled = enabled;
    if !enabled {
        snapshot.state = gnomcode_core::update::UpdateStateKind::Disabled;
        snapshot.available_version = None;
        snapshot.downloaded_path = None;
        snapshot.sha256 = None;
    } else if snapshot.state == gnomcode_core::update::UpdateStateKind::Disabled {
        snapshot.state = gnomcode_core::update::UpdateStateKind::UpToDate;
    }
    Ok(Json(snapshot.clone()))
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
    async fn update_status_and_opt_out_endpoints_enforce_auth_and_persist() {
        let app = build_router(AppState {
            db: Arc::new(Mutex::new(Database::open_in_memory().expect("database"))),
            bearer_token: "update-token".to_owned(),
            orchestrator: None,
            remote: Arc::new(crate::remote::RemoteManager::new()),
        });

        let unauth = Request::builder()
            .uri("/api/v1/update/status")
            .header("Host", "127.0.0.1")
            .body(Body::empty())
            .expect("req");
        assert_eq!(
            app.clone().oneshot(unauth).await.expect("res").status(),
            StatusCode::UNAUTHORIZED
        );

        let opt_out = Request::builder()
            .method("POST")
            .uri("/api/v1/settings/update")
            .header("Host", "127.0.0.1")
            .header("Authorization", "Bearer update-token")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"enabled":false}"#))
            .expect("req");
        let save_res = app.clone().oneshot(opt_out).await.expect("res");
        assert_eq!(save_res.status(), StatusCode::OK);

        let status_req = Request::builder()
            .uri("/api/v1/update/status")
            .header("Host", "127.0.0.1")
            .header("Authorization", "Bearer update-token")
            .body(Body::empty())
            .expect("req");
        let status_res = app.oneshot(status_req).await.expect("res");
        assert_eq!(status_res.status(), StatusCode::OK);
        let body = axum::body::to_bytes(status_res.into_body(), 4096)
            .await
            .expect("bytes");
        let parsed: gnomcode_core::update::UpdateStatus =
            serde_json::from_slice(&body).expect("json");
        assert!(!parsed.auto_update_enabled);
        assert_eq!(
            parsed.state,
            gnomcode_core::update::UpdateStateKind::Disabled
        );
    }
}
