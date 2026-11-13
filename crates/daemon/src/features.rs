//! Off-by-default feature toggles API and Remote RPC handlers (E3, ADR 0049).

use crate::api::{AppState, validate_auth};
use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use gnomcode_core::FeatureToggles;
use gnomcode_store::Database;
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Returns the current E3 feature toggles.
///
/// # Errors
/// Returns `StatusCode::UNAUTHORIZED` on invalid auth or `StatusCode::INTERNAL_SERVER_ERROR` on DB failure.
pub(crate) async fn get_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let db = Arc::clone(&state.db);
    let toggles = tokio::task::spawn_blocking(move || db.blocking_lock().get_feature_toggles())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(toggles))
}

/// Persists updated E3 feature toggles.
///
/// # Errors
/// Returns `StatusCode::UNAUTHORIZED` on invalid auth or `StatusCode::INTERNAL_SERVER_ERROR` on DB failure.
pub(crate) async fn post_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(toggles): Json<FeatureToggles>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let db = Arc::clone(&state.db);
    tokio::task::spawn_blocking(move || db.blocking_lock().set_feature_toggles(&toggles))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(toggles))
}

/// Handles E2EE Remote `features.get` and `features.set` requests.
pub(crate) async fn handle_remote(db: &Arc<Mutex<Database>>, request: &Value) -> Value {
    match request.get("type").and_then(Value::as_str) {
        Some("features.get") => match db.lock().await.get_feature_toggles() {
            Ok(toggles) => json!({ "type": "features", "features": toggles }),
            Err(err) => {
                json!({ "type": "error", "message": format!("Failed to load feature toggles: {err}") })
            }
        },
        Some("features.set") => {
            let Some(raw) = request.get("features") else {
                return json!({ "type": "error", "message": "Missing features payload" });
            };
            let Ok(toggles) = serde_json::from_value::<FeatureToggles>(raw.clone()) else {
                return json!({ "type": "error", "message": "Invalid features payload" });
            };
            match db.lock().await.set_feature_toggles(&toggles) {
                Ok(()) => json!({ "type": "features", "features": toggles }),
                Err(err) => {
                    json!({ "type": "error", "message": format!("Failed to save feature toggles: {err}") })
                }
            }
        }
        _ => json!({ "type": "error", "message": "Unsupported feature toggle request" }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{AppState, build_router};
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use tempfile::tempdir;
    use tower::ServiceExt;

    #[tokio::test]
    async fn features_http_and_remote_default_false_and_roundtrip() {
        let dir = tempdir().expect("tempdir");
        let db = Arc::new(Mutex::new(
            Database::open(&dir.path().join("state.db")).expect("open db"),
        ));

        let initial = handle_remote(&db, &json!({ "type": "features.get" })).await;
        assert_eq!(initial["type"], "features");
        assert_eq!(initial["features"]["teams_enabled"], false);
        assert_eq!(initial["features"]["extra_adapters_enabled"], false);
        assert_eq!(initial["features"]["scheduled_messages_enabled"], false);
        assert_eq!(initial["features"]["account_sync_enabled"], false);

        let updated = FeatureToggles {
            teams_enabled: false,
            extra_adapters_enabled: true,
            scheduled_messages_enabled: false,
            account_sync_enabled: true,
        };
        let remote_set =
            handle_remote(&db, &json!({ "type": "features.set", "features": updated })).await;
        assert_eq!(remote_set["features"]["extra_adapters_enabled"], true);
        assert_eq!(remote_set["features"]["account_sync_enabled"], true);

        let app = build_router(AppState {
            db: Arc::clone(&db),
            bearer_token: "test-token".to_owned(),
            orchestrator: None,
            remote: Arc::new(crate::remote::RemoteManager::new()),
        });
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/settings/features")
                    .header("authorization", "Bearer test-token")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = to_bytes(resp.into_body(), 8192).await.expect("body");
        let parsed: FeatureToggles = serde_json::from_slice(&bytes).expect("json");
        assert_eq!(parsed, updated);
    }
}
