//! Profile and notification settings handlers.

use super::*;

pub(super) async fn user_get_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let db = state.db.lock().await;
    let profile = db
        .get_user_profile()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(profile))
}

pub(super) async fn user_post_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(profile): Json<UserProfile>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let db = state.db.lock().await;
    db.update_user_profile(&profile)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(profile))
}

pub(super) async fn notification_settings_get_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let db = state.db.lock().await;
    let settings = db
        .get_notification_settings()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(settings))
}

pub(super) async fn notification_settings_post_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(settings): Json<gnomcode_core::notifications::NotificationSettings>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let settings = settings.sanitized();
    let db = state.db.lock().await;
    db.save_notification_settings(&settings)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(settings))
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
    async fn notification_settings_api_requires_auth_and_persists_changes() {
        let app = build_router(AppState {
            db: Arc::new(Mutex::new(Database::open_in_memory().expect("database"))),
            bearer_token: "notify-token".to_owned(),
            orchestrator: None,
            remote: Arc::new(crate::remote::RemoteManager::new()),
        });

        let unauth = Request::builder()
            .uri("/api/v1/settings/notifications")
            .header("Host", "127.0.0.1")
            .body(Body::empty())
            .expect("request");
        assert_eq!(
            app.clone().oneshot(unauth).await.expect("res").status(),
            StatusCode::UNAUTHORIZED
        );

        let update = Request::builder()
            .method("POST")
            .uri("/api/v1/settings/notifications")
            .header("Host", "127.0.0.1")
            .header("Authorization", "Bearer notify-token")
            .header("Content-Type", "application/json")
            .body(Body::from(
                r#"{"approval_waiting":false,"turn_finished":true,"turn_finished_min_minutes":7,"limit_low":false,"limit_reset":true}"#,
            ))
            .expect("request");
        let save_res = app.clone().oneshot(update).await.expect("save res");
        assert_eq!(save_res.status(), StatusCode::OK);

        let get_req = Request::builder()
            .uri("/api/v1/settings/notifications")
            .header("Host", "127.0.0.1")
            .header("Authorization", "Bearer notify-token")
            .body(Body::empty())
            .expect("request");
        let get_res = app.oneshot(get_req).await.expect("get res");
        assert_eq!(get_res.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(get_res.into_body(), 4096)
            .await
            .expect("body");
        let parsed: gnomcode_core::notifications::NotificationSettings =
            serde_json::from_slice(&bytes).expect("json");
        assert!(!parsed.approval_waiting);
        assert!(parsed.turn_finished);
        assert_eq!(parsed.turn_finished_min_minutes, 7);
        assert!(!parsed.limit_low);
        assert!(parsed.limit_reset);
    }
}
