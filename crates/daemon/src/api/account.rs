//! Supabase configuration and auth session handlers.

use super::*;

pub(super) async fn supabase_config_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let mut url = std::env::var("GNOMCODE_SUPABASE_URL").unwrap_or_default();
    let mut key = std::env::var("GNOMCODE_SUPABASE_KEY").unwrap_or_default();

    if url.is_empty() || key.is_empty() {
        let home = gnomcode_platform::user_home_dir();
        let env_file = home.join(".gnomcode").join("supabase.env");
        if let Ok(content) = std::fs::read_to_string(env_file) {
            for line in content.lines() {
                let trimmed = line.trim();
                if let Some(rest) = trimmed.strip_prefix("GNOMCODE_SUPABASE_URL=") {
                    url = rest.trim().to_owned();
                } else if let Some(rest) = trimmed.strip_prefix("GNOMCODE_SUPABASE_KEY=") {
                    key = rest.trim().to_owned();
                }
            }
        }
    }

    Ok((
        cors_headers(),
        Json(json!({
            "supabase_url": url,
            "supabase_key": key,
        })),
    ))
}

pub(super) fn session_file_path() -> std::path::PathBuf {
    gnomcode_platform::gnomcode_home().join("session.json")
}

pub(super) async fn auth_session_get_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let path = session_file_path();
    let session = std::fs::read(&path)
        .ok()
        .and_then(|raw| serde_json::from_slice::<Value>(&raw).ok());
    let paired = session.as_ref().is_some_and(|session| {
        session
            .pointer("/user/id")
            .and_then(Value::as_str)
            .is_some()
            && session["access_token"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
            && session["refresh_token"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
    });
    let email = session
        .as_ref()
        .and_then(|session| session.pointer("/user/email"))
        .and_then(Value::as_str);
    Ok((
        cors_headers(),
        Json(json!({ "paired": paired, "email": email })),
    ))
}

pub(super) async fn auth_session_post_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    if payload["remote_access_confirmed"] != true {
        return Err(StatusCode::PRECONDITION_REQUIRED);
    }
    let client = reqwest::Client::builder()
        .https_only(true)
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let (base, key) = crate::sync::cloud_config();
    crate::sync::validate_session(&client, &base, &key, &payload)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    crate::sync::save_cloud_session(&session_file_path(), &payload)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let owner = payload
        .pointer("/user/id")
        .and_then(Value::as_str)
        .ok_or(StatusCode::BAD_REQUEST)?;
    let auth = crate::sync::CloudAuth {
        user_id: owner.to_owned(),
        access_token: payload["access_token"]
            .as_str()
            .ok_or(StatusCode::BAD_REQUEST)?
            .to_owned(),
        base_url: base,
        public_key: key,
    };
    state
        .remote
        .devices
        .set_account_access(&client, &auth, true)
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    Ok((cors_headers(), Json(json!({ "success": true }))))
}

pub(super) async fn auth_signin_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    if payload["remote_access_confirmed"] != true {
        return Err(StatusCode::PRECONDITION_REQUIRED);
    }
    let email = payload["email"]
        .as_str()
        .filter(|value| value.len() <= 320 && value.contains('@'))
        .ok_or(StatusCode::BAD_REQUEST)?;
    let password = payload["password"]
        .as_str()
        .filter(|value| !value.is_empty() && value.len() <= 1024)
        .ok_or(StatusCode::BAD_REQUEST)?;
    let client = reqwest::Client::builder()
        .https_only(true)
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let (base, key) = crate::sync::cloud_config();
    let response = client
        .post(format!("{base}/auth/v1/token?grant_type=password"))
        .header("apikey", &key)
        .json(&json!({"email":email,"password":password}))
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    if !response.status().is_success() {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let session = response
        .json::<Value>()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    crate::sync::save_cloud_session(&session_file_path(), &session)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let owner = session
        .pointer("/user/id")
        .and_then(Value::as_str)
        .ok_or(StatusCode::BAD_GATEWAY)?;
    let auth = crate::sync::CloudAuth {
        user_id: owner.to_owned(),
        access_token: session["access_token"]
            .as_str()
            .ok_or(StatusCode::BAD_REQUEST)?
            .to_owned(),
        base_url: base,
        public_key: key,
    };
    state
        .remote
        .devices
        .set_account_access(&client, &auth, true)
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    Ok(Json(json!({"success":true})))
}

pub(super) async fn auth_session_delete_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let path = session_file_path();
    if let Ok(bytes) = tokio::fs::read(&path).await
        && let Ok(session) = serde_json::from_slice::<Value>(&bytes)
        && let Some(owner) = session.pointer("/user/id").and_then(Value::as_str)
    {
        state
            .remote
            .devices
            .disconnect_owner(owner)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    state
        .remote
        .devices
        .disconnect()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    state
        .remote
        .revoke()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    crate::sync::remove_cloud_session(&path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((cors_headers(), Json(json!({ "success": true }))))
}
