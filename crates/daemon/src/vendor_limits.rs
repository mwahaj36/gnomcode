//! Opt-in exact limits from provider APIs, using the agent's own local sign-in (ADR 0029).
//!
//! Tokens are read at fetch time, sent only to their provider (or loopback), and never
//! logged, stored or rewritten by Gnomcode. Only the official `claude` CLI refreshes the Claude
//! sign-in (ADR 0030). Disabled unless the user runs `gnomcode limits enable`.

use gnomcode_core::QuotaSnapshot;
use gnomcode_store::{Database, SourceCheckpoint};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// Providers that can be opted in to API-based limits.
pub const PROVIDERS: [&str; 2] = ["claude", "antigravity"];
const POLL: Duration = Duration::from_secs(300);

fn consent_path() -> PathBuf {
    gnomcode_platform::gnomcode_home().join("vendor-limits.json")
}

fn read_consent(path: &Path) -> Option<serde_json::Value> {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
}

fn enabled_in(path: &Path, provider: &str) -> bool {
    read_consent(path)
        .is_some_and(|value| value.get(provider) == Some(&serde_json::Value::Bool(true)))
}

fn set_flag_in(path: &Path, key: &str, on: bool) -> std::io::Result<()> {
    let mut value = read_consent(path)
        .filter(serde_json::Value::is_object)
        .unwrap_or_else(|| serde_json::json!({}));
    value[key] = serde_json::Value::Bool(on);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, value.to_string())
}

/// Returns whether the user opted in to API-based limits for `provider`.
#[must_use]
pub fn enabled(provider: &str) -> bool {
    enabled_in(&consent_path(), provider)
}

/// Records the user's opt-in choice for `provider`.
///
/// # Errors
/// Returns an I/O error if the consent file cannot be written.
pub fn set_enabled(provider: &str, on: bool) -> std::io::Result<()> {
    set_flag_in(&consent_path(), provider, on)
}

fn refresh_enabled_in(path: &Path, provider: &str) -> bool {
    provider == "claude"
        && enabled_in(path, provider)
        && read_consent(path)
            .and_then(|value| value.get("claude_refresh").cloned())
            .is_none_or(|flag| flag != serde_json::Value::Bool(false))
}

/// Returns whether Gnomcode may run the official `claude` CLI to refresh an expiring sign-in.
///
/// On by default once Claude limits are enabled; `gnomcode limits enable claude --no-refresh` turns it off.
#[must_use]
pub fn refresh_enabled(provider: &str) -> bool {
    refresh_enabled_in(&consent_path(), provider)
}

/// Records whether Gnomcode may refresh the Claude sign-in through the official CLI.
///
/// # Errors
/// Returns an I/O error if the consent file cannot be written.
pub fn set_refresh_enabled(on: bool) -> std::io::Result<()> {
    set_flag_in(&consent_path(), "claude_refresh", on)
}

/// Refresh when the token expires within this window.
const REFRESH_WINDOW_MS: i64 = 30 * 60 * 1000;
const REFRESH_EVERY: Duration = Duration::from_secs(15 * 60);
const REFRESH_TIMEOUT: Duration = Duration::from_secs(30);

/// Keeps the Claude sign-in fresh by running the official CLI's `auth status`, which refreshes
/// `.credentials.json` itself without sending a model request (ADR 0030).
#[derive(Default)]
struct Refresher {
    last_attempt: Option<std::time::Instant>,
}

impl Refresher {
    /// Runs the CLI when the token is expired or expires within 30 minutes, at most once per
    /// 15 minutes. Returns whether the CLI ran and exited successfully. Any failure is only
    /// logged without output contents, so the stale-reading issue stays visible.
    async fn refresh_if_due(
        &mut self,
        dir: &Path,
        now_ms: i64,
        now: std::time::Instant,
        command: impl FnOnce() -> std::process::Command,
        timeout: Duration,
    ) -> bool {
        let Some(expires_ms) = claude_oauth_in(dir).ok().map(|oauth| {
            oauth
                .get("expiresAt")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0)
        }) else {
            return false;
        };
        if expires_ms - now_ms > REFRESH_WINDOW_MS {
            return false;
        }
        if self
            .last_attempt
            .is_some_and(|last| now.saturating_duration_since(last) < REFRESH_EVERY)
        {
            return false;
        }
        self.last_attempt = Some(now);
        let command = command();
        match tokio::task::spawn_blocking(move || run_hidden(command, timeout)).await {
            Ok(Ok(())) => true,
            Ok(Err(reason)) => {
                tracing::warn!("Claude sign-in refresh failed: {reason}");
                false
            }
            Err(_) => false,
        }
    }
}

/// Runs `command` with no input and discarded output, killing it at `timeout`.
fn run_hidden(mut command: std::process::Command, timeout: Duration) -> Result<(), &'static str> {
    use std::process::Stdio;
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "the CLI could not be started")?;
    let deadline = std::time::Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(_)) => return Err("the CLI exited with an error"),
            Ok(None) if std::time::Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("the CLI timed out");
            }
            Err(_) => return Err("the CLI could not be monitored"),
        }
    }
}

