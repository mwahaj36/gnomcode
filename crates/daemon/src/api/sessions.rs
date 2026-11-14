//! Session launch, focus, prompt, PTY stream, listing and live snapshot handlers.

use super::*;

/// Query parameters for listing sessions with pagination & filters.
#[derive(Debug, Deserialize, Default)]
pub struct SessionQueryParams {
    /// Exact local project ID; filtering precedes pagination.
    pub project: Option<String>,
    /// 1-indexed page number.
    pub page: Option<usize>,
    /// Limit per page (default 10).
    pub limit: Option<usize>,
    /// Agent slug filter (e.g. `antigravity`, `codex`, `claude-code`, or `all`).
    pub agent: Option<String>,
    /// Status filter (`active`, `idle`, or `all`).
    pub status: Option<String>,
    /// Conversation metadata search, applied before pagination.
    pub search: Option<String>,
    /// Managed Gnomcode conversations or imported view-only sessions.
    pub source: Option<String>,
    /// Include child agent conversations (off by default).
    pub include_subagents: Option<bool>,
}

pub(super) async fn focus_session_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let target = state
        .db
        .lock()
        .await
        .get_session_focus_target(&id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let Some(target) = target else {
        return Err(StatusCode::NOT_FOUND);
    };
    let repo_path = target.repo_root.clone();
    let (focused, destination) = tokio::task::spawn_blocking(move || {
        if target.pid.is_some_and(gnomcode_platform::bring_to_front) {
            return (true, "process");
        }
        if gnomcode_platform::focus_agent_application(&target.agent, &target.surface) {
            return (true, "application");
        }
        (false, "unavailable")
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        json!({ "ok": focused, "focused": focused, "target": destination, "exact_session": false, "repo_path": if focused { None } else { repo_path },
            "message": if focused { "Opened the agent window. Select the session in the app if needed." } else { "The session window could not be opened. Open the agent app to continue." }
        }),
    ))
}

/// Request payload to launch an agent in a workspace.
#[derive(Debug, Deserialize)]
pub struct LaunchSessionRequest {
    /// Agent identifier (e.g. `claude`, `codex`, `antigravity`).
    pub agent: String,
    /// Absolute path to the repository directory.
    pub repo_path: String,
    /// Optional model identifier.
    pub model: Option<String>,
    /// Run the agent in its own git worktree on a new branch (ADR 0051).
    #[serde(default)]
    pub worktree: bool,
    /// Optional branch name for the worktree; defaults to `gnomcode/<session id>`.
    #[serde(default)]
    pub worktree_branch: Option<String>,
}

