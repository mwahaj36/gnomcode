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

#[derive(Debug, Deserialize, Default)]
pub(super) struct OnboardingQuery {
    provider: Option<String>,
    distro: Option<String>,
}

/// `GET /api/v1/onboarding/status?provider=&distro=`: one provider, or all three.
pub(super) async fn onboarding_status_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(params): Query<OnboardingQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let distro = params.distro.as_deref().filter(|d| !d.is_empty());
    let wanted: Vec<&str> = match params.provider.as_deref() {
        Some(p) => vec![p],
        None => crate::onboarding::PROVIDERS.to_vec(),
    };
    let mut providers = Vec::new();
    for provider in wanted {
        let status = crate::onboarding::status(&state.db, provider, distro)
            .await
            .ok_or(StatusCode::BAD_REQUEST)?;
        providers.push(status);
    }
    Ok(Json(json!({ "providers": providers })))
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

    #[tokio::test]
    async fn onboarding_status_requires_auth_and_rejects_bad_input() {
        let app = build_router(AppState {
            db: Arc::new(Mutex::new(Database::open_in_memory().expect("database"))),
            bearer_token: "t".to_owned(),
            orchestrator: None,
            remote: Arc::new(crate::remote::RemoteManager::new()),
        });
        let get = |uri: &str, auth: bool| {
            let mut b = Request::builder().uri(uri).header("Host", "127.0.0.1");
            if auth {
                b = b.header("Authorization", "Bearer t");
            }
            b.body(Body::empty()).expect("request")
        };
        let res = app
            .clone()
            .oneshot(get("/api/v1/onboarding/status", false))
            .await
            .expect("res");
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        for bad in [
            "/api/v1/onboarding/status?provider=gemini",
            "/api/v1/onboarding/status?provider=codex&distro=-x",
        ] {
            let res = app.clone().oneshot(get(bad, true)).await.expect("res");
            assert_eq!(res.status(), StatusCode::BAD_REQUEST, "{bad}");
        }
    }
}
