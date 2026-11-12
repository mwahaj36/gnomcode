//! Opt-in exact limits from provider APIs, using the agent's own local sign-in (ADR 0029).
//!
//! Tokens are read at fetch time, sent only to their provider (or loopback), and never
//! logged, stored, refreshed or rewritten. Disabled unless the user runs `gnomcode limits enable`.

use gnomcode_core::QuotaSnapshot;
use gnomcode_store::{Database, SourceCheckpoint};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// Providers that can be opted in to API-based limits.
pub const PROVIDERS: [&str; 2] = ["claude", "antigravity"];
const POLL: Duration = Duration::from_secs(300);

fn consent_path() -> PathBuf {
    gnomcode_platform::gnomcode_home().join("vendor-limits.json")
}

/// Returns whether the user opted in to API-based limits for `provider`.
#[must_use]
pub fn enabled(provider: &str) -> bool {
    std::fs::read(consent_path())
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .is_some_and(|value| value.get(provider) == Some(&serde_json::Value::Bool(true)))
}

/// Records the user's opt-in choice for `provider`.
///
/// # Errors
/// Returns an I/O error if the consent file cannot be written.
pub fn set_enabled(provider: &str, on: bool) -> std::io::Result<()> {
    let mut value = std::fs::read(consent_path())
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .filter(serde_json::Value::is_object)
        .unwrap_or_else(|| serde_json::json!({}));
    value[provider] = serde_json::Value::Bool(on);
    std::fs::create_dir_all(gnomcode_platform::gnomcode_home())?;
    std::fs::write(consent_path(), value.to_string())
}

/// Why each provider's last poll produced no reading; cleared by the next good one.
static ISSUES: std::sync::Mutex<std::collections::BTreeMap<String, String>> =
    std::sync::Mutex::new(std::collections::BTreeMap::new());

/// Quota rows name Claude `claude-code`; the opt-in name is `claude`.
fn issue_key(provider: &str) -> &str {
    if provider == "claude-code" {
        "claude"
    } else {
        provider
    }
}

pub(crate) fn record_issue(provider: &str, reason: Option<String>) {
    if let Ok(mut issues) = ISSUES.lock() {
        match reason {
            Some(reason) => issues.insert(issue_key(provider).to_owned(), reason),
            None => issues.remove(issue_key(provider)),
        };
    }
}

/// Returns why the opt-in source for quota provider `provider` is not producing fresh readings.
///
/// The text is a fixed user-facing reason, never a credential or response body.
#[must_use]
pub fn issue(provider: &str) -> Option<String> {
    ISSUES.lock().ok()?.get(issue_key(provider)).cloned()
}

/// Stores a poll's rows, or remembers why there are none so the API can say the reading is stale.
async fn record_outcome(
    db: &Arc<Mutex<Database>>,
    provider: &str,
    outcome: Result<Vec<QuotaSnapshot>, String>,
) {
    let quota = match outcome {
        Ok(quota) => quota,
        Err(reason) => {
            tracing::warn!("{provider} limits unavailable: {reason}");
            record_issue(provider, Some(reason));
            return;
        }
    };
    let now = chrono::Utc::now();
    let checkpoint = SourceCheckpoint {
        file_path: format!("vendor:{provider}"),
        last_read_offset: 0,
        file_size: 0,
        fingerprint: now.to_rfc3339(),
        last_ingested_at: now,
    };
    let stored = db
        .lock()
        .await
        .commit_batch(&[], &[], &[], &[], &quota, &checkpoint);
    if stored.is_err() {
        tracing::warn!("{provider} limits could not be stored");
        record_issue(provider, Some("Limits could not be stored".to_owned()));
    } else {
        record_issue(provider, None);
    }
}

fn claude_access_token() -> Result<String, String> {
    let dir = std::env::var_os("CLAUDE_CONFIG_DIR")
        .filter(|dir| !dir.is_empty())
        .map_or_else(
            || gnomcode_platform::user_home_dir().join(".claude"),
            PathBuf::from,
        );
    claude_access_token_in(&dir, chrono::Utc::now().timestamp_millis())
}