pub(super) async fn launch_session_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<LaunchSessionRequest>,
) -> Result<axum::response::Response, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let session_id = format!(
        "{}-{}",
        payload.agent,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    let repo = std::path::Path::new(&payload.repo_path);
    let starting = match crate::wsl_launch::preflight(&payload.agent, repo).await {
        Ok(starting) => starting,
        Err(crate::wsl_launch::Refusal::BadFolder) => return Err(StatusCode::BAD_REQUEST),
        Err(crate::wsl_launch::Refusal::Missing(message)) => {
            return Ok((
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(json!({"ok":false,"message":message})),
            )
                .into_response());
        }
    };
    let isolated = match crate::worktree_api::prepare_launch(
        repo,
        &session_id,
        payload.worktree,
        payload.worktree_branch.as_deref(),
    )
    .await
    {
        Ok(record) => record,
        Err(error) => {
            return Ok((
                error.status(),
                Json(json!({"ok":false,"message":error.to_string()})),
            )
                .into_response());
        }
    };
    let cwd = isolated
        .as_ref()
        .map_or_else(|| repo.to_path_buf(), |record| record.cwd.clone().into());
    match gnomcode_pty::launch_session(&session_id, &payload.agent, &cwd, payload.model.as_deref())
        .await
    {
        Ok(record) => {
            state
                .db
                .lock()
                .await
                .save_conversation_launch(
                    &record.session_id,
                    &record.agent,
                    &record.repo_path,
                    record.model.as_deref(),
                )
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            let mut body =
                serde_json::to_value(&record).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            if let (Some(isolated), Some(fields)) = (&isolated, body.as_object_mut())
                && let Value::Object(extra) = crate::worktree_api::fields(isolated)
            {
                fields.extend(extra);
            }
            crate::wsl_launch::annotate(&mut body, repo, starting);
            Ok(Json(body).into_response())
        }
        Err(_) => {
            if let Some(isolated) = &isolated {
                crate::worktree_api::abort_launch(isolated).await;
            }
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Registered WSL distros for the "Run in" picker (ADR 0060). Registry only plus the
/// running list; never starts a distro. Empty off Windows or without WSL.
pub(super) async fn wsl_distros_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let distros = tokio::task::spawn_blocking(|| {
        let running = gnomcode_platform::wsl::running_distros();
        gnomcode_platform::wsl::registered_distros()
            .into_iter()
            .map(|name| {
                let up = running.iter().any(|r| r.eq_ignore_ascii_case(&name));
                // Only a running distro is read; a stopped one stays "unknown".
                let states = if up {
                    gnomcode_platform::wsl::claude_hook_states(&name)
                } else {
                    Vec::new()
                };
                let hook = if states.is_empty() {
                    "unknown"
                } else if states.iter().all(|(_, hooked)| *hooked) {
                    "integrated"
                } else {
                    "missing"
                };
                json!({"name": name, "label": gnomcode_core::wsl::distro_label(&name), "running": up, "claude_hook": hook})
            })
            .collect::<Vec<Value>>()
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({ "distros": distros })))
}

/// Body of `POST /api/v1/wsl/integrate`.
#[derive(Debug, Deserialize)]
pub struct WslIntegrateRequest {
    /// Registered distro name.
    pub distro: String,
}

/// "Integrate in WSL": runs `gnomcode integrate claude` (which backs up first) for every
/// `~/.claude` of a running distro that lacks the hook, writing the interop path of this
/// binary. Never boots a distro; sessions keep working without the hook (fail open).
pub(super) async fn wsl_integrate_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<WslIntegrateRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let distro = payload.distro;
    let result = tokio::task::spawn_blocking(move || {
        let exe = std::env::current_exe().map_err(|_| "Gnomcode path unavailable".to_owned())?;
        let interop = gnomcode_platform::wsl::windows_to_wsl_path(&exe.to_string_lossy())
            .ok_or_else(|| "Gnomcode is not on a drive WSL can reach".to_owned())?;
        let states = gnomcode_platform::wsl::claude_hook_states(&distro);
        if states.is_empty() {
            return Err(format!("{distro} is not running or has no Claude settings yet. Start Claude there once, then retry."));
        }
        let mut done = 0;
        for (dir, _) in states.into_iter().filter(|(_, hooked)| !hooked) {
            let mut command = std::process::Command::new(&exe);
            command
                .args(["integrate", "claude"])
                .env("CLAUDE_CONFIG_DIR", &dir)
                .env("GNOMCODE_HOOK_EXE", &interop)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt as _;
                command.creation_flags(0x0800_0000);
            }
            if !command.status().is_ok_and(|s| s.success()) {
                return Err(format!("Integration failed in {distro}. Run `{interop} integrate claude` there."));
            }
            done += 1;
        }
        Ok(format!("Claude hook installed in {distro} ({done} profile(s))."))
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(match result {
        Ok(message) => Json(json!({"ok": true, "message": message})),
        Err(message) => Json(json!({"ok": false, "message": message})),
    })
}

pub(super) static WORKSPACE_TERMINAL_LAUNCH: tokio::sync::Mutex<()> =
    tokio::sync::Mutex::const_new(());

pub(super) async fn workspace_terminal_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let _launch = WORKSPACE_TERMINAL_LAUNCH.lock().await;
    let hosted = gnomcode_pty::list_active_sessions();
    let repo = if let Some(record) = hosted.iter().find(|record| record.session_id == id) {
        record.repo_path.clone()
    } else {
        state
            .db
            .lock()
            .await
            .get_session_focus_target(&id)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .and_then(|target| target.repo_root)
            .ok_or(StatusCode::NOT_FOUND)?
    };
    use sha2::Digest as _;
    let digest = sha2::Sha256::digest(format!("{id}:{repo}").as_bytes());
    let hash = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let session_id = format!("workspace-terminal-{hash}");
    if let Some(record) = hosted
        .into_iter()
        .find(|record| record.session_id == session_id)
    {
        return Ok(Json(record));
    }
    let record =
        gnomcode_pty::launch_session(&session_id, "terminal", std::path::Path::new(&repo), None)
            .await
            .map_err(|_| StatusCode::BAD_GATEWAY)?;
    Ok(Json(record))
}

pub(super) async fn stop_session_handler(
    headers: HeaderMap,
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    match gnomcode_pty::stop_session(&id).await {
        Ok(()) => Ok(Json(json!({ "ok": true, "session_id": id }))),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

pub(super) async fn list_hosted_sessions_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let hosted = gnomcode_pty::list_active_sessions();
    let db = state.db.lock().await;
    let sessions = db.list_active_sessions(usize::MAX).unwrap_or_default();
    let bindings = crate::remote::session_bindings(&sessions, &hosted);
    crate::remote::sync_conversation_bindings(&db, &sessions, &hosted)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let records: Vec<serde_json::Value> = hosted
        .iter()
        .filter(|record| {
            db.conversation_target(&record.session_id)
                .ok()
                .flatten()
                .is_none_or(|(host, _, _)| host == record.session_id)
        })
        .map(|r| {
            let root = db
                .conversation_id(&r.session_id)
                .unwrap_or_else(|_| r.session_id.clone());
            let saved_native = db
                .conversation_target(&r.session_id)
                .ok()
                .flatten()
                .and_then(|(_, native, _)| native);
            let native = bindings
                .iter()
                .find(|(_, record)| record.session_id == r.session_id)
                .map(|(session, _)| *session)
                .or_else(|| {
                    sessions
                        .iter()
                        .find(|session| Some(session.id.as_str()) == saved_native.as_deref())
                });
            let observed = native.and_then(|session| session.model.as_ref());
            let requested = db
                .conversation_launch(&root)
                .ok()
                .flatten()
                .and_then(|launch| launch.model);
            let pending = requested.as_ref().filter(|model| Some(*model) != observed);
            let title = crate::conversation_title(&db, &root)
                .or_else(|| native.and_then(|session| session.title.clone()))
                .unwrap_or_else(|| {
                    std::path::Path::new(&r.repo_path).file_name().map_or_else(
                        || "New conversation".to_owned(),
                        |name| name.to_string_lossy().into_owned(),
                    )
                });
            json!({
                "session_id": root,
                "agent": r.agent,
                "title": title,
                "repo_path": r.repo_path,
                "model": pending.or(observed).or(r.model.as_ref()),
                "model_observed": pending.is_none() && observed.is_some(),
                "started_at": r.started_at,
                "child_pid": r.child_pid,
            })
        })
        .collect();
    Ok(Json(json!({ "sessions": records })))
}

/// Request payload to deliver a prompt to an active session.
#[derive(Debug, Deserialize)]
pub struct PromptSessionRequest {
    /// Prompt text to submit.
    pub text: String,
    /// Optional client-generated idempotency key for deduplicated immediate or queued delivery.
    #[serde(default)]
    pub id: Option<String>,
    /// When true, queue the prompt for the next confirmed safe turn boundary if the session is busy or not ready.
    #[serde(default)]
    pub queue_if_busy: Option<bool>,
}

/// Request payload to update a pending queued prompt.
#[derive(Debug, Deserialize)]
pub struct UpdateQueuedPromptRequest {
    /// Updated prompt text.
    pub text: String,
}

fn validate_prompt_text(raw: &str) -> Result<&str, &'static str> {
    let text = raw.trim();
    if text.is_empty()
        || text.encode_utf16().count() > 4000
        || text
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\t'))
        || text.starts_with('/')
    {
        return Err(
            "Use a nonempty message up to 4000 characters without native slash commands or control characters.",
        );
    }
    Ok(text)
}

pub(crate) fn valid_message_id(raw: &str) -> Option<&str> {
    let trimmed = raw.trim();
    (!trimmed.is_empty()
        && trimmed.len() <= 96
        && trimmed
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b':')))
    .then_some(trimmed)
}

