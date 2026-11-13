//! Guardrail rules, alerts, resume queue and relay handlers.

use super::*;

pub(super) async fn rules_get_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let db = state.db.lock().await;
    Ok(Json(
        db.list_rules()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

pub(super) async fn rules_post_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(rule): Json<GuardrailRule>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    if rule.id.is_empty()
        || rule.id.len() > 64
        || !rule
            .id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        || rule
            .max_spend_usd
            .is_some_and(|v| !v.is_finite() || v <= 0.0)
        || rule.max_turns == Some(0)
        || (rule.max_spend_usd.is_none() && rule.max_turns.is_none())
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    let db = state.db.lock().await;
    db.save_rule(&rule)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rule))
}

pub(super) async fn rule_delete_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let db = state.db.lock().await;
    db.delete_rule(&id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({ "success": true })))
}

pub(super) async fn alerts_get_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let db = state.db.lock().await;
    Ok(Json(
        db.list_alerts(100)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

pub(super) async fn resume_queue_get_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let db = state.db.lock().await;
    Ok(Json(
        db.list_resume_queue()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

#[derive(Debug, Deserialize)]
pub(crate) struct ArmResumeQueueRequest {
    pub(crate) session_id: String,
    pub(crate) resume_prompt: Option<String>,
}

pub(super) async fn resume_queue_arm_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<ArmResumeQueueRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    if payload
        .resume_prompt
        .as_deref()
        .is_some_and(|prompt| prompt != "continue")
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    if payload.session_id.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let reply = crate::automation::command(
        &state.db,
        json!({"type":"resume_on_reset","session_id":payload.session_id,"enabled":true}),
    )
    .await;
    if reply["type"] == "error" {
        return Err(StatusCode::CONFLICT);
    }
    Ok(Json(reply))
}

#[derive(Debug, Deserialize)]
pub(crate) struct DisarmResumeQueueRequest {
    pub(crate) session_id: String,
}

pub(super) async fn resume_queue_disarm_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<DisarmResumeQueueRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let db = state.db.lock().await;
    db.disarm_resume_queue(&payload.session_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({ "success": true })))
}

pub(super) async fn relay_get_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let db = state.db.lock().await;
    let config = db
        .get_context_relay_config()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let relays = db
        .list_context_relays()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({
        "is_pro": true,
        "config": config,
        "relays": relays,
    })))
}

#[derive(Debug, Deserialize)]
pub(crate) struct RelayConfigUpdateRequest {
    pub(crate) policy: Option<String>,
    pub(crate) strategy: Option<String>,
    pub(crate) threshold_pct: Option<u32>,
}

pub(super) async fn relay_config_post_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<RelayConfigUpdateRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let db = state.db.lock().await;
    let mut config = db
        .get_context_relay_config()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if let Some(p) = payload.policy
        && matches!(p.as_str(), "ask_first" | "auto" | "disabled")
    {
        config.policy = p;
    }
    if let Some(s) = payload.strategy
        && matches!(s.as_str(), "same_agent" | "failover")
    {
        config.strategy = s;
    }
    if let Some(t) = payload.threshold_pct
        && (50..=100).contains(&t)
    {
        config.threshold_pct = t;
    }
    db.save_context_relay_config(&config)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({ "success": true, "config": config })))
}

#[derive(Debug, Deserialize)]
pub(crate) struct RelayActionRequest {
    pub(crate) id: String,
}

pub(super) async fn relay_execute_post_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<RelayActionRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let db = state.db.lock().await;
    let relays = db
        .list_context_relays()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let entry = if let Some(e) = relays
        .into_iter()
        .find(|r| r.id == payload.id || r.session_id == payload.id)
    {
        e
    } else {
        let (sessions, _) = db
            .list_sessions_paginated(None, None, 0, 100)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let sid = payload.id.trim_start_matches("relay-");
        if let Some(s) = sessions.iter().find(|s| s.id == sid || s.id == payload.id) {
            let cwd = db
                .get_session_focus_target(&s.id)
                .ok()
                .flatten()
                .and_then(|t| t.repo_root)
                .filter(|p| !p.trim().is_empty())
                .or_else(|| db.find_antigravity_workspace(&s.id))
                // No recorded folder (Codex chat, temporary, unassigned): never start the agent somewhere else.
                .ok_or(StatusCode::CONFLICT)?;
            let entry = gnomcode_store::ContextRelayEntry {
                id: format!("relay-{}", s.id),
                session_id: s.id.clone(),
                source_agent: s.agent.clone(),
                target_agent: s.agent.clone(),
                project_name: s.project_name.clone(),
                cwd,
                fill_pct: 92,
                status: "pending".to_owned(),
                created_at: chrono::Utc::now().to_rfc3339(),
                executed_at: None,
            };
            db.record_context_relay(&entry)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            entry
        } else {
            return Err(StatusCode::NOT_FOUND);
        }
    };

    crate::pro::execute_relay_handoff(&entry).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    db.update_context_relay_status(&entry.id, "executed")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(
        json!({ "success": true, "id": entry.id, "status": "executed" }),
    ))
}

pub(super) async fn relay_dismiss_post_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<RelayActionRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let db = state.db.lock().await;
    db.update_context_relay_status(&payload.id, "dismissed")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        json!({ "success": true, "id": payload.id, "status": "dismissed" }),
    ))
}

#[derive(Debug, Deserialize)]
pub(crate) struct GuardrailCheckQuery {
    pub(crate) session_id: String,
}

pub(super) async fn guardrail_check_handler(
    Query(query): Query<GuardrailCheckQuery>,
) -> impl IntoResponse {
    let blocked = crate::pro::is_session_blocked(&query.session_id);
    Json(json!({ "blocked": blocked }))
}

pub(super) async fn automation_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    Ok(Json(crate::automation::command(&state.db, request).await))
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
    async fn local_automation_and_export_are_available_without_a_license() {
        let app = build_router(AppState {
            db: Arc::new(Mutex::new(Database::open_in_memory().expect("database"))),
            bearer_token: "local-test-token".to_owned(),
            orchestrator: None,
            remote: Arc::new(crate::remote::RemoteManager::new()),
        });
        for path in [
            "/api/v1/rules",
            "/api/v1/alerts",
            "/api/v1/export",
            "/api/v1/resume-queue",
            "/api/v1/pro/relay",
        ] {
            let request = Request::builder()
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
                StatusCode::OK,
                "{path}"
            );
        }
    }

    #[tokio::test]
    async fn resume_queue_rejects_custom_prompts_without_persisting_them() {
        let app = build_router(AppState {
            db: Arc::new(Mutex::new(Database::open_in_memory().expect("database"))),
            bearer_token: "local-test-token".to_owned(),
            orchestrator: None,
            remote: Arc::new(crate::remote::RemoteManager::new()),
        });
        let request = Request::builder()
            .method("POST")
            .uri("/api/v1/resume-queue/arm")
            .header("Host", "127.0.0.1:41738")
            .header("Authorization", "Bearer local-test-token")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"session_id":"codex:test","agent":"codex","resume_prompt":"private test text"}"#))
            .expect("request");
        assert_eq!(
            app.oneshot(request).await.expect("response").status(),
            StatusCode::BAD_REQUEST
        );
    }
}
