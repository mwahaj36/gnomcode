//! Model catalog, installed agent and routing policy handlers.

use super::*;

/// A model reported by an installed CLI catalog, with optional pricing metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCatalogItem {
    /// Agent identifier.
    pub agent: String,
    /// Model identifier.
    pub model: String,
    /// Human-readable display name.
    pub display_name: String,
    /// Provider name.
    pub provider: String,
    /// Context window size in tokens.
    pub context_window: Option<u64>,
    /// Estimated remaining quota percentage.
    pub remaining_pct: Option<u32>,
    /// Input cost per 1M tokens in USD.
    pub input_cost_per_mtok: Option<f64>,
    /// Output cost per 1M tokens in USD.
    pub output_cost_per_mtok: Option<f64>,
    /// Where the CLI reported the model; this is not a pricing catalog.
    pub availability_source: String,
    /// Time the official CLI refreshed its model catalog.
    pub observed_at: String,
}

pub(super) async fn installed_agents_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let agents = vec![
        json!({
            "id": "claude",
            "name": "Claude Code",
            "installed": gnomcode_platform::executables::executable_available("claude"),
            "install_command": "npm install -g @anthropic-ai/claude-code",
            "login_command": "claude auth login"
        }),
        json!({
            "id": "codex",
            "name": "OpenAI Codex",
            "installed": gnomcode_platform::executables::executable_available("codex"),
            "install_command": "npm install -g @openai/codex",
            "login_command": "codex login"
        }),
        json!({
            "id": "antigravity",
            "name": "Google Antigravity",
            "installed": gnomcode_platform::executables::executable_available("antigravity"),
            "install_command": "Install Antigravity from antigravity.google",
            "login_command": "agy"
        }),
        json!({
            "id": "terminal",
            "name": "Terminal Shell",
            "installed": true,
            "install_command": serde_json::Value::Null,
            "login_command": serde_json::Value::Null
        }),
    ];
    Ok(Json(json!({ "agents": agents })))
}

pub(super) fn codex_models_from_cache(
    bytes: &[u8],
    now: chrono::DateTime<chrono::Utc>,
) -> Vec<ModelCatalogItem> {
    let Ok(cache) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return Vec::new();
    };
    let Some(observed) = cache
        .get("fetched_at")
        .and_then(serde_json::Value::as_str)
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
    else {
        return Vec::new();
    };
    let age = now.signed_duration_since(observed);
    if age < chrono::Duration::minutes(-5) || age > chrono::Duration::hours(24) {
        return Vec::new();
    }
    let Some(models) = cache.get("models").and_then(serde_json::Value::as_array) else {
        return Vec::new();
    };
    let mut seen = std::collections::HashSet::new();
    models
        .iter()
        .filter_map(|entry| {
            if entry.get("visibility").and_then(serde_json::Value::as_str) != Some("list") {
                return None;
            }
            let model = entry.get("slug")?.as_str()?;
            if model.is_empty()
                || model.len() > 128
                || model.starts_with('-')
                || !model
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"-_.:/".contains(&byte))
                || !seen.insert(model)
            {
                return None;
            }
            let pricing = gnomcode_core::pricing::all_model_pricings()
                .iter()
                .find(|price| price.family.eq_ignore_ascii_case(model));
            Some(ModelCatalogItem {
                agent: "codex".into(),
                model: model.into(),
                display_name: entry
                    .get("display_name")
                    .and_then(serde_json::Value::as_str)
                    .filter(|name| !name.trim().is_empty())
                    .unwrap_or(model)
                    .into(),
                provider: "openai".into(),
                context_window: entry
                    .get("context_window")
                    .and_then(serde_json::Value::as_u64)
                    .filter(|value| *value > 0)
                    .or_else(|| pricing.map(|price| price.context_window)),
                remaining_pct: None,
                input_cost_per_mtok: pricing.map(|price| price.input_per_mtok_usd),
                output_cost_per_mtok: pricing.map(|price| price.output_per_mtok_usd),
                availability_source: "codex_cli_cache".into(),
                observed_at: observed.to_rfc3339(),
            })
        })
        .collect()
}