fn claude_access_token_in(dir: &std::path::Path, now_ms: i64) -> Result<String, String> {
    let file =
        gnomcode_platform::read_from_offset(&dir.join(obfstr::obfstr!(".credentials.json")), 0)
            .map_err(|_| "Claude sign-in not found; sign in to Claude Code".to_owned())?
            .bytes;
    let oauth = serde_json::from_slice::<serde_json::Value>(&file)
        .ok()
        .and_then(|value| value.get("claudeAiOauth").cloned())
        .ok_or("Claude sign-in has no OAuth login (API-key logins have no plan limits)")?;
    let expires_ms = oauth
        .get("expiresAt")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0);
    // Never refresh: Claude Code rotates refresh tokens, so refreshing here could sign it out.
    // The Claude desktop app signs in elsewhere and never rewrites this file, so only the CLI refreshes it.
    if expires_ms <= now_ms {
        return Err("Claude sign-in expired; run `claude` in a terminal to refresh it".into());
    }
    oauth
        .get("accessToken")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| "Claude sign-in has no access token".into())
}

async fn fetch_claude(client: &reqwest::Client) -> Result<Vec<QuotaSnapshot>, String> {
    let token = claude_access_token()?;
    let response = client
        .get(obfstr::obfstr!("https://api.anthropic.com/api/oauth/usage").to_owned())
        .bearer_auth(token)
        .header("anthropic-beta", obfstr::obfstr!("oauth-2025-04-20"))
        .header("Accept", "application/json")
        .header(
            "User-Agent",
            concat!("gnomcode/", env!("CARGO_PKG_VERSION")),
        )
        .send()
        .await
        .map_err(|error| format!("Claude limits request failed: {error}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "Claude limits request returned HTTP {}",
            status.as_u16()
        ));
    }
    let body = response
        .json::<serde_json::Value>()
        .await
        .map_err(|_| "Claude limits response was not JSON".to_owned())?;
    Ok(gnomcode_adapters::claude::parse_oauth_usage(
        &body,
        chrono::Utc::now(),
    ))
}

/// Asks a running Antigravity language server (loopback only) for per-model quota.
async fn fetch_antigravity() -> Result<Vec<QuotaSnapshot>, String> {
    let gemini = std::env::var_os("GEMINI_HOME")
        .filter(|dir| !dir.is_empty())
        .map_or_else(
            || gnomcode_platform::user_home_dir().join(".gemini"),
            PathBuf::from,
        );
    let mut hubs: Vec<_> = std::fs::read_dir(gemini.join("antigravity").join("daemon"))
        .map_err(|_| "Antigravity is not installed or has never run".to_owned())?
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("hub_"))
        .filter_map(|entry| Some((entry.metadata().ok()?.modified().ok()?, entry.path())))
        .collect();
    hubs.sort_by_key(|hub| std::cmp::Reverse(hub.0));
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(4))
        .build()
        .map_err(|error| error.to_string())?;
    for (_, path) in hubs.into_iter().take(3) {
        let Some(hub) = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        else {
            continue;
        };
        let (Some(port), Some(csrf)) = (
            hub.get("httpPort").and_then(serde_json::Value::as_u64),
            hub.get("csrfToken").and_then(serde_json::Value::as_str),
        ) else {
            continue;
        };
        // Prefer the pooled summary (5-hour and weekly per group); older servers only report per model.
        for method in ["RetrieveUserQuotaSummary", "GetUserStatus"] {
            let request = client
                .post(format!(
                    "http://127.0.0.1:{port}{}{method}",
                    obfstr::obfstr!("/exa.language_server_pb.LanguageServerService/")
                ))
                .header("Connect-Protocol-Version", "1")
                .header(obfstr::obfstr!("X-Codeium-Csrf-Token"), csrf)
                .json(&serde_json::json!({
                    "metadata": {"ideName": "antigravity", "extensionName": "antigravity", "locale": "en"}
                }));
            let Ok(reply) = request.send().await else {
                continue;
            };
            let Ok(body) = reply.json::<serde_json::Value>().await else {
                continue;
            };
            let now = chrono::Utc::now();
            let quota = if method == "GetUserStatus" {
                gnomcode_adapters::antigravity::parse_user_status(&body, now)
            } else {
                gnomcode_adapters::antigravity::parse_quota_summary(&body, now)
            };
            if !quota.is_empty() {
                return Ok(quota);
            }
        }
    }
    Err("No running Antigravity agent; limits appear while one is running".into())
}

