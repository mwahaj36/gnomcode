//! Local-only storage and validation for canned quick-action prompts (never raw argv).

use crate::api::{AppState, validate_auth};
use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::PathBuf;

const MAX_ACTIONS: usize = 20;
const MAX_ID_LEN: usize = 64;
const MAX_LABEL_LEN: usize = 48;
const MAX_PROMPT_UTF16: usize = 4000;

/// A canned natural-language quick prompt (never shell commands or raw argv).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct QuickAction {
    /// Stable identifier for the quick action.
    pub id: String,
    /// Short button label shown in the UI.
    pub label: String,
    /// Natural-language prompt text sent through the normal prompt path.
    pub prompt: String,
}

/// Request payload for replacing saved quick actions.
#[derive(Debug, Deserialize)]
pub(crate) struct SaveQuickActionsRequest {
    /// Validated list of canned quick actions.
    pub actions: Vec<QuickAction>,
}

fn default_actions() -> Vec<QuickAction> {
    vec![
        QuickAction {
            id: "continue".to_owned(),
            label: "Continue".to_owned(),
            prompt: "Continue from where you left off.".to_owned(),
        },
        QuickAction {
            id: "run-tests".to_owned(),
            label: "Run tests".to_owned(),
            prompt: "Run the relevant tests and summarize any failures.".to_owned(),
        },
        QuickAction {
            id: "summarize-diff".to_owned(),
            label: "Summarize changes".to_owned(),
            prompt: "Summarize the current workspace changes and any remaining risks.".to_owned(),
        },
    ]
}

fn storage_path() -> PathBuf {
    gnomcode_platform::gnomcode_home().join("quick-actions.json")
}

fn validate_action(action: &QuickAction) -> Result<QuickAction, &'static str> {
    let id = action.id.trim();
    if id.is_empty()
        || id.len() > MAX_ID_LEN
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err("Invalid quick action id");
    }
    let label = action.label.trim();
    if label.is_empty()
        || label.chars().count() > MAX_LABEL_LEN
        || label.chars().any(char::is_control)
    {
        return Err("Invalid quick action label");
    }
    let prompt = action.prompt.trim();
    if prompt.is_empty()
        || prompt.encode_utf16().count() > MAX_PROMPT_UTF16
        || prompt
            .chars()
            .any(|ch| ch.is_control() && !matches!(ch, '\n' | '\t'))
        || prompt.trim_start().starts_with('/')
    {
        return Err("Invalid quick action prompt");
    }
    Ok(QuickAction {
        id: id.to_owned(),
        label: label.to_owned(),
        prompt: prompt.to_owned(),
    })
}

fn validate_list(actions: &[QuickAction]) -> Result<Vec<QuickAction>, &'static str> {
    if actions.len() > MAX_ACTIONS {
        return Err("Too many quick actions (maximum 20)");
    }
    let mut seen = std::collections::HashSet::new();
    let mut clean = Vec::with_capacity(actions.len());
    for action in actions {
        let valid = validate_action(action)?;
        if !seen.insert(valid.id.clone()) {
            return Err("Duplicate quick action id");
        }
        clean.push(valid);
    }
    Ok(clean)
}

fn load_actions() -> Vec<QuickAction> {
    let path = storage_path();
    let Ok(bytes) = std::fs::read(&path) else {
        return default_actions();
    };
    serde_json::from_slice::<Vec<QuickAction>>(&bytes)
        .ok()
        .and_then(|list| validate_list(&list).ok())
        .unwrap_or_else(default_actions)
}

fn save_actions(actions: &[QuickAction]) -> Result<Vec<QuickAction>, &'static str> {
    let clean = validate_list(actions)?;
    let path = storage_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|_| "Could not create quick actions directory")?;
    }
    let encoded =
        serde_json::to_vec_pretty(&clean).map_err(|_| "Could not serialize quick actions")?;
    gnomcode_platform::write_private_file(&path, &encoded)
        .map_err(|_| "Could not save quick actions")?;
    Ok(clean)
}

/// Returns saved canned quick actions.
pub(crate) async fn get_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let actions = tokio::task::spawn_blocking(load_actions)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({ "actions": actions })))
}

/// Replaces saved canned quick actions after validation.
pub(crate) async fn put_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<SaveQuickActionsRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let saved = tokio::task::spawn_blocking(move || save_actions(&payload.actions))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(Json(json!({ "actions": saved })))
}

/// Handles E2EE Remote `quick_actions` get/set requests.
pub(crate) async fn remote_command(request: &Value) -> Value {
    let action = request
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("get");
    match action {
        "get" => match tokio::task::spawn_blocking(load_actions).await {
            Ok(actions) => json!({ "type": "quick_actions", "actions": actions }),
            Err(_) => json!({ "type": "error", "message": "Could not load quick actions" }),
        },
        "set" => {
            let Some(raw) = request.get("actions").cloned() else {
                return json!({ "type": "error", "message": "Missing quick actions list" });
            };
            let Ok(parsed) = serde_json::from_value::<Vec<QuickAction>>(raw) else {
                return json!({ "type": "error", "message": "Invalid quick actions payload" });
            };
            match tokio::task::spawn_blocking(move || save_actions(&parsed)).await {
                Ok(Ok(saved)) => json!({ "type": "quick_actions", "actions": saved }),
                Ok(Err(message)) => json!({ "type": "error", "message": message }),
                Err(_) => json!({ "type": "error", "message": "Could not save quick actions" }),
            }
        }
        _ => json!({ "type": "error", "message": "Unsupported quick_actions operation" }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn quick_actions_roundtrip_and_reject_commands_or_unknown_fields() {
        const CHILD: &str = "GNOMCODE_TEST_QUICK_ACTIONS_HOME";
        if let Some(home) = std::env::var_os(CHILD) {
            let _ = home;
            let initial = remote_command(&json!({"type": "quick_actions"})).await;
            assert_eq!(initial["type"], "quick_actions");
            assert!(!initial["actions"].as_array().expect("defaults").is_empty());

            let saved = remote_command(&json!({
                "type": "quick_actions",
                "action": "set",
                "actions": [{"id": "check", "label": "Check", "prompt": "Run cargo test"}]
            }))
            .await;
            assert_eq!(saved["type"], "quick_actions");
            assert_eq!(saved["actions"][0]["id"], "check");

            let reloaded = remote_command(&json!({"type": "quick_actions"})).await;
            assert_eq!(reloaded["actions"][0]["prompt"], "Run cargo test");

            let slash_rejected = remote_command(&json!({
                "type": "quick_actions",
                "action": "set",
                "actions": [{"id": "bad", "label": "Bad", "prompt": "/compact"}]
            }))
            .await;
            assert_eq!(slash_rejected["type"], "error");

            let argv_rejected = remote_command(&json!({
                "type": "quick_actions",
                "action": "set",
                "actions": [{"id": "bad", "label": "Bad", "prompt": "hi", "argv": ["rm"]}]
            }))
            .await;
            assert_eq!(argv_rejected["type"], "error");
            return;
        }
        let home = tempfile::tempdir().expect("home");
        let status = std::process::Command::new(std::env::current_exe().expect("test binary"))
            .args([
                "--exact",
                "quick_actions::tests::quick_actions_roundtrip_and_reject_commands_or_unknown_fields",
            ])
            .env(CHILD, home.path())
            .env("GNOMCODE_HOME", home.path())
            .status()
            .expect("isolated test");
        assert!(status.success());
    }
}