/// Reads fresh, visible CLI model metadata without reading provider credentials.
#[must_use]
pub fn available_models() -> Vec<ModelCatalogItem> {
    use std::io::Read as _;
    if !gnomcode_platform::executables::executable_available("codex") {
        return Vec::new();
    }
    let cache = gnomcode_platform::discover_agent_roots()
        .into_iter()
        .find(|root| root.agent == gnomcode_core::AgentId::Codex)
        .and_then(|root| {
            root.path
                .parent()
                .map(|path| path.join(obfstr::obfstr!("models_cache.json")))
        });
    let Some(path) = cache else {
        return Vec::new();
    };
    let Ok(file) = gnomcode_platform::open_shared_read(&path) else {
        return Vec::new();
    };
    let mut bytes = Vec::new();
    if file
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .is_err()
        || bytes.len() > 8 * 1024 * 1024
    {
        return Vec::new();
    }
    codex_models_from_cache(&bytes, chrono::Utc::now())
}

pub(super) async fn models_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let models = tokio::task::spawn_blocking(available_models)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(models))
}

pub(super) async fn model_catalog_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    Ok(Json(
        json!({"providers":crate::model_catalog::discover().await}),
    ))
}

pub(super) async fn routing_policy_get_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let db = state.db.lock().await;
    let policy = db
        .get_routing_policy()
        .unwrap_or(gnomcode_core::RoutingPolicy::AskFirst);
    Ok(Json(json!({ "policy": policy.as_str() })))
}

/// Payload for updating global routing policy.
#[derive(Debug, Deserialize)]
pub struct UpdateRoutingPolicyRequest {
    /// Policy string: `ask_first`, `automatic`, `deny_cross_provider`.
    pub policy: String,
}

pub(super) async fn routing_policy_post_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<UpdateRoutingPolicyRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let Some(policy) = gnomcode_core::RoutingPolicy::parse_policy(&payload.policy) else {
        return Err(StatusCode::BAD_REQUEST);
    };

    let _control = crate::CONVERSATION_CONTROL.lock().await;
    let db = state.db.lock().await;
    db.save_routing_policy(policy)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({ "ok": true, "policy": policy.as_str() })))
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

    #[test]
    fn model_picker_uses_only_visible_fresh_cli_models() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-10-29T05:00:00Z")
            .expect("time")
            .with_timezone(&chrono::Utc);
        let bytes = br#"{"fetched_at":"2026-10-29T04:00:00Z","models":[{"slug":"gpt-test-visible","display_name":"Visible CLI model","visibility":"list"},{"slug":"gpt-test-hidden","visibility":"hide"},{"slug":"--yolo","visibility":"list"},{"slug":"gpt-no-visibility"},{"slug":"gpt-test-visible","visibility":"list"}]}"#;
        let items = super::codex_models_from_cache(bytes, now);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].model, "gpt-test-visible");
        assert_eq!(items[0].display_name, "Visible CLI model");
        assert_eq!(items[0].availability_source, "codex_cli_cache");
        assert_eq!(items[0].input_cost_per_mtok, None);
        assert_eq!(items[0].context_window, None);
        assert!(super::codex_models_from_cache(bytes, now + chrono::Duration::days(2)).is_empty());
        assert!(super::codex_models_from_cache(b"broken", now).is_empty());
        assert!(super::codex_models_from_cache(br#"{"models":[]}"#, now).is_empty());
    }

    #[tokio::test]
    async fn routing_policy_endpoints_get_and_set() {
        let app = build_router(AppState {
            db: Arc::new(Mutex::new(Database::open_in_memory().expect("database"))),
            bearer_token: "policy-token".to_owned(),
            orchestrator: None,
            remote: Arc::new(crate::remote::RemoteManager::new()),
        });

        // 1. GET default policy
        let req_get = Request::builder()
            .uri("/api/v1/routing/policy")
            .header("Host", "127.0.0.1")
            .header("Authorization", "Bearer policy-token")
            .body(Body::empty())
            .expect("request");
        let res_get = app.clone().oneshot(req_get).await.expect("res");
        assert_eq!(res_get.status(), StatusCode::OK);

        // 2. POST update policy
        let req_post = Request::builder()
            .method("POST")
            .uri("/api/v1/routing/policy")
            .header("Host", "127.0.0.1")
            .header("Authorization", "Bearer policy-token")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"policy":"automatic"}"#))
            .expect("request");
        let res_post = app.clone().oneshot(req_post).await.expect("res");
        assert_eq!(res_post.status(), StatusCode::OK);

        // 3. POST invalid policy -> 400 Bad Request
        let req_bad = Request::builder()
            .method("POST")
            .uri("/api/v1/routing/policy")
            .header("Host", "127.0.0.1")
            .header("Authorization", "Bearer policy-token")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"policy":"invalid_policy_name"}"#))
            .expect("request");
        let res_bad = app.oneshot(req_bad).await.expect("res");
        assert_eq!(res_bad.status(), StatusCode::BAD_REQUEST);
    }
}