fn https_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .https_only(true)
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|error| error.to_string())
}

/// Fetches `provider`'s limits once, returning the rows or why they are unavailable.
///
/// # Errors
/// Returns a user-facing reason (signed out, expired, HTTP status, not running).
pub async fn check(provider: &str) -> Result<Vec<QuotaSnapshot>, String> {
    match provider {
        "claude" => fetch_claude(&https_client()?).await,
        "antigravity" => fetch_antigravity().await,
        _ => Err(format!("unsupported provider {provider}")),
    }
}

/// Polls opted-in providers every five minutes and stores their quota rows.
pub fn spawn(db: Arc<Mutex<Database>>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(POLL);
        loop {
            ticker.tick().await;
            for provider in PROVIDERS {
                if !enabled(provider) {
                    record_issue(provider, None);
                    continue;
                }
                record_outcome(&db, provider, check(provider).await).await;
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use gnomcode_core::{QuotaWindow, TrustLevel};

    fn reading(provider: &str) -> QuotaSnapshot {
        QuotaSnapshot {
            provider: provider.to_owned(),
            limit_id: "unit".to_owned(),
            window: QuotaWindow::Weekly,
            used_pct: 2.0,
            resets_at: None,
            plan: None,
            basis: TrustLevel::Exact,
            observed_at: chrono::Utc::now(),
        }
    }

    #[tokio::test]
    async fn a_failed_poll_is_reported_until_the_next_good_reading() {
        let db = Arc::new(Mutex::new(Database::open_in_memory().expect("database")));
        record_outcome(&db, "unit-test", Err("sign-in expired".to_owned())).await;
        assert_eq!(issue("unit-test").as_deref(), Some("sign-in expired"));
        assert!(
            db.lock()
                .await
                .get_latest_quota_snapshots()
                .expect("rows")
                .is_empty()
        );
        record_outcome(&db, "unit-test", Ok(vec![reading("unit-test")])).await;
        assert_eq!(issue("unit-test"), None);
        assert_eq!(
            db.lock()
                .await
                .get_latest_quota_snapshots()
                .expect("rows")
                .len(),
            1
        );
    }

    #[test]
    fn issues_are_looked_up_by_quota_provider_id() {
        record_issue("claude", Some("expired".to_owned()));
        assert_eq!(issue("claude-code").as_deref(), Some("expired"));
        record_issue("claude", None);
        assert_eq!(issue("claude-code"), None);
    }

    #[test]
    fn an_expired_sign_in_names_the_fix_and_a_live_one_is_used() {
        let dir = tempfile::tempdir().expect("temp dir");
        let write = |expires_at: i64| {
            std::fs::write(
                dir.path().join(".credentials.json"),
                serde_json::json!({"claudeAiOauth": {"accessToken": "test-token", "expiresAt": expires_at}})
                    .to_string(),
            )
            .expect("credentials");
        };
        write(1_000);
        assert_eq!(
            claude_access_token_in(dir.path(), 2_000),
            Err("Claude sign-in expired; run `claude` in a terminal to refresh it".to_owned())
        );
        write(3_000);
        assert_eq!(
            claude_access_token_in(dir.path(), 2_000),
            Ok("test-token".to_owned())
        );
    }
}
