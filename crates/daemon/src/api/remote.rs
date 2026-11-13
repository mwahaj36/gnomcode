//! Remote device, status and revoke handlers.

use super::*;

pub(super) async fn remote_devices_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<Value>, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    Ok(Json(state.remote.devices.view().await))
}

pub(super) async fn remote_device_action_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    if payload["type"] == "enable_account_access" && payload["remote_access_confirmed"] != true {
        return Err(StatusCode::PRECONDITION_REQUIRED);
    }
    let client = reqwest::Client::builder()
        .https_only(true)
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let auth = crate::sync::read_cloud_auth(&client, &session_file_path())
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    match payload["type"].as_str() {
        Some("enable_account_access") => {
            state
                .remote
                .devices
                .set_account_access(&client, &auth, true)
                .await
                .map_err(|_| StatusCode::BAD_GATEWAY)?;
        }
        Some("disable_account_access") => {
            state
                .remote
                .devices
                .set_account_access(&client, &auth, false)
                .await
                .map_err(|_| StatusCode::BAD_GATEWAY)?;
        }
        Some("remove_device") => {
            let device_id = payload["device_id"]
                .as_str()
                .filter(|id| crate::account_devices::is_hex(id, 32))
                .ok_or(StatusCode::BAD_REQUEST)?;
            state
                .remote
                .devices
                .remove_device(&client, &auth, device_id)
                .await
                .map_err(|_| StatusCode::BAD_GATEWAY)?;
        }
        _ => return Err(StatusCode::BAD_REQUEST),
    }
    Ok(Json(state.remote.devices.view().await))
}

pub(super) async fn remote_status_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    Ok(Json(json!(state.remote.status().await)))
}

pub(super) async fn remote_revoke_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    state
        .remote
        .devices
        .disconnect()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let revoked = state
        .remote
        .revoke()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({"ok": true, "revoked": revoked})))
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
    async fn native_link_requires_account_wide_consent_before_auth_or_persistence() {
        let app = build_router(AppState {
            db: Arc::new(Mutex::new(Database::open_in_memory().expect("database"))),
            bearer_token: "consent-test".into(),
            orchestrator: None,
            remote: Arc::new(crate::remote::RemoteManager::new()),
        });
        for path in [
            "/api/v1/auth/signin",
            "/api/v1/auth/session",
            "/api/v1/remote/devices",
        ] {
            let request = Request::builder()
                .method("POST")
                .uri(path)
                .header("Host", "127.0.0.1")
                .header("Authorization", "Bearer consent-test")
                .header("Content-Type", "application/json")
                .body(Body::from(if path.ends_with("devices") {
                    r#"{"type":"enable_account_access"}"#
                } else {
                    "{}"
                }))
                .expect("request");
            assert_eq!(
                app.clone()
                    .oneshot(request)
                    .await
                    .expect("response")
                    .status(),
                StatusCode::PRECONDITION_REQUIRED,
                "linking must require explicit account-wide access consent before any network/write: {path}"
            );
        }
    }

    #[tokio::test]
    async fn retired_browser_pairing_endpoint_cannot_start_a_collector() {
        let db = Database::open_in_memory().expect("database");
        let app = build_router(AppState {
            db: Arc::new(Mutex::new(db)),
            bearer_token: "local-test-token".to_owned(),
            orchestrator: None,
            remote: Arc::new(crate::remote::RemoteManager::new()),
        });
        for path in [
            "/api/v1/auth/pair",
            "/api/v1/remote/pairing",
            "/api/v1/remote/confirm",
        ] {
            for method in ["GET", "POST", "DELETE"] {
                let request = Request::builder()
                    .method(method)
                    .uri(path)
                    .header("Host", "127.0.0.1:41738")
                    .header("Authorization", "Bearer local-test-token")
                    .body(Body::empty())
                    .expect("request");
                assert_eq!(
                    app.clone()
                        .oneshot(request)
                        .await
                        .expect("response")
                        .status(),
                    StatusCode::NOT_FOUND
                );
            }
        }
    }

    #[tokio::test]
    async fn remote_pairing_status_and_revoke_endpoints() {
        if std::env::var_os("GNOMCODE_TEST_ISOLATED_REMOTE").is_none() {
            let home = tempfile::tempdir().expect("isolated home");
            let output =
                std::process::Command::new(std::env::current_exe().expect("test executable"))
                    .args([
                        "--exact",
                        "api::tests::remote_pairing_status_and_revoke_endpoints",
                    ])
                    .env("GNOMCODE_HOME", home.path())
                    .env("GNOMCODE_TEST_ISOLATED_REMOTE", "1")
                    .output()
                    .expect("isolated test");
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stdout)
            );
            return;
        }
        let app = build_router(AppState {
            db: Arc::new(Mutex::new(Database::open_in_memory().expect("database"))),
            bearer_token: "remote-token".to_owned(),
            orchestrator: None,
            remote: Arc::new(crate::remote::RemoteManager::new()),
        });

        // 1. GET /api/v1/remote/pairing
        let req_pair = Request::builder()
            .uri("/api/v1/remote/pairing")
            .header("Host", "127.0.0.1")
            .header("Authorization", "Bearer remote-token")
            .body(Body::empty())
            .expect("request");
        let res_pair = app.clone().oneshot(req_pair).await.expect("res");
        assert_ne!(res_pair.status(), StatusCode::OK);

        // 2. GET /api/v1/remote/status
        let req_stat = Request::builder()
            .uri("/api/v1/remote/status")
            .header("Host", "127.0.0.1")
            .header("Authorization", "Bearer remote-token")
            .body(Body::empty())
            .expect("request");
        let res_stat = app.clone().oneshot(req_stat).await.expect("res");
        assert_eq!(res_stat.status(), StatusCode::OK);

        // 3. POST /api/v1/remote/revoke
        let req_rev = Request::builder()
            .method("POST")
            .uri("/api/v1/remote/revoke")
            .header("Host", "127.0.0.1")
            .header("Authorization", "Bearer remote-token")
            .body(Body::empty())
            .expect("request");
        let res_rev = app.oneshot(req_rev).await.expect("res");
        assert_eq!(res_rev.status(), StatusCode::OK);
    }
}
