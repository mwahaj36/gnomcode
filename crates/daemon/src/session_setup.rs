//! Setup screens of hosted agents (`SPEC §3.10`): reports when a launched agent waits on a
//! folder-trust question or another setup screen, and confirms folder trust for one explicit
//! click from the console or a paired phone.
//!
//! Nothing here answers an agent on Gnomcode's own initiative, and nothing but the
//! folder-trust question can be answered from here: a login, key or update screen is only
//! reported, so the person opens the agent terminal for it.

use crate::api::{AppState, validate_auth};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};
use gnomcode_pty::SetupBlocker;
use gnomcode_store::Database;
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::sync::Mutex;

fn agent_name(agent: &str) -> &'static str {
    match agent {
        "claude" | "claude-code" => "Claude Code",
        "codex" => "Codex",
        "antigravity" | "agy" => "Antigravity",
        _ => "The agent",
    }
}

/// The live host that owns a conversation, if Gnomcode launched one for it.
async fn host_of(db: &Arc<Mutex<Database>>, id: &str) -> Option<gnomcode_pty::SessionRecord> {
    let database = Arc::clone(db);
    let id = id.to_owned();
    tokio::task::spawn_blocking(move || {
        let guard = database.blocking_lock();
        let sessions = guard.list_active_sessions(usize::MAX).ok()?;
        let hosted = gnomcode_pty::list_active_sessions();
        crate::remote::conversation_host(&guard, &id, &sessions, &hosted)
            .ok()?
            .cloned()
    })
    .await
    .ok()
    .flatten()
}

/// What the conversation's hosted agent is waiting on: `blocker` is `trust_folder`, `setup`
/// or null. An unknown conversation or unreachable host reports null, never a question.
pub(crate) async fn status(db: &Arc<Mutex<Database>>, id: &str) -> Value {
    let Some(record) = host_of(db, id).await else {
        return json!({"type": "setup", "session_id": id, "blocker": null});
    };
    let mut blocker = gnomcode_pty::setup_blocker(&record).await;
    if blocker == Some(SetupBlocker::TrustFolder)
        && crate::approvals::project_trusted(db, id, &record.agent, "TrustFolder").await
        && gnomcode_pty::confirm_trust(&record).await.unwrap_or(false)
    {
        blocker = None;
    }
    json!({
        "type": "setup",
        "session_id": id,
        "blocker": blocker,
        "agent": agent_name(&record.agent),
        "folder": record.repo_path,
        "limit_message": gnomcode_pty::limit_message(&record).await,
        // Only a protocol host reports this; null means "ask the transcript", as before.
        "busy": gnomcode_pty::turn_busy(&record).await,
    })
}

/// Confirms the folder-trust question the person was shown. `Ok(false)` when that question
/// is no longer on the agent's screen; `Err` when the host cannot be reached.
pub(crate) async fn trust_folder(db: &Arc<Mutex<Database>>, id: &str) -> Result<bool, ()> {
    let record = host_of(db, id).await.ok_or(())?;
    if gnomcode_pty::setup_blocker(&record).await != Some(SetupBlocker::TrustFolder) {
        return Ok(false);
    }
    let confirmed = gnomcode_pty::confirm_trust(&record).await.map_err(|_| ())?;
    if confirmed {
        tracing::info!(agent = %record.agent, "folder trust confirmed by the user");
    }
    Ok(confirmed)
}

/// `GET /api/v1/sessions/{id}/setup`.
pub(crate) async fn status_handler(
    headers: HeaderMap,
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    Ok(Json(status(&state.db, &id).await))
}

/// `POST /api/v1/sessions/{id}/trust-folder`: `409` when no trust question is showing.
pub(crate) async fn trust_handler(
    headers: HeaderMap,
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    match trust_folder(&state.db, &id).await {
        Ok(true) => Ok(Json(json!({"ok": true, "session_id": id}))),
        Ok(false) => Err(StatusCode::CONFLICT),
        Err(()) => Err(StatusCode::NOT_FOUND),
    }
}

/// Encrypted Remote requests `setup` (status) and `trust_folder` (confirm).
pub(crate) async fn remote_command(db: &Arc<Mutex<Database>>, request: &Value) -> Value {
    let id = request
        .get("session_id")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if id.is_empty() || id.len() > 256 || id.chars().any(char::is_control) {
        return json!({"type": "error", "message": "Choose a conversation first."});
    }
    if request.get("type").and_then(Value::as_str) == Some("setup") {
        return status(db, id).await;
    }
    match trust_folder(db, id).await {
        Ok(true) => json!({"type": "trust_folder_ack", "session_id": id}),
        Ok(false) => json!({
            "type": "error",
            "message": "The agent is no longer asking to trust this folder.",
        }),
        Err(()) => json!({
            "type": "error",
            "message": "Could not reach the agent. Open its terminal on your laptop.",
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn an_unknown_conversation_reports_no_blocker_and_cannot_be_trusted() {
        let db = Arc::new(Mutex::new(Database::open_in_memory().expect("database")));
        let status = status(&db, "codex:missing").await;
        assert_eq!(status["blocker"], Value::Null);
        assert_eq!(trust_folder(&db, "codex:missing").await, Err(()));
        let reply = remote_command(&db, &json!({"type": "trust_folder", "session_id": ""})).await;
        assert_eq!(reply["type"], "error");
    }
}