pub(super) async fn prompt_session_handler(
    headers: HeaderMap,
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(payload): Json<PromptSessionRequest>,
) -> Result<axum::response::Response, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let text = match validate_prompt_text(&payload.text) {
        Ok(text) => text,
        Err(msg) => {
            return Ok((StatusCode::BAD_REQUEST, Json(json!({ "message": msg }))).into_response());
        }
    };

    if let Some(raw_id) = payload.id.as_deref() {
        let Some(msg_id) = valid_message_id(raw_id) else {
            return Ok((
                StatusCode::BAD_REQUEST,
                Json(json!({ "message": "Invalid prompt delivery id" })),
            )
                .into_response());
        };
        let existing = {
            let db = state.db.lock().await;
            db.get_scheduled_message(&id, msg_id)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        };
        if let Some(item) = existing {
            let status = match item.status.as_str() {
                "pending" => "queued",
                "delivering" => "sending",
                "delivered" => "sent",
                other => other,
            };
            return Ok(Json(json!({
                "ok": matches!(status, "queued" | "sending" | "sent"),
                "status": status,
                "id": item.id,
                "session_id": id,
                "deduplicated": true,
            }))
            .into_response());
        }
    }

    // Observed external sessions without a managed launch must never silently become managed.
    let (exists, is_managed) = {
        let db = state.db.lock().await;
        let sessions = db.list_active_sessions(usize::MAX).unwrap_or_default();
        let hosted = gnomcode_pty::list_active_sessions();
        let exists = sessions.iter().any(|s| s.id == id)
            || crate::remote::conversation_host(&db, &id, &sessions, &hosted)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                .is_some();
        let managed = db
            .conversation_launch(&id)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .is_some()
            || crate::remote::conversation_host(&db, &id, &sessions, &hosted)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                .is_some();
        (exists, managed)
    };
    if exists && !is_managed {
        return Ok((
            StatusCode::CONFLICT,
            Json(json!({
                "status": "observed_read_only",
                "message": "Observed external session is read-only. Use Continue here in Actions to start a managed session."
            })),
        )
            .into_response());
    }

    match crate::deliver_conversation_prompt(&state.db, &id, text).await {
        Ok(()) => Ok(Json(json!({
            "ok": true,
            "status": "sent",
            "id": payload.id,
            "session_id": id
        }))
        .into_response()),
        Err(error @ (crate::HandoffError::Busy | crate::HandoffError::NotReady))
            if payload.queue_if_busy == Some(true) =>
        {
            let msg_id = payload
                .id
                .as_deref()
                .and_then(valid_message_id)
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| format!("q-{}", uuid_like_id()));
            let db = state.db.lock().await;
            match db.schedule_message(&msg_id, &id, text, 0) {
                Ok(_) => Ok(Json(json!({
                    "ok": true,
                    "status": "queued",
                    "id": msg_id,
                    "session_id": id,
                    "reason": match error {
                        crate::HandoffError::Busy => "Agent is busy; queued for next safe turn boundary.",
                        _ => "Agent is not at prompt yet; queued for next safe turn boundary.",
                    }
                }))
                .into_response()),
                Err(_) => Ok(handoff_error_response(error)),
            }
        }
        Err(error) => Ok(handoff_error_response(error)),
    }
}

