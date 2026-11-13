//! Project and workspace browsing handlers.

use super::*;

pub(super) async fn projects_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(params): Query<ReportQueryParams>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let range = params.range.as_deref().unwrap_or("all");
    if !matches!(range, "all" | "today" | "7d" | "30d" | "custom")
        || (range == "custom" && params.from.is_none())
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    for bound in [params.from.as_ref(), params.to.as_ref()]
        .into_iter()
        .flatten()
    {
        chrono::DateTime::parse_from_rfc3339(bound).map_err(|_| StatusCode::BAD_REQUEST)?;
    }
    let range = range.to_owned();
    let projects = tokio::task::spawn_blocking(move || {
        state.db.blocking_lock().generate_project_report(
            &range,
            params.from.as_deref(),
            params.to.as_deref(),
        )
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(projects))
}

pub(super) async fn workspaces_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(mut request): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    if !request.is_object() {
        return Err(StatusCode::BAD_REQUEST);
    }
    request["type"] = json!("workspaces");
    Ok(Json(crate::remote::dispatch(&state.db, request).await))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use gnomcode_store::Database;
    use serde_json::json;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tower::ServiceExt as _;

    #[tokio::test]
    async fn workspace_browser_requires_auth_and_lists_directories() {
        let dir = tempfile::tempdir().expect("workspace");
        std::fs::create_dir(dir.path().join("project")).expect("directory");
        let app = build_router(AppState {
            db: Arc::new(Mutex::new(Database::open_in_memory().expect("database"))),
            bearer_token: "local-test-token".to_owned(),
            orchestrator: None,
            remote: Arc::new(crate::remote::RemoteManager::new()),
        });
        let body = json!({"path": dir.path().to_string_lossy()}).to_string();
        let request = |auth: &str| {
            Request::builder()
                .method("POST")
                .uri("/api/v1/workspaces")
                .header("host", "127.0.0.1:41738")
                .header("authorization", auth)
                .header("content-type", "application/json")
                .body(Body::from(body.clone()))
                .expect("request")
        };
        assert_eq!(
            app.clone()
                .oneshot(request("Bearer wrong"))
                .await
                .expect("response")
                .status(),
            StatusCode::UNAUTHORIZED
        );
        let response = app
            .oneshot(request("Bearer local-test-token"))
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), 100_000)
            .await
            .expect("body");
        let listing: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
        assert_eq!(listing["folders"][0]["name"], "project");
    }

    #[tokio::test]
    async fn project_filters_and_sync_status_require_valid_local_requests() {
        let app = build_router(AppState {
            db: Arc::new(Mutex::new(Database::open_in_memory().expect("database"))),
            bearer_token: "test-token".to_owned(),
            orchestrator: None,
            remote: Arc::new(crate::remote::RemoteManager::new()),
        });
        for (path, expected) in [
            ("/api/v1/projects?range=all", StatusCode::OK),
            ("/api/v1/projects?range=invalid", StatusCode::BAD_REQUEST),
            ("/api/v1/projects?from=invalid", StatusCode::BAD_REQUEST),
            ("/api/v1/sync/status", StatusCode::OK),
        ] {
            let request = Request::builder()
                .uri(path)
                .header("Host", "127.0.0.1:41738")
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .expect("request");
            assert_eq!(
                app.clone()
                    .oneshot(request)
                    .await
                    .expect("response")
                    .status(),
                expected,
                "{path}"
            );
        }
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/model-catalog")
                    .header("Host", "127.0.0.1:41738")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "model discovery must not launch CLI probes before authentication"
        );
    }
}
