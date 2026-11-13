//! Doctor diagnostics handler.

use super::*;

#[derive(Debug, Deserialize, Default)]
pub(super) struct DoctorQueryParams {
    repo: Option<String>,
}

pub(super) async fn doctor_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(params): Query<DoctorQueryParams>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let repo = params
        .repo
        .map(std::path::PathBuf::from)
        .filter(|path| path.is_dir());
    let db_path = gnomcode_platform::default_db_path();
    let report = crate::doctor::run_for_daemon(&state.db, &db_path, repo).await;
    Ok(Json(report))
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
    async fn doctor_endpoint_requires_auth_and_returns_checks() {
        let app = build_router(AppState {
            db: Arc::new(Mutex::new(Database::open_in_memory().expect("database"))),
            bearer_token: "doctor-token".to_owned(),
            orchestrator: None,
            remote: Arc::new(crate::remote::RemoteManager::new()),
        });

        let unauth = Request::builder()
            .uri("/api/v1/doctor")
            .header("Host", "127.0.0.1")
            .body(Body::empty())
            .expect("request");
        let unauth_res = app.clone().oneshot(unauth).await.expect("res");
        assert_eq!(unauth_res.status(), StatusCode::UNAUTHORIZED);

        let authed = Request::builder()
            .uri("/api/v1/doctor")
            .header("Host", "127.0.0.1")
            .header("Authorization", "Bearer doctor-token")
            .body(Body::empty())
            .expect("request");
        let res = app.oneshot(authed).await.expect("res");
        assert_eq!(res.status(), StatusCode::OK);
        let body = axum::body::to_bytes(res.into_body(), 65536)
            .await
            .expect("body");
        let report: crate::doctor::DoctorReport = serde_json::from_slice(&body).expect("json");
        let ids: Vec<&str> = report.checks.iter().map(|c| c.id.as_str()).collect();
        for expected in [
            "agent_clis",
            "claude_signin",
            "limit_sources",
            "claude_integration",
            "daemon",
            "disk_space",
            "keep_awake",
        ] {
            assert!(ids.contains(&expected), "missing check {expected}");
        }
    }
}
