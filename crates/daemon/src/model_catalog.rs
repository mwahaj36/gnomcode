//! Bounded, on-demand model metadata from official CLI accounts. No inference requests.

use serde::Serialize;
use serde_json::Value;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

const LIMIT: u64 = 2 * 1024 * 1024;

/// One selectable model with its explicit host CLI.
#[derive(Clone, Debug, Serialize)]
pub struct ModelChoice {
    /// Official CLI used to run the model.
    pub agent: String,
    /// Exact model ID or native alias reported by that CLI.
    pub model: String,
    /// Native display label.
    pub display_name: String,
    /// Native catalog source, not a pricing registry or entitlement guarantee.
    pub availability_source: String,
}

/// Account/catalog state, including unavailable providers so omissions are explained.
#[derive(Clone, Debug, Serialize)]
pub struct ProviderCatalog {
    /// Official CLI identity.
    pub agent: String,
    /// Human-readable provider/CLI name.
    pub name: String,
    /// authenticated, configured, signed_out, unavailable, or unknown.
    pub status: String,
    /// Explanation of the native account observation or missing catalog.
    pub message: String,
    /// Choices reported for this CLI account.
    pub models: Vec<ModelChoice>,
}

impl ProviderCatalog {
    fn new(agent: &str, name: &str) -> Self {
        Self {
            agent: agent.into(),
            name: name.into(),
            status: "unavailable".into(),
            message: "CLI not found on the desktop process PATH. Restart Spawnt after installing it or changing PATH.".into(),
            models: Vec::new(),
        }
    }
}

fn valid_model(model: &str) -> bool {
    !model.is_empty()
        && model.len() <= 128
        && !model.starts_with('-')
        && model
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.:/[]".contains(&byte))
}

fn choices(agent: &str, source: &str, models: &Value) -> Vec<ModelChoice> {
    let mut seen = std::collections::HashSet::new();
    models
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|row| {
            let id = row
                .get("value")
                .or_else(|| row.get("modelId"))
                .or_else(|| row.get("id"))?
                .as_str()?;
            if !valid_model(id) || !seen.insert(id) {
                return None;
            }
            let label = row
                .get("displayName")
                .or_else(|| row.get("name"))
                .and_then(Value::as_str)
                .filter(|name| !name.trim().is_empty())
                .unwrap_or(id);
            Some(ModelChoice {
                agent: agent.into(),
                model: if id == "default" {
                    "@default".into()
                } else {
                    id.into()
                },
                display_name: spawnt_core::redact::redact_chat_content(label)
                    .chars()
                    .take(256)
                    .collect(),
                availability_source: source.into(),
            })
        })
        .collect()
}

async fn command_output(
    agent: &str,
    args: &[&str],
    cwd: &std::path::Path,
) -> Option<(bool, Vec<u8>)> {
    let mut command = tokio::process::Command::from(
        spawnt_platform::executables::background_agent_command(agent),
    );
    command
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    let mut child = command.spawn().ok()?;
    let mut bytes = Vec::new();
    child
        .stdout
        .take()?
        .take(LIMIT + 1)
        .read_to_end(&mut bytes)
        .await
        .ok()?;
    if bytes.len() > LIMIT as usize {
        return None;
    }
    let success = child.wait().await.ok()?.success();
    Some((success, bytes))
}