fn uuid_like_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    format!("{nanos:x}-{}", std::process::id())
}

pub(super) async fn queue_get_handler(
    headers: HeaderMap,
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let db = state.db.lock().await;
    let items = db
        .scheduled_messages(&id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({ "session_id": id, "items": items })))
}

pub(super) async fn queue_patch_handler(
    headers: HeaderMap,
    Path((id, msg_id)): Path<(String, String)>,
    State(state): State<AppState>,
    Json(payload): Json<UpdateQueuedPromptRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let text = match validate_prompt_text(&payload.text) {
        Ok(text) => text,
        Err(msg) => {
            return Ok((StatusCode::BAD_REQUEST, Json(json!({ "message": msg }))).into_response());
        }
    };
    let db = state.db.lock().await;
    let updated = db
        .update_scheduled_message(&id, &msg_id, text)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !updated {
        return Ok((
            StatusCode::CONFLICT,
            Json(json!({ "message": "Queued prompt can no longer be edited." })),
        )
            .into_response());
    }
    Ok(Json(json!({ "ok": true, "id": msg_id, "status": "queued" })).into_response())
}

pub(super) async fn queue_delete_handler(
    headers: HeaderMap,
    Path((id, msg_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let db = state.db.lock().await;
    let cancelled = db
        .cancel_message(&id, &msg_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !cancelled {
        return Ok((
            StatusCode::CONFLICT,
            Json(json!({ "message": "Queued prompt can no longer be cancelled." })),
        )
            .into_response());
    }
    Ok(Json(json!({ "ok": true, "id": msg_id, "status": "cancelled" })).into_response())
}

/// Query parameters for PTY streaming WebSocket.
#[derive(Debug, Deserialize)]
pub struct PtyStreamQuery {
    /// Bearer authentication token.
    pub token: Option<String>,
}

pub(super) async fn pty_stream_handler(
    headers: HeaderMap,
    Query(query): Query<PtyStreamQuery>,
    Path(id): Path<String>,
    State(state): State<AppState>,
    ws: axum::extract::ws::WebSocketUpgrade,
) -> Result<impl IntoResponse, StatusCode> {
    validate_pty_auth(&headers, query.token.as_deref(), &state.bearer_token)?;

    let hosted = gnomcode_pty::list_active_sessions();
    let db = state.db.lock().await;
    let sessions = db.list_active_sessions(usize::MAX).unwrap_or_default();
    let record = crate::remote::conversation_host(&db, &id, &sessions, &hosted)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let backend_url = format!(
        "ws://127.0.0.1:{}/stream?token={}",
        record.port, record.token
    );
    drop(db);

    Ok(ws.on_upgrade(move |client_ws| async move {
        proxy_pty_ws(client_ws, backend_url).await;
    }))
}

pub(super) fn validate_pty_auth(
    headers: &HeaderMap,
    query_token: Option<&str>,
    expected: &str,
) -> Result<(), StatusCode> {
    match validate_auth(headers, expected) {
        Err(StatusCode::UNAUTHORIZED) if query_token == Some(expected) => {
            let mut authenticated = headers.clone();
            let value = format!("Bearer {expected}")
                .parse()
                .map_err(|_| StatusCode::UNAUTHORIZED)?;
            authenticated.insert("authorization", value);
            validate_auth(&authenticated, expected)
        }
        result => result,
    }
}

pub(super) async fn proxy_pty_ws(client_ws: axum::extract::ws::WebSocket, backend_url: String) {
    use futures_util::{SinkExt, StreamExt};
    let Ok((backend_ws, _)) = tokio_tungstenite::connect_async(&backend_url).await else {
        return;
    };
    let (mut backend_write, mut backend_read) = backend_ws.split();
    let (mut client_write, mut client_read) = client_ws.split();

    let client_to_backend = async {
        while let Some(Ok(msg)) = client_read.next().await {
            match msg {
                axum::extract::ws::Message::Text(t) => {
                    let _ = backend_write
                        .send(tokio_tungstenite::tungstenite::Message::Text(
                            t.as_str().into(),
                        ))
                        .await;
                }
                axum::extract::ws::Message::Binary(b) => {
                    let _ = backend_write
                        .send(tokio_tungstenite::tungstenite::Message::Binary(b))
                        .await;
                }
                axum::extract::ws::Message::Close(_) => {
                    let _ = backend_write
                        .send(tokio_tungstenite::tungstenite::Message::Close(None))
                        .await;
                    break;
                }
                _ => {}
            }
        }
    };

    let backend_to_client = async {
        while let Some(Ok(msg)) = backend_read.next().await {
            match msg {
                tokio_tungstenite::tungstenite::Message::Text(t) => {
                    if client_write
                        .send(axum::extract::ws::Message::Text(t.as_str().into()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                tokio_tungstenite::tungstenite::Message::Binary(b) => {
                    if client_write
                        .send(axum::extract::ws::Message::Binary(b))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                tokio_tungstenite::tungstenite::Message::Close(_) => {
                    let _ = client_write
                        .send(axum::extract::ws::Message::Close(None))
                        .await;
                    break;
                }
                _ => {}
            }
        }
    };

    tokio::select! {
        _ = client_to_backend => {},
        _ = backend_to_client => {},
    }
}

pub(super) async fn list_sessions_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(params): Query<SessionQueryParams>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(10).clamp(1, 100);
    let offset = (page - 1).saturating_mul(limit);

    let db = state.db.lock().await;
    let live_hosted = crate::remote::live_hosted_native_ids(&db);
    let (mut sessions, total_count) = db
        .list_sessions_filtered_with_hosts(
            params.agent.as_deref(),
            params.status.as_deref(),
            params.project.as_deref().filter(|value| !value.is_empty()),
            &live_hosted,
            offset,
            limit,
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let host = gnomcode_platform::hostname();
    for s in &mut sessions {
        s.title = crate::conversation_title(&db, &s.id).or(s.title.take());
        if s.device_name.is_none() {
            s.device_name = Some(host.clone());
        }
    }

    let (_, ongoing_count) = db
        .list_sessions_paginated_with_hosts(None, Some("ongoing"), &live_hosted, 0, 1)
        .unwrap_or_default();
    let (_, active_count) = db
        .list_sessions_paginated_with_hosts(None, Some("active"), &live_hosted, 0, 1)
        .unwrap_or_default();
    let (_, awaiting_approval_count) = db
        .list_sessions_paginated_with_hosts(None, Some("awaiting_approval"), &live_hosted, 0, 1)
        .unwrap_or_default();

    #[derive(Serialize)]
    struct PaginatedResponse {
        sessions: Vec<gnomcode_store::SessionListItem>,
        total_count: usize,
        page: usize,
        limit: usize,
        total_pages: usize,
        active_count: usize,
        ongoing_count: usize,
        awaiting_approval_count: usize,
    }

    let total_pages = total_count.div_ceil(limit);

    Ok(Json(PaginatedResponse {
        sessions,
        total_count,
        page,
        limit,
        total_pages: total_pages.max(1),
        active_count,
        ongoing_count,
        awaiting_approval_count,
    }))
}

#[derive(Debug, Deserialize, Default)]
pub(super) struct LiveQueryParams {
    range: Option<String>,
    from: Option<String>,
    to: Option<String>,
    agent: Option<String>,
    status: Option<String>,
    page: Option<usize>,
    limit: Option<usize>,
}

pub(super) async fn live_snapshot_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(params): Query<LiveQueryParams>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let range = params.range.as_deref().unwrap_or("all");
    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(10).clamp(1, 100);
    let offset = (page - 1).saturating_mul(limit);

    let db = state.db.lock().await;

    let report = db
        .generate_usage_report_timeline(range, params.from.as_deref(), params.to.as_deref())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let user_profile = db
        .get_user_profile()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let live_hosted = crate::remote::live_hosted_native_ids(&db);
    let (mut sessions, total_count) = db
        .list_sessions_paginated_with_hosts(
            params.agent.as_deref(),
            params.status.as_deref(),
            &live_hosted,
            offset,
            limit,
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let host = gnomcode_platform::hostname();
    for s in &mut sessions {
        if s.device_name.is_none() {
            s.device_name = Some(host.clone());
        }
    }

    let (_, ongoing_count) = db
        .list_sessions_paginated_with_hosts(None, Some("ongoing"), &live_hosted, 0, 1)
        .unwrap_or_default();
    let (_, active_count) = db
        .list_sessions_paginated_with_hosts(None, Some("active"), &live_hosted, 0, 1)
        .unwrap_or_default();

    let quota = db
        .get_latest_quota_snapshots()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let custom_pricing = db
        .get_custom_model_pricings()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut roi_by_provider = Vec::new();
    let mut combined_plan = 0.0;
    let get_spend = |prov_names: &[&str]| -> f64 {
        report
            .providers
            .iter()
            .find(|p| {
                prov_names
                    .iter()
                    .any(|name| p.provider.eq_ignore_ascii_case(name))
            })
            .map_or(0.0, |p| p.total_cost_usd)
    };

    let (time_factor, period_label): (f64, &str) = match range {
        "today" | "24h" => (1.0 / 30.0, "day"),
        "7d" => (7.0 / 30.0, "7d"),
        "30d" => (1.0, "mo"),
        _ => {
            if let (Some(early_s), Some(late_s)) = (&report.earliest_at, &report.latest_at) {
                if let (Ok(e_dt), Ok(l_dt)) = (
                    chrono::DateTime::parse_from_rfc3339(early_s),
                    chrono::DateTime::parse_from_rfc3339(late_s),
                ) {
                    let days = (l_dt - e_dt).num_days().max(1) as f64;
                    (days / 30.0, "period")
                } else {
                    (1.0, "period")
                }
            } else {
                (1.0, "period")
            }
        }
    };

    let claude_spend = get_spend(&["Claude Code", "Anthropic", "Anthropic (Claude)"]);
    let openai_spend = get_spend(&["OpenAI Codex", "OpenAI", "OpenAI (Codex)"]);
    let google_spend = get_spend(&["Google Antigravity", "Google", "Google (Antigravity)"]);

    if user_profile.claude_plan_usd > 0.0 && claude_spend > 0.0 {
        let eff_plan = user_profile.claude_plan_usd * time_factor;
        combined_plan += eff_plan;
        let roi = gnomcode_core::pricing::calculate_plan_roi(claude_spend, eff_plan);
        roi_by_provider.push(json!({
            "provider": "Claude Code",
            "plan_usd": eff_plan,
            "monthly_plan_usd": user_profile.claude_plan_usd,
            "spend_usd": claude_spend,
            "multiplier": roi.multiplier,
            "net_savings_usd": roi.net_savings_usd,
            "period_label": period_label,
        }));
    }

    if user_profile.openai_plan_usd > 0.0 && openai_spend > 0.0 {
        let eff_plan = user_profile.openai_plan_usd * time_factor;
        combined_plan += eff_plan;
        let roi = gnomcode_core::pricing::calculate_plan_roi(openai_spend, eff_plan);
        roi_by_provider.push(json!({
            "provider": "OpenAI Codex",
            "plan_usd": eff_plan,
            "monthly_plan_usd": user_profile.openai_plan_usd,
            "spend_usd": openai_spend,
            "multiplier": roi.multiplier,
            "net_savings_usd": roi.net_savings_usd,
            "period_label": period_label,
        }));
    }

    if user_profile.google_plan_usd > 0.0 && google_spend > 0.0 {
        let eff_plan = user_profile.google_plan_usd * time_factor;
        combined_plan += eff_plan;
        let roi = gnomcode_core::pricing::calculate_plan_roi(google_spend, eff_plan);
        roi_by_provider.push(json!({
            "provider": "Google Antigravity",
            "plan_usd": eff_plan,
            "monthly_plan_usd": user_profile.google_plan_usd,
            "spend_usd": google_spend,
            "multiplier": roi.multiplier,
            "net_savings_usd": roi.net_savings_usd,
            "period_label": period_label,
        }));
    }

    let combined_roi_calc =
        gnomcode_core::pricing::calculate_plan_roi(report.total_cost_usd, combined_plan);
    let combined_roi = json!({
        "provider": "All Plans (Combined)",
        "plan_usd": combined_plan,
        "monthly_plan_usd": user_profile.claude_plan_usd
            + user_profile.openai_plan_usd
            + user_profile.google_plan_usd,
        "spend_usd": report.total_cost_usd,
        "multiplier": combined_roi_calc.multiplier,
        "net_savings_usd": combined_roi_calc.net_savings_usd,
        "period_label": period_label,
    });

    let total_pages = total_count.div_ceil(limit);
    Ok((
        cors_headers(),
        Json(json!({
            "timeline": report,
            "user_profile": user_profile,
            "roi_by_provider": roi_by_provider,
            "combined_roi": combined_roi,
            "quota": quota,
            "sessions": sessions,
            "total_count": total_count,
            "page": page,
            "limit": limit,
            "total_pages": total_pages.max(1),
            "active_count": active_count,
            "ongoing_count": ongoing_count,
            "custom_pricing": custom_pricing,
        })),
    ))
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
    fn pty_query_token_does_not_bypass_host_or_origin_validation() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("host", "attacker.example".parse().expect("host"));
        assert_eq!(
            super::validate_pty_auth(&headers, Some("secret"), "secret"),
            Err(StatusCode::FORBIDDEN)
        );
        headers.insert("host", "127.0.0.1".parse().expect("host"));
        headers.insert(
            "origin",
            "https://attacker.example".parse().expect("origin"),
        );
        assert_eq!(
            super::validate_pty_auth(&headers, Some("secret"), "secret"),
            Err(StatusCode::FORBIDDEN)
        );
        headers.insert("origin", "http://127.0.0.1:3000".parse().expect("origin"));
        assert_eq!(
            super::validate_pty_auth(&headers, Some("secret"), "secret"),
            Ok(())
        );
        assert_eq!(
            super::validate_pty_auth(&headers, Some("wrong"), "secret"),
            Err(StatusCode::UNAUTHORIZED)
        );
    }

    #[tokio::test]
    async fn private_snapshot_and_pairing_require_authentication() {
        let db = Database::open_in_memory().expect("in-memory db");
        let app = build_router(AppState {
            db: Arc::new(Mutex::new(db)),
            bearer_token: "secret-test-token".to_owned(),
            orchestrator: None,
            remote: Arc::new(crate::remote::RemoteManager::new()),
        });
        for path in [
            "/api/v1/live",
            "/api/v1/projects",
            "/api/v1/auth/session",
            "/api/v1/supabase-config",
            "/api/v1/sync/status",
            "/api/v1/remote/devices",
            "/api/v1/reconcile",
        ] {
            let mut builder = Request::builder();
            if path == "/api/v1/reconcile" {
                builder = builder.method("POST");
            }
            let request = builder
                .uri(path)
                .header("Host", "127.0.0.1:41738")
                .body(Body::empty())
                .expect("request");
            let response = app.clone().oneshot(request).await.expect("response");
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{path}");
        }
    }

    #[tokio::test]
    async fn focus_requires_auth_and_never_claims_an_unknown_session_opened() {
        for (authorized, expected) in [
            (false, StatusCode::UNAUTHORIZED),
            (true, StatusCode::NOT_FOUND),
        ] {
            let app = build_router(AppState {
                db: Arc::new(Mutex::new(Database::open_in_memory().expect("database"))),
                bearer_token: "focus-unit".to_owned(),
                orchestrator: None,
                remote: Arc::new(crate::remote::RemoteManager::new()),
            });
            let mut request = Request::builder()
                .method("POST")
                .uri("/api/v1/sessions/missing/focus")
                .header("Host", "127.0.0.1");
            if authorized {
                request = request.header("Authorization", "Bearer focus-unit");
            }
            let response = app
                .oneshot(request.body(Body::empty()).expect("request"))
                .await
                .expect("response");
            assert_eq!(response.status(), expected);
        }
    }

    #[tokio::test]
    async fn unavailable_focus_returns_a_local_copy_path_without_claiming_success() {
        use gnomcode_core::{AgentId, Project, Session, SessionState, Surface};
        let mut db = Database::open_in_memory().expect("database");
        let project = Project::from_origin(None, "/workspace/focus-test", None);
        let session = Session {
            id: "codex:focus-unit".to_owned(),
            agent: AgentId::Codex,
            surface: Surface::Cli,
            project_id: project.id.clone(),
            parent_session_id: None,
            model: None,
            title: None,
            state: SessionState::Idle,
            pid: None,
            started_at: chrono::Utc::now(),
            last_event_at: chrono::Utc::now(),
        };
        let checkpoint = gnomcode_store::SourceCheckpoint {
            file_path: "/fixture".to_owned(),
            last_read_offset: 0,
            file_size: 0,
            fingerprint: "unit".to_owned(),
            last_ingested_at: chrono::Utc::now(),
        };
        db.commit_batch(&[project], &[session], &[], &[], &[], &checkpoint)
            .expect("fixture");
        let app = build_router(AppState {
            db: Arc::new(Mutex::new(db)),
            bearer_token: "focus-unit".to_owned(),
            orchestrator: None,
            remote: Arc::new(crate::remote::RemoteManager::new()),
        });
        let request = Request::builder()
            .method("POST")
            .uri("/api/v1/sessions/codex:focus-unit/focus")
            .header("Host", "127.0.0.1")
            .header("Authorization", "Bearer focus-unit")
            .body(Body::empty())
            .expect("request");
        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .expect("body");
        let value: serde_json::Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(value["focused"], false);
        assert_eq!(value["exact_session"], false);
        assert_eq!(value["repo_path"], "/workspace/focus-test");
    }

    #[tokio::test]
    async fn launch_endpoint_rejects_missing_directory_and_requires_auth() {
        let app = build_router(AppState {
            db: Arc::new(Mutex::new(Database::open_in_memory().expect("database"))),
            bearer_token: "launch-token".to_owned(),
            orchestrator: None,
            remote: Arc::new(crate::remote::RemoteManager::new()),
        });

        // 1. Missing auth
        let unauth = Request::builder()
            .method("POST")
            .uri("/api/v1/sessions/launch")
            .header("Host", "127.0.0.1")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"agent":"claude","repo_path":"."}"#))
            .expect("request");
        let res = app.clone().oneshot(unauth).await.expect("response");
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

        // 2. Non-existent directory
        let bad_dir = Request::builder()
            .method("POST")
            .uri("/api/v1/sessions/launch")
            .header("Host", "127.0.0.1")
            .header("Authorization", "Bearer launch-token")
            .header("Content-Type", "application/json")
            .body(Body::from(
                r#"{"agent":"claude","repo_path":"nonexistent_dir_12345"}"#,
            ))
            .expect("request");
        let res = app.oneshot(bad_dir).await.expect("response");
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn prompt_resolution_waits_for_database_instead_of_reporting_missing() {
        let db = Arc::new(Mutex::new(Database::open_in_memory().expect("database")));
        let guard = db.lock().await;
        let app = build_router(AppState {
            db: db.clone(),
            bearer_token: "prompt-test".into(),
            orchestrator: None,
            remote: Arc::new(crate::remote::RemoteManager::new()),
        });
        let request = Request::builder()
            .method("POST")
            .uri("/api/v1/sessions/missing/prompt")
            .header("Host", "127.0.0.1")
            .header("Authorization", "Bearer prompt-test")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"text":"Hello"}"#))
            .expect("request");
        let task = tokio::spawn(async move { app.oneshot(request).await.expect("response") });
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        assert!(
            !task.is_finished(),
            "database contention must not become a missing session"
        );
        drop(guard);
        assert_eq!(task.await.expect("task").status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn multiline_prompt_is_validated_before_host_lookup() {
        let app = build_router(AppState {
            db: Arc::new(Mutex::new(Database::open_in_memory().expect("database"))),
            bearer_token: "prompt-test".into(),
            orchestrator: None,
            remote: Arc::new(crate::remote::RemoteManager::new()),
        });
        let request = Request::builder()
            .method("POST")
            .uri("/api/v1/sessions/missing/prompt")
            .header("Host", "127.0.0.1")
            .header("Authorization", "Bearer prompt-test")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"text":"Review auth\nThen run tests"}"#))
            .expect("request");
        assert_eq!(
            app.oneshot(request).await.expect("response").status(),
            StatusCode::NOT_FOUND
        );
    }

    #[tokio::test]
    async fn hosted_and_prompt_endpoints_require_auth_and_validate_input() {
        let app = build_router(AppState {
            db: Arc::new(Mutex::new(Database::open_in_memory().expect("database"))),
            bearer_token: "prompt-token".to_owned(),
            orchestrator: None,
            remote: Arc::new(crate::remote::RemoteManager::new()),
        });

        let unauth = Request::builder()
            .method("GET")
            .uri("/api/v1/sessions/hosted")
            .header("Host", "127.0.0.1")
            .body(Body::empty())
            .expect("request");
        let res = app.clone().oneshot(unauth).await.expect("response");
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

        let auth = Request::builder()
            .method("GET")
            .uri("/api/v1/sessions/hosted")
            .header("Host", "127.0.0.1")
            .header("Authorization", "Bearer prompt-token")
            .body(Body::empty())
            .expect("request");
        let res = app.clone().oneshot(auth).await.expect("response");
        assert_eq!(res.status(), StatusCode::OK);

        let bad_prompt = Request::builder()
            .method("POST")
            .uri("/api/v1/sessions/nonexistent/prompt")
            .header("Host", "127.0.0.1")
            .header("Authorization", "Bearer prompt-token")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"text":"/exit"}"#))
            .expect("request");
        let res = app.clone().oneshot(bad_prompt).await.expect("response");
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        let unknown = Request::builder()
            .method("POST")
            .uri("/api/v1/sessions/nonexistent/prompt")
            .header("Host", "127.0.0.1")
            .header("Authorization", "Bearer prompt-token")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"text":"hello world"}"#))
            .expect("request");
        let res = app.clone().oneshot(unknown).await.expect("response");
        assert_eq!(res.status(), StatusCode::NOT_FOUND);

        let agents_req = Request::builder()
            .method("GET")
            .uri("/api/v1/agents/installed")
            .header("Host", "127.0.0.1")
            .header("Authorization", "Bearer prompt-token")
            .body(Body::empty())
            .expect("request");
        let res = app.oneshot(agents_req).await.expect("response");
        assert_eq!(res.status(), StatusCode::OK);
    }
}
