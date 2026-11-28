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
    let client = https_client()?;
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
    complete_cloud_sign_in(&state, &client, session, base, key).await?;
    Ok(Json(json!({"success":true})))
}

/// Saves a Supabase session and links this desktop to the account.
async fn complete_cloud_sign_in(
    state: &AppState,
    client: &reqwest::Client,
    session: Value,
    base: String,
    key: String,
) -> Result<(), StatusCode> {
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
        .set_account_access(client, &auth, true)
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    Ok(())
}

/// One desktop OAuth sign-in in flight: PKCE verifier and when it started.
static PENDING_OAUTH: std::sync::Mutex<Option<(String, std::time::Instant)>> =
    std::sync::Mutex::new(None);
const OAUTH_TTL: std::time::Duration = std::time::Duration::from_secs(600);

fn https_client() -> Result<reqwest::Client, StatusCode> {
    reqwest::Client::builder()
        .https_only(true)
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
}

/// RFC 7636 S256 code challenge: unpadded base64url of SHA-256(verifier).
pub(crate) fn pkce_challenge(verifier: &str) -> String {
    use sha2::Digest;
    gnomcode_core::file_preview::encode_base64(&sha2::Sha256::digest(verifier.as_bytes()))
        .replace('+', "-")
        .replace('/', "_")
        .trim_end_matches('=')
        .to_owned()
}

/// Starts browser sign-in with GitHub or Google (SPEC 3.15). Returns the URL the console opens
/// in the system browser; the provider returns to `/auth/callback` on this loopback port.
pub(super) async fn auth_oauth_start_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    if payload["remote_access_confirmed"] != true {
        return Err(StatusCode::PRECONDITION_REQUIRED);
    }
    let provider = match payload["provider"].as_str() {
        Some(p @ ("github" | "google")) => p,
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    let port = headers
        .get("host")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.rsplit_once(':'))
        .and_then(|(_, port)| port.parse::<u16>().ok())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let verifier: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    let challenge = pkce_challenge(&verifier);
    *PENDING_OAUTH
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? =
        Some((verifier, std::time::Instant::now()));
    let (base, _) = crate::sync::cloud_config();
    let redirect = format!("http://127.0.0.1:{port}/auth/callback");
    let url = reqwest::Url::parse_with_params(
        &format!("{base}/auth/v1/authorize"),
        &[
            ("provider", provider),
            ("redirect_to", redirect.as_str()),
            ("code_challenge", challenge.as_str()),
            ("code_challenge_method", "s256"),
        ],
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({"url": url.as_str()})))
}

fn callback_page(ok: bool) -> axum::response::Html<String> {
    let (title, body) = if ok {
        (
            "Signed in",
            "Gnomcode is connected to your account. You can close this tab and return to the app.",
        )
    } else {
        (
            "Sign-in did not finish",
            "Start sign-in again from Gnomcode. The link works once and expires after 10 minutes.",
        )
    };
    axum::response::Html(format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Gnomcode</title><style>body{{margin:0;min-height:100vh;display:grid;place-items:center;background:#0f1117;color:#e6e8ee;font:16px system-ui,sans-serif;text-align:center;padding:16px}}p{{color:#9aa1b2}}</style></head><body><main><h1>{title}</h1><p>{body}</p></main></body></html>"
    ))
}

/// Provider return for desktop OAuth. The browser holds no bearer, so this only completes a
/// pending flow the console started: the code is useless without the in-memory verifier.
pub(super) async fn auth_callback_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    axum::extract::Query(query): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let loopback = headers
        .get("host")
        .and_then(|h| h.to_str().ok())
        .map(|h| h.split(':').next().unwrap_or(h))
        .is_some_and(|h| h == "127.0.0.1" || h == "localhost");
    let pending = PENDING_OAUTH.lock().ok().and_then(|mut slot| slot.take());
    let (Some(code), Some((verifier, started)), true) = (query.get("code"), pending, loopback)
    else {
        return callback_page(false);
    };
    if started.elapsed() > OAUTH_TTL || code.len() > 512 {
        return callback_page(false);
    }
    let Ok(client) = https_client() else {
        return callback_page(false);
    };
    let (base, key) = crate::sync::cloud_config();
    let Ok(response) = client
        .post(format!("{base}/auth/v1/token?grant_type=pkce"))
        .header("apikey", &key)
        .json(&json!({"auth_code": code, "code_verifier": verifier}))
        .send()
        .await
    else {
        return callback_page(false);
    };
    if !response.status().is_success() {
        return callback_page(false);
    }
    let Ok(session) = response.json::<Value>().await else {
        return callback_page(false);
    };
    callback_page(
        complete_cloud_sign_in(&state, &client, session, base, key)
            .await
            .is_ok(),
    )
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

#[cfg(test)]
mod tests {
    #[test]
    fn pkce_challenge_is_unpadded_base64url_sha256() {
        assert_eq!(
            super::pkce_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gXr1Q8ylA"),
            // Python: base64.urlsafe_b64encode(hashlib.sha256(v).digest()).rstrip(b"=")
            "FgsCsZiDyRueoKIjFF_6HwgT0n_-DpVbNU5hTO-lWyw"
        );
    }
}