async fn claude_models(cwd: &std::path::Path) -> Option<Vec<ModelChoice>> {
    let mut command = tokio::process::Command::from(
        spawnt_platform::executables::background_agent_command("claude"),
    );
    command
        .args([
            "--print",
            "--input-format",
            "stream-json",
            "--output-format",
            "stream-json",
            "--verbose",
            "--no-session-persistence",
            "--settings",
            r#"{"disableAllHooks":true}"#,
            "--strict-mcp-config",
            "--mcp-config",
            r#"{"mcpServers":{}}"#,
            "--tools",
            "",
        ])
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    let mut child = command.spawn().ok()?;
    let mut input = child.stdin.take()?;
    input.write_all(b"{\"type\":\"control_request\",\"request_id\":\"spawnt-models\",\"request\":{\"subtype\":\"initialize\",\"hooks\":{},\"agents\":{},\"skills\":[]}}\n").await.ok()?;
    let mut output = BufReader::new(child.stdout.take()?.take(LIMIT));
    let mut line = String::new();
    while output.read_line(&mut line).await.ok()? > 0 {
        if let Ok(value) = serde_json::from_str::<Value>(&line) {
            let response = &value["response"];
            if value["type"] == "control_response" && response["request_id"] == "spawnt-models" {
                if response["subtype"] != "success" || !response["response"]["models"].is_array() {
                    return None;
                }
                let models = choices(
                    "claude",
                    "claude_cli_initialize",
                    &response["response"]["models"],
                );
                drop(input);
                let _ = child.start_kill();
                let _ = child.wait().await;
                return Some(models);
            }
        }
        line.clear();
    }
    None
}

fn gemini_account_configured(
    settings: &Value,
    accounts: &Value,
    api_key: bool,
    vertex: bool,
) -> bool {
    match settings
        .pointer("/security/auth/selectedType")
        .and_then(Value::as_str)
    {
        Some("oauth-personal") => accounts
            .get("active")
            .and_then(Value::as_str)
            .is_some_and(|account| !account.trim().is_empty()),
        Some("gemini-api-key") => api_key,
        Some("vertex-ai") => vertex,
        None => api_key || vertex,
        _ => false,
    }
}

fn read_metadata(path: &std::path::Path) -> Value {
    use std::io::Read as _;
    let Ok(file) = spawnt_platform::open_shared_read(path) else {
        return Value::Null;
    };
    let mut bytes = Vec::new();
    if file.take(LIMIT + 1).read_to_end(&mut bytes).is_err() || bytes.len() > LIMIT as usize {
        return Value::Null;
    }
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

async fn discover_provider(agent: &str, name: &str) -> ProviderCatalog {
    let mut result = ProviderCatalog::new(agent, name);
    let executable = if agent == "antigravity" { "agy" } else { agent };
    if !spawnt_platform::executables::executable_available(executable) {
        return result;
    }
    result.status = "unknown".into();
    result.message = "This CLI version did not expose a usable account/model catalog. Open the CLI to sign in or update it, then refresh.".into();
    let Ok(cwd) = tempfile::tempdir() else {
        return result;
    };
    let discovery = async {
        match agent {
            "codex" => {
                let (signed_in, _) =
                    command_output("codex", &["login", "status"], cwd.path()).await?;
                if !signed_in {
                    result.message = "Codex sign-in could not be confirmed. Check codex login status on your desktop, then sign in with codex login if needed.".into();
                    return Some(());
                }
                result.status = "authenticated".into();
                result.models = tokio::task::spawn_blocking(crate::api::available_models)
                    .await
                    .ok()?
                    .into_iter()
                    .map(|model| ModelChoice {
                        agent: model.agent,
                        model: model.model,
                        display_name: model.display_name,
                        availability_source: model.availability_source,
                    })
                    .collect();
                result.message=if result.models.is_empty() { "Signed in. Open Codex to refresh its model catalog, then try Refresh." } else { "Signed in to Codex. Models come from its fresh visible CLI catalog; access is checked by Codex when used." }.into();
            }
            "claude" => {
                let (success, bytes) =
                    command_output("claude", &["auth", "status"], cwd.path()).await?;
                let auth: Value = serde_json::from_slice(&bytes).ok()?;
                if auth.get("loggedIn").and_then(Value::as_bool) == Some(false) {
                    result.status = "signed_out".into();
                    result.message = "Sign in on your desktop with claude auth login.".into();
                    return Some(());
                }
                if !success || auth.get("loggedIn").and_then(Value::as_bool) != Some(true) {
                    return None;
                }
                result.models = claude_models(cwd.path()).await?;
                result.status = "authenticated".into();
                result.message="Signed in to Claude Code. Models and aliases come from its native model selector.".into();
            }
            "gemini" => {
                let home = std::env::var_os("GEMINI_HOME")
                    .filter(|path| !path.is_empty())
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|| {
                        spawnt_platform::user_home_dir().join(obfstr::obfstr!(".gemini"))
                    });
                let api_key = std::env::var_os("GEMINI_API_KEY").is_some();
                let vertex = std::env::var_os("GOOGLE_CLOUD_PROJECT").is_some();
                let configured = tokio::task::spawn_blocking(move || {
                    gemini_account_configured(
                        &read_metadata(&home.join(obfstr::obfstr!("settings.json"))),
                        &read_metadata(&home.join(obfstr::obfstr!("google_accounts.json"))),
                        api_key,
                        vertex,
                    )
                })
                .await
                .ok()?;
                if !configured {
                    result.message = "Gemini sign-in could not be confirmed. Open Gemini CLI on your desktop to select a signed-in account, then refresh.".into();
                    return Some(());
                }
                let (success, bytes) = command_output(
                    "gemini",
                    &["models", "list", "--output-format", "json"],
                    cwd.path(),
                )
                .await?;
                if !success {
                    return None;
                }
                let listing = serde_json::from_slice::<Value>(&bytes).ok()?;
                if !listing.is_array() {
                    return None;
                }
                result.models = choices("gemini", "gemini_cli_models", &listing);
                result.status = "configured".into();
                result.message="Account configured in Gemini CLI. Its native model list is shown; current access is checked by Gemini when used.".into();
            }
            _ => {
                result.message="Antigravity is installed. Spawnt has no supported read-only catalog source for this CLI yet; use its native model selector.".into();
            }
        }
        Some(())
    };
    let _ = tokio::time::timeout(Duration::from_secs(8), discovery).await;
    if matches!(result.status.as_str(), "authenticated" | "configured")
        && !result.models.iter().any(|model| model.model == "@default")
    {
        result.models.insert(
            0,
            ModelChoice {
                agent: agent.into(),
                model: "@default".into(),
                display_name: "Use CLI default".into(),
                availability_source: "native_cli_default".into(),
            },
        );
    }
    result
}