fn claude_config_dir() -> PathBuf {
    std::env::var_os("CLAUDE_CONFIG_DIR")
        .filter(|dir| !dir.is_empty())
        .map_or_else(
            || gnomcode_platform::user_home_dir().join(".claude"),
            PathBuf::from,
        )
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
    claude_access_token_in(&claude_config_dir(), chrono::Utc::now().timestamp_millis())
}

fn claude_oauth_in(dir: &Path) -> Result<serde_json::Value, String> {
    let file =
        gnomcode_platform::read_from_offset(&dir.join(obfstr::obfstr!(".credentials.json")), 0)
            .map_err(|_| "Claude sign-in not found; sign in to Claude Code".to_owned())?
            .bytes;
    serde_json::from_slice::<serde_json::Value>(&file)
        .ok()
        .and_then(|value| value.get("claudeAiOauth").cloned())
        .ok_or_else(|| {
            "Claude sign-in has no OAuth login (API-key logins have no plan limits)".to_owned()
        })
}

fn claude_access_token_in(dir: &Path, now_ms: i64) -> Result<String, String> {
    let oauth = claude_oauth_in(dir)?;
    let expires_ms = oauth
        .get("expiresAt")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0);
    // Gnomcode never refreshes the token itself: Claude Code rotates refresh tokens. The daemon
    // asks the official CLI to do it (`Refresher`); the Claude desktop app does not rewrite this file.
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
        let mut refresher = Refresher::default();
        loop {
            ticker.tick().await;
            for provider in PROVIDERS {
                if !enabled(provider) {
                    record_issue(provider, None);
                    continue;
                }
                // A refreshed token is picked up by the poll that follows in this same pass.
                if refresh_enabled(provider) {
                    refresher
                        .refresh_if_due(
                            &claude_config_dir(),
                            chrono::Utc::now().timestamp_millis(),
                            std::time::Instant::now(),
                            || {
                                let mut command =
                                    gnomcode_platform::executables::background_agent_command(
                                        "claude",
                                    );
                                command.args(["auth", "status"]);
                                command
                            },
                            REFRESH_TIMEOUT,
                        )
                        .await;
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

    const SIGNED_IN: &str = "claudeAiOauth";

    fn write_credentials(dir: &std::path::Path, expires_at: i64) {
        std::fs::write(
            dir.join(".credentials.json"),
            serde_json::json!({SIGNED_IN: {"accessToken": "test-token", "expiresAt": expires_at}})
                .to_string(),
        )
        .expect("credentials");
    }

    /// A stand-in for the official CLI that appends its arguments to `marker.txt` and exits.
    fn fake_cli(dir: &std::path::Path, hang: bool) -> PathBuf {
        #[cfg(windows)]
        let (name, body) = (
            "fake-claude.cmd",
            format!(
                "@echo off\r\necho %*>>\"%~dp0marker.txt\"\r\n{}",
                if hang {
                    "ping -n 30 127.0.0.1 >nul\r\n"
                } else {
                    ""
                }
            ),
        );
        #[cfg(not(windows))]
        let (name, body) = (
            "fake-claude.sh",
            format!(
                "#!/bin/sh\necho \"$@\" >> \"$(dirname \"$0\")/marker.txt\"\n{}",
                if hang { "sleep 30\n" } else { "" }
            ),
        );
        let path = dir.join(name);
        std::fs::write(&path, body).expect("script");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
                .expect("permissions");
        }
        path
    }

    fn runs(dir: &std::path::Path) -> usize {
        std::fs::read_to_string(dir.join("marker.txt")).map_or(0, |text| text.lines().count())
    }

    fn command(path: &std::path::Path) -> std::process::Command {
        let mut command = std::process::Command::new(path);
        command.args(["auth", "status"]);
        command
    }

    const NOW_MS: i64 = 1_000_000_000_000;
    const MINUTE_MS: i64 = 60_000;

    #[tokio::test]
    async fn an_expired_sign_in_runs_the_cli_with_only_the_status_command() {
        let dir = tempfile::tempdir().expect("temp dir");
        write_credentials(dir.path(), NOW_MS - MINUTE_MS);
        let cli = fake_cli(dir.path(), false);
        let mut refresher = Refresher::default();
        let ran = refresher
            .refresh_if_due(
                dir.path(),
                NOW_MS,
                std::time::Instant::now(),
                || command(&cli),
                Duration::from_secs(20),
            )
            .await;
        assert!(ran);
        assert_eq!(runs(dir.path()), 1);
        let args = std::fs::read_to_string(dir.path().join("marker.txt")).expect("marker");
        assert_eq!(args.trim(), "auth status");
    }

    #[tokio::test]
    async fn a_sign_in_expiring_within_thirty_minutes_is_refreshed_but_a_fresh_one_is_not() {
        let dir = tempfile::tempdir().expect("temp dir");
        let cli = fake_cli(dir.path(), false);
        let now = std::time::Instant::now();
        let mut refresher = Refresher::default();
        write_credentials(dir.path(), NOW_MS + 31 * MINUTE_MS);
        assert!(
            !refresher
                .refresh_if_due(
                    dir.path(),
                    NOW_MS,
                    now,
                    || command(&cli),
                    Duration::from_secs(20)
                )
                .await
        );
        assert_eq!(runs(dir.path()), 0);
        write_credentials(dir.path(), NOW_MS + 29 * MINUTE_MS);
        assert!(
            refresher
                .refresh_if_due(
                    dir.path(),
                    NOW_MS,
                    now,
                    || command(&cli),
                    Duration::from_secs(20)
                )
                .await
        );
        assert_eq!(runs(dir.path()), 1);
    }

    #[tokio::test]
    async fn refreshing_is_limited_to_once_every_fifteen_minutes() {
        let dir = tempfile::tempdir().expect("temp dir");
        write_credentials(dir.path(), NOW_MS - MINUTE_MS);
        let cli = fake_cli(dir.path(), false);
        let now = std::time::Instant::now();
        let mut refresher = Refresher::default();
        let mut attempt = async |at: std::time::Instant| {
            refresher
                .refresh_if_due(
                    dir.path(),
                    NOW_MS,
                    at,
                    || command(&cli),
                    Duration::from_secs(20),
                )
                .await
        };
        assert!(attempt(now).await);
        assert!(!attempt(now + Duration::from_secs(14 * 60)).await);
        assert!(attempt(now + Duration::from_secs(15 * 60 + 1)).await);
        assert_eq!(runs(dir.path()), 2);
    }

    #[tokio::test]
    async fn a_hung_cli_is_stopped_at_the_timeout_and_counts_as_an_attempt() {
        let dir = tempfile::tempdir().expect("temp dir");
        write_credentials(dir.path(), NOW_MS - MINUTE_MS);
        let cli = fake_cli(dir.path(), true);
        let now = std::time::Instant::now();
        let mut refresher = Refresher::default();
        let started = std::time::Instant::now();
        let ran = refresher
            .refresh_if_due(
                dir.path(),
                NOW_MS,
                now,
                || command(&cli),
                Duration::from_millis(500),
            )
            .await;
        assert!(!ran);
        assert!(started.elapsed() < Duration::from_secs(10));
        assert!(
            !refresher
                .refresh_if_due(
                    dir.path(),
                    NOW_MS,
                    now,
                    || command(&cli),
                    Duration::from_millis(500)
                )
                .await
        );
    }

    #[tokio::test]
    async fn a_missing_cli_or_sign_in_is_not_an_error() {
        let dir = tempfile::tempdir().expect("temp dir");
        let now = std::time::Instant::now();
        let mut refresher = Refresher::default();
        let missing = dir.path().join("no-such-cli");
        assert!(
            !refresher
                .refresh_if_due(
                    dir.path(),
                    NOW_MS,
                    now,
                    || command(&missing),
                    Duration::from_secs(5)
                )
                .await
        );
        write_credentials(dir.path(), NOW_MS - MINUTE_MS);
        assert!(
            !Refresher::default()
                .refresh_if_due(
                    dir.path(),
                    NOW_MS,
                    now,
                    || command(&missing),
                    Duration::from_secs(5)
                )
                .await
        );
    }

    #[test]
    fn refresh_follows_the_opt_in_and_defaults_on_when_limits_are_enabled() {
        let dir = tempfile::tempdir().expect("temp dir");
        let consent = dir.path().join("vendor-limits.json");
        assert!(!refresh_enabled_in(&consent, "claude"), "no file");
        std::fs::write(&consent, r#"{"antigravity":true}"#).expect("consent");
        assert!(
            !refresh_enabled_in(&consent, "claude"),
            "limits not enabled"
        );
        std::fs::write(&consent, r#"{"claude":true}"#).expect("consent");
        assert!(refresh_enabled_in(&consent, "claude"), "default on");
        std::fs::write(&consent, r#"{"claude":true,"claude_refresh":false}"#).expect("consent");
        assert!(!refresh_enabled_in(&consent, "claude"), "turned off");
        assert!(!refresh_enabled_in(&consent, "antigravity"), "claude only");
        std::fs::write(&consent, r#"{"claude":false,"claude_refresh":true}"#).expect("consent");
        assert!(!refresh_enabled_in(&consent, "claude"), "limits disabled");
    }

    #[test]
    fn the_refresh_setting_is_written_beside_the_opt_in() {
        let dir = tempfile::tempdir().expect("temp dir");
        let consent = dir.path().join("vendor-limits.json");
        set_flag_in(&consent, "claude", true).expect("enable");
        set_flag_in(&consent, "claude_refresh", false).expect("no refresh");
        assert!(enabled_in(&consent, "claude"));
        assert!(!refresh_enabled_in(&consent, "claude"));
        set_flag_in(&consent, "claude_refresh", true).expect("refresh");
        assert!(refresh_enabled_in(&consent, "claude"));
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