/// Discovers all supported CLI providers concurrently, without prompts or credential-file reads.
pub async fn discover() -> Vec<ProviderCatalog> {
    let (codex, claude, gemini, antigravity) = tokio::join!(
        discover_provider("codex", "OpenAI Codex"),
        discover_provider("claude", "Claude Code"),
        discover_provider("gemini", "Gemini CLI"),
        discover_provider("antigravity", "Antigravity")
    );
    vec![codex, claude, gemini, antigravity]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn native_choices_are_safe_deduplicated_and_never_include_account_fields() {
        let models = choices(
            "claude",
            "native",
            &json!([{"value":"sonnet","displayName":"Sonnet","token":"private"},{"value":"sonnet"},{"value":"--yolo"},{"value":"opus[1m]","displayName":"Opus long"}]),
        );
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].model, "sonnet");
        assert_eq!(models[1].model, "opus[1m]");
        assert!(
            !serde_json::to_string(&models)
                .expect("serialize")
                .contains("private")
        );
    }
    #[test]
    fn gemini_old_accounts_and_installation_markers_are_not_login_evidence() {
        let settings = json!({"security":{"auth":{"selectedType":"oauth-personal"}}});
        assert!(!gemini_account_configured(
            &settings,
            &json!({"active":null,"old":["old@example.invalid"]}),
            false,
            false
        ));
        assert!(gemini_account_configured(
            &settings,
            &json!({"active":"test@example.invalid"}),
            false,
            false
        ));
        assert!(!gemini_account_configured(
            &Value::Null,
            &json!({"active":"test@example.invalid"}),
            false,
            false
        ));
    }
}
