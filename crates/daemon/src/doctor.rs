//! Environment and runtime diagnostics for `gnomcode doctor` and `GET /api/v1/doctor`.

use chrono::{DateTime, Utc};
use gnomcode_store::Database;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncReadExt as _;

const FIVE_GIB: u64 = 5 * 1024 * 1024 * 1024;
const HUNDRED_MIB: u64 = 100 * 1024 * 1024;
const CLAUDE_REFRESH_WARN_MS: i64 = 30 * 60 * 1000;
const READING_STALE_SECS: i64 = 15 * 60;
const CLI_PROBE_TIMEOUT: Duration = Duration::from_secs(2);
const CLI_OUTPUT_CAP: u64 = 4096;

/// Diagnostic severity for a single check or the overall report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    /// Check passed with no action needed.
    Ok,
    /// Non-fatal issue or unconfigured optional capability.
    Warn,
    /// Actionable failure; `gnomcode doctor` exits non-zero.
    Fail,
}

/// One diagnostic check result with an optional one-line remediation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorCheck {
    /// Stable machine identifier (e.g. `agent_clis`, `claude_signin`).
    pub id: String,
    /// Human-readable check name.
    pub label: String,
    /// Result status (`ok`, `warn`, or `fail`).
    pub status: CheckStatus,
    /// One-line summary of the observed state.
    pub summary: String,
    /// One-line fix instruction when action is needed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<String>,
    /// Structured metadata for JSON consumers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// Complete diagnostic report returned by `GET /api/v1/doctor` and `gnomcode doctor --json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorReport {
    /// Worst status across all checks.
    pub status: CheckStatus,
    /// `true` when no check has status `fail`.
    pub ok: bool,
    /// UTC timestamp when diagnostics ran.
    pub checked_at: String,
    /// Resolved Gnomcode data directory.
    pub gnomcode_home: String,
    /// Resolved SQLite database path.
    pub db_path: String,
    /// Ordered list of diagnostic checks.
    pub checks: Vec<DoctorCheck>,
}

/// Detected binary and version for one supported agent CLI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentCliStatus {
    /// Agent CLI binary name (`claude`, `codex`, `gemini`, `agy`).
    pub agent: String,
    /// Whether the executable was found on PATH or standard install locations.
    pub detected: bool,
    /// Resolved executable path or launcher command when detected.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary: Option<String>,
    /// Reported `--version` line when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Error summary if execution failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Inputs for deterministic evaluation in tests and live execution.
pub struct DoctorInputs<'a> {
    /// Resolved `GNOMCODE_HOME` directory.
    pub gnomcode_home: PathBuf,
    /// Resolved Claude configuration directory (`CLAUDE_CONFIG_DIR` or `~/.claude`).
    pub claude_config_dir: PathBuf,
    /// Target SQLite database path.
    pub db_path: PathBuf,
    /// Target repository or working directory for repo drive free space.
    pub repo_dir: Option<PathBuf>,
    /// Optional open database handle for quota snapshot ages.
    pub db: Option<&'a Database>,
    /// Error message if opening the database failed.
    pub db_error: Option<String>,
    /// Current UTC timestamp.
    pub now: DateTime<Utc>,
    /// Whether diagnostics are running inside the live daemon process.
    pub in_daemon: bool,
    /// Live daemon reachability override when running from CLI (`None` = absent/not running).
    pub daemon_reachable: Option<Result<String, String>>,
    /// Whether keep-awake is enabled (`GNOMCODE_KEEP_AWAKE != "0"`).
    pub keep_awake_enabled: bool,
    /// Whether the platform supports keep-awake.
    pub keep_awake_supported: bool,
    /// Number of active keep-awake holds when known.
    pub keep_awake_holds: Option<usize>,
    /// Probed agent CLIs.
    pub agent_clis: Vec<AgentCliStatus>,
    /// In-memory provider limit issues (e.g. from `vendor_limits::issue`).
    pub provider_issues: Vec<(String, String)>,
}

/// Classifies free disk bytes (`fail` < 100 MiB, `warn` < 5 GiB, `ok` >= 5 GiB).
#[must_use]
pub fn classify_disk_bytes(bytes: u64) -> CheckStatus {
    if bytes < HUNDRED_MIB {
        CheckStatus::Fail
    } else if bytes < FIVE_GIB {
        CheckStatus::Warn
    } else {
        CheckStatus::Ok
    }
}

/// Evaluates all doctor checks from `inputs`.
#[must_use]
pub fn evaluate(inputs: &DoctorInputs<'_>) -> DoctorReport {
    let checks = vec![
        check_agent_clis(&inputs.agent_clis),
        check_claude_signin(&inputs.claude_config_dir, inputs.now),
        check_limit_sources(inputs),
        check_claude_integration(&inputs.claude_config_dir),
        check_daemon(inputs),
        check_disk_space(&inputs.gnomcode_home, inputs.repo_dir.as_deref()),
        check_keep_awake(
            inputs.keep_awake_enabled,
            inputs.keep_awake_supported,
            inputs.keep_awake_holds,
        ),
    ];
    let status = checks
        .iter()
        .map(|c| c.status)
        .max()
        .unwrap_or(CheckStatus::Ok);
    DoctorReport {
        status,
        ok: status != CheckStatus::Fail,
        checked_at: inputs.now.to_rfc3339(),
        gnomcode_home: inputs.gnomcode_home.display().to_string(),
        db_path: inputs.db_path.display().to_string(),
        checks,
    }
}

fn check_agent_clis(clis: &[AgentCliStatus]) -> DoctorCheck {
    let detected: Vec<&AgentCliStatus> = clis.iter().filter(|c| c.detected).collect();
    let failed: Vec<&AgentCliStatus> = detected
        .iter()
        .copied()
        .filter(|c| c.error.is_some())
        .collect();
    let details = serde_json::json!({ "clis": clis });
    if !failed.is_empty() {
        let names = failed
            .iter()
            .map(|c| c.agent.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return DoctorCheck {
            id: "agent_clis".into(),
            label: "Agent CLIs".into(),
            status: CheckStatus::Warn,
            summary: format!(
                "{} CLI(s) detected, but version check failed for {names}",
                detected.len()
            ),
            fix: Some(format!(
                "Verify executable permissions and PATH for {names}."
            )),
            details: Some(details),
        };
    }
    if detected.is_empty() {
        return DoctorCheck {
            id: "agent_clis".into(),
            label: "Agent CLIs".into(),
            status: CheckStatus::Warn,
            summary: "No supported agent CLIs detected".into(),
            fix: Some(
                "Install an official CLI (claude, codex, gemini, or agy) and ensure it is on PATH."
                    .into(),
            ),
            details: Some(details),
        };
    }
    let summary_items = detected
        .iter()
        .map(|c| match &c.version {
            Some(ver) => format!("{} ({ver})", c.agent),
            None => c.agent.clone(),
        })
        .collect::<Vec<_>>()
        .join(", ");
    DoctorCheck {
        id: "agent_clis".into(),
        label: "Agent CLIs".into(),
        status: CheckStatus::Ok,
        summary: format!("{} detected: {summary_items}", detected.len()),
        fix: None,
        details: Some(details),
    }
}

fn check_claude_signin(claude_dir: &Path, now: DateTime<Utc>) -> DoctorCheck {
    let creds_path = claude_dir.join(obfstr::obfstr!(".credentials.json"));
    let Ok(chunk) = gnomcode_platform::read_from_offset(&creds_path, 0) else {
        return DoctorCheck {
            id: "claude_signin".into(),
            label: "Claude sign-in".into(),
            status: CheckStatus::Warn,
            summary: "Claude sign-in file not found".into(),
            fix: Some("Run `claude` in a terminal to sign in.".into()),
            details: None,
        };
    };
    let Ok(parsed) = serde_json::from_slice::<serde_json::Value>(&chunk.bytes) else {
        return DoctorCheck {
            id: "claude_signin".into(),
            label: "Claude sign-in".into(),
            status: CheckStatus::Fail,
            summary: "Claude credentials file contains invalid JSON".into(),
            fix: Some("Re-authenticate by running `claude auth login` in a terminal.".into()),
            details: None,
        };
    };
    let expires_ms = parsed
        .get(obfstr::obfstr!("claudeAiOauth"))
        .and_then(|oauth| oauth.get("expiresAt"))
        .and_then(serde_json::Value::as_i64);
    let Some(expires_ms) = expires_ms else {
        return DoctorCheck {
            id: "claude_signin".into(),
            label: "Claude sign-in".into(),
            status: CheckStatus::Warn,
            summary: "Claude OAuth expiry metadata not found".into(),
            fix: Some("Run `claude auth login` for OAuth sign-in if using plan limits.".into()),
            details: None,
        };
    };
    let expires_at = DateTime::from_timestamp_millis(expires_ms).map(|dt| dt.to_rfc3339());
    let delta_ms = expires_ms - now.timestamp_millis();
    let expires_in_seconds = delta_ms / 1000;
    let details = Some(serde_json::json!({
        "expires_at": expires_at,
        "expires_in_seconds": expires_in_seconds,
    }));
    if delta_ms <= 0 {
        return DoctorCheck {
            id: "claude_signin".into(),
            label: "Claude sign-in".into(),
            status: CheckStatus::Fail,
            summary: format!(
                "Claude sign-in expired {} ago",
                format_duration(-expires_in_seconds)
            ),
            fix: Some(
                "Run `claude auth login` or `claude` in a terminal to refresh sign-in.".into(),
            ),
            details,
        };
    }
    if delta_ms <= CLAUDE_REFRESH_WARN_MS {
        return DoctorCheck {
            id: "claude_signin".into(),
            label: "Claude sign-in".into(),
            status: CheckStatus::Warn,
            summary: format!(
                "Claude sign-in expires in {}",
                format_duration(expires_in_seconds)
            ),
            fix: Some("Run `claude auth status` to refresh the sign-in.".into()),
            details,
        };
    }
    DoctorCheck {
        id: "claude_signin".into(),
        label: "Claude sign-in".into(),
        status: CheckStatus::Ok,
        summary: format!(
            "Claude sign-in valid (expires in {})",
            format_duration(expires_in_seconds)
        ),
        fix: None,
        details,
    }
}

fn check_limit_sources(inputs: &DoctorInputs<'_>) -> DoctorCheck {
    if let Some(error) = &inputs.db_error {
        return DoctorCheck {
            id: "limit_sources".into(),
            label: "Limit sources".into(),
            status: CheckStatus::Fail,
            summary: format!("Could not open database: {error}"),
            fix: Some("Check GNOMCODE_HOME directory permissions and database integrity.".into()),
            details: None,
        };
    }
    let snapshots = inputs
        .db
        .and_then(|db| db.get_latest_quota_snapshots().ok())
        .unwrap_or_default();
    let consent_path = inputs.gnomcode_home.join("vendor-limits.json");
    let claude_api = crate::vendor_limits::enabled_in(&consent_path, "claude");
    let claude_statusline = inputs
        .gnomcode_home
        .join(gnomcode_ingest::CLAUDE_LIMITS_FILE)
        .is_file();
    let antigravity_api = crate::vendor_limits::enabled_in(&consent_path, "antigravity");

    let latest_for = |provider: &str| {
        snapshots
            .iter()
            .filter(|snap| snap.provider == provider)
            .map(|snap| snap.observed_at)
            .max()
    };
    let issue_for = |key: &str| {
        inputs
            .provider_issues
            .iter()
            .find(|(provider, _)| provider == key)
            .map(|(_, msg)| msg.clone())
    };

    let providers_cfg = [
        (
            "claude-code",
            "claude",
            claude_api || claude_statusline,
            if claude_api {
                "provider_api"
            } else if claude_statusline {
                "statusline"
            } else {
                "off"
            },
        ),
        ("codex", "codex", true, "local_logs"),
        (
            "antigravity",
            "antigravity",
            antigravity_api,
            if antigravity_api {
                "language_server"
            } else {
                "off"
            },
        ),
    ];

    let mut entries = Vec::new();
    let mut parts = Vec::new();
    let mut issues = Vec::new();
    let mut stale_opt_in = false;
    let mut any_opt_in_or_reading = claude_api || claude_statusline || antigravity_api;

    for (slug, issue_key, enabled, source) in providers_cfg {
        let last_at = latest_for(slug);
        if last_at.is_some() {
            any_opt_in_or_reading = true;
        }
        let age_seconds =
            last_at.map(|ts| inputs.now.signed_duration_since(ts).num_seconds().max(0));
        let issue = issue_for(issue_key);
        if let Some(ref msg) = issue {
            issues.push(format!("{slug}: {msg}"));
        }
        if enabled && slug != "codex" && age_seconds.is_none_or(|age| age > READING_STALE_SECS) {
            stale_opt_in = true;
        }
        let age_label = age_seconds.map_or_else(|| "none".to_owned(), format_duration);
        parts.push(format!("{slug}: {source} (age {age_label})"));
        entries.push(serde_json::json!({
            "provider": slug,
            "enabled": enabled,
            "source": source,
            "last_reading_at": last_at.map(|ts| ts.to_rfc3339()),
            "age_seconds": age_seconds,
            "issue": issue,
        }));
    }

    let details = Some(serde_json::json!({ "providers": entries }));
    if !issues.is_empty() {
        return DoctorCheck {
            id: "limit_sources".into(),
            label: "Limit sources".into(),
            status: CheckStatus::Warn,
            summary: issues.join("; "),
            fix: Some(
                "Run `gnomcode limits check <provider>` or verify sign-in/running IDE.".into(),
            ),
            details,
        };
    }
    if !any_opt_in_or_reading {
        return DoctorCheck {
            id: "limit_sources".into(),
            label: "Limit sources".into(),
            status: CheckStatus::Warn,
            summary: "No opt-in limit sources enabled and no quota readings recorded".into(),
            fix: Some(
                "Run `gnomcode limits enable claude` or `gnomcode limits enable antigravity`."
                    .into(),
            ),
            details,
        };
    }
    if stale_opt_in {
        return DoctorCheck {
            id: "limit_sources".into(),
            label: "Limit sources".into(),
            status: CheckStatus::Warn,
            summary: parts.join(" | "),
            fix: Some(
                "Run `gnomcode limits check claude` or start the daemon to refresh limits.".into(),
            ),
            details,
        };
    }
    DoctorCheck {
        id: "limit_sources".into(),
        label: "Limit sources".into(),
        status: CheckStatus::Ok,
        summary: parts.join(" | "),
        fix: None,
        details,
    }
}

fn check_claude_integration(claude_dir: &Path) -> DoctorCheck {
    let path = claude_dir.join("settings.json");
    if !path.exists() {
        return DoctorCheck {
            id: "claude_integration".into(),
            label: "Claude integration".into(),
            status: CheckStatus::Warn,
            summary: "Claude settings.json not found; hook and status line not installed".into(),
            fix: Some("Run `gnomcode integrate claude` to install hooks and status line.".into()),
            details: Some(serde_json::json!({
                "hook_installed": false,
                "status_line_installed": false,
            })),
        };
    }
    let Ok(bytes) = std::fs::read(&path) else {
        return DoctorCheck {
            id: "claude_integration".into(),
            label: "Claude integration".into(),
            status: CheckStatus::Fail,
            summary: "Claude settings.json could not be read".into(),
            fix: Some("Check file permissions on Claude settings.json.".into()),
            details: None,
        };
    };
    let Ok(settings) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return DoctorCheck {
            id: "claude_integration".into(),
            label: "Claude integration".into(),
            status: CheckStatus::Fail,
            summary: "Claude settings.json contains invalid JSON".into(),
            fix: Some(
                "Fix JSON syntax in Claude settings.json and re-run `gnomcode integrate claude`."
                    .into(),
            ),
            details: None,
        };
    };
    if !settings.is_object() {
        return DoctorCheck {
            id: "claude_integration".into(),
            label: "Claude integration".into(),
            status: CheckStatus::Fail,
            summary: "Claude settings.json must be a JSON object".into(),
            fix: Some(
                "Fix Claude settings.json structure and re-run `gnomcode integrate claude`.".into(),
            ),
            details: None,
        };
    }
    let hook_installed = settings
        .pointer("/hooks/PreToolUse")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|entries| {
            entries.iter().any(|entry| {
                entry
                    .get("hooks")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|handlers| {
                        handlers.iter().any(|handler| {
                            let cmd = handler
                                .get("command")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or("");
                            let is_gnomcode = Path::new(cmd)
                                .file_name()
                                .and_then(|name| name.to_str())
                                .is_some_and(|name| ["gnomcode", "gnomcode.exe"].contains(&name));
                            is_gnomcode && handler.get("args") == Some(&serde_json::json!(["hook"]))
                        })
                    })
            })
        });
    let status_line_cmd = settings
        .pointer("/statusLine/command")
        .and_then(serde_json::Value::as_str);
    let status_line_installed =
        status_line_cmd.is_some_and(|cmd| cmd.contains("gnomcode") && cmd.ends_with(" statusline"));
    let custom_status_line = status_line_cmd.is_some() && !status_line_installed;
    let details = Some(serde_json::json!({
        "hook_installed": hook_installed,
        "status_line_installed": status_line_installed,
        "custom_status_line": custom_status_line,
    }));

    match (hook_installed, status_line_installed) {
        (true, true) => DoctorCheck {
            id: "claude_integration".into(),
            label: "Claude integration".into(),
            status: CheckStatus::Ok,
            summary: "Claude guardrail hook and limits status line installed".into(),
            fix: None,
            details,
        },
        (true, false) if custom_status_line => DoctorCheck {
            id: "claude_integration".into(),
            label: "Claude integration".into(),
            status: CheckStatus::Warn,
            summary: "Claude guardrail hook installed; custom status line preserved".into(),
            fix: Some("Run `gnomcode limits enable claude` to read Claude limits without replacing your status line.".into()),
            details,
        },
        (true, false) => DoctorCheck {
            id: "claude_integration".into(),
            label: "Claude integration".into(),
            status: CheckStatus::Warn,
            summary: "Claude guardrail hook installed; limits status line missing".into(),
            fix: Some("Run `gnomcode integrate claude` to install the limits status line.".into()),
            details,
        },
        (false, true) => DoctorCheck {
            id: "claude_integration".into(),
            label: "Claude integration".into(),
            status: CheckStatus::Warn,
            summary: "Claude limits status line installed; guardrail hook missing".into(),
            fix: Some("Run `gnomcode integrate claude` to install the guardrail hook.".into()),
            details,
        },
        (false, false) => DoctorCheck {
            id: "claude_integration".into(),
            label: "Claude integration".into(),
            status: CheckStatus::Warn,
            summary: "Claude guardrail hook and status line not installed".into(),
            fix: Some("Run `gnomcode integrate claude` to install them.".into()),
            details,
        },
    }
}

fn check_daemon(inputs: &DoctorInputs<'_>) -> DoctorCheck {
    if inputs.in_daemon {
        return DoctorCheck {
            id: "daemon".into(),
            label: "Daemon".into(),
            status: CheckStatus::Ok,
            summary: format!(
                "Daemon reachable (v{}, PID {})",
                env!("CARGO_PKG_VERSION"),
                std::process::id()
            ),
            fix: None,
            details: Some(serde_json::json!({
                "reachable": true,
                "pid": std::process::id(),
                "version": env!("CARGO_PKG_VERSION"),
            })),
        };
    }
    match &inputs.daemon_reachable {
        Some(Ok(summary)) => DoctorCheck {
            id: "daemon".into(),
            label: "Daemon".into(),
            status: CheckStatus::Ok,
            summary: summary.clone(),
            fix: None,
            details: Some(serde_json::json!({ "reachable": true })),
        },
        Some(Err(reason)) => DoctorCheck {
            id: "daemon".into(),
            label: "Daemon".into(),
            status: CheckStatus::Fail,
            summary: reason.clone(),
            fix: Some("Restart the daemon with `gnomcode daemon`.".into()),
            details: Some(serde_json::json!({ "reachable": false })),
        },
        None => DoctorCheck {
            id: "daemon".into(),
            label: "Daemon".into(),
            status: CheckStatus::Warn,
            summary: "Daemon is not running".into(),
            fix: Some("Start the daemon with `gnomcode daemon`.".into()),
            details: Some(serde_json::json!({ "reachable": false })),
        },
    }
}

fn check_disk_space(gnomcode_home: &Path, repo_dir: Option<&Path>) -> DoctorCheck {
    let mut targets = vec![("gnomcode_home", gnomcode_home.to_path_buf())];
    if let Some(repo) = repo_dir {
        targets.push(("repo", repo.to_path_buf()));
    }
    let mut worst = CheckStatus::Ok;
    let mut summaries = Vec::new();
    let mut details_rows = Vec::new();

    for (name, path) in targets {
        let free = gnomcode_platform::disk_free_bytes(&path);
        match free {
            Some(bytes) => {
                let status = classify_disk_bytes(bytes);
                if status > worst {
                    worst = status;
                }
                summaries.push(format!("{name}: {} free", format_bytes(bytes)));
                details_rows.push(serde_json::json!({
                    "target": name,
                    "path": path.display().to_string(),
                    "free_bytes": bytes,
                    "status": status,
                }));
            }
            None => {
                if CheckStatus::Warn > worst {
                    worst = CheckStatus::Warn;
                }
                summaries.push(format!("{name}: unknown free space"));
                details_rows.push(serde_json::json!({
                    "target": name,
                    "path": path.display().to_string(),
                    "free_bytes": serde_json::Value::Null,
                    "status": CheckStatus::Warn,
                }));
            }
        }
    }

    let fix = match worst {
        CheckStatus::Ok => None,
        CheckStatus::Warn => Some("Free up disk space so at least 5 GB remains available.".into()),
        CheckStatus::Fail => {
            Some("Free up disk space immediately (under 100 MB remaining).".into())
        }
    };
    DoctorCheck {
        id: "disk_space".into(),
        label: "Disk space".into(),
        status: worst,
        summary: summaries.join(" | "),
        fix,
        details: Some(serde_json::json!({ "targets": details_rows })),
    }
}

fn check_keep_awake(enabled: bool, supported: bool, holds: Option<usize>) -> DoctorCheck {
    let details = Some(serde_json::json!({
        "enabled": enabled,
        "supported": supported,
        "holds": holds,
    }));
    if !enabled {
        return DoctorCheck {
            id: "keep_awake".into(),
            label: "Keep-awake".into(),
            status: CheckStatus::Warn,
            summary: "Keep-awake disabled (GNOMCODE_KEEP_AWAKE=0)".into(),
            fix: Some(
                "Unset GNOMCODE_KEEP_AWAKE to prevent sleep during active or scheduled work."
                    .into(),
            ),
            details,
        };
    }
    if !supported {
        return DoctorCheck {
            id: "keep_awake".into(),
            label: "Keep-awake".into(),
            status: CheckStatus::Warn,
            summary: "Keep-awake unavailable (systemd-inhibit not found)".into(),
            fix: Some("Install systemd to enable sleep inhibition on Linux.".into()),
            details,
        };
    }
    let summary = match holds {
        Some(n) if n > 0 => format!("Keep-awake active ({n} hold)"),
        Some(_) => "Keep-awake ready (idle)".into(),
        None => "Keep-awake ready (daemon stopped)".into(),
    };
    DoctorCheck {
        id: "keep_awake".into(),
        label: "Keep-awake".into(),
        status: CheckStatus::Ok,
        summary,
        fix: None,
        details,
    }
}

fn format_duration(secs: i64) -> String {
    let secs = secs.max(0);
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else if secs < 86400 {
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        if m == 0 {
            format!("{h}h")
        } else {
            format!("{h}h {m}m")
        }
    } else {
        let d = secs / 86400;
        let h = (secs % 86400) / 3600;
        if h == 0 {
            format!("{d}d")
        } else {
            format!("{d}d {h}h")
        }
    }
}

fn format_bytes(bytes: u64) -> String {
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    const MIB: f64 = 1024.0 * 1024.0;
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.1} GB", bytes as f64 / GIB)
    } else {
        format!("{:.0} MB", bytes as f64 / MIB)
    }
}

async fn probe_one_cli(agent: &'static str) -> AgentCliStatus {
    let Some((program, args)) = gnomcode_platform::executables::resolve_agent_executable(agent)
    else {
        return AgentCliStatus {
            agent: agent.to_owned(),
            detected: false,
            binary: None,
            version: None,
            error: None,
        };
    };
    let binary = if args.is_empty() {
        program
    } else {
        format!("{program} {}", args.join(" "))
    };
    let mut command = tokio::process::Command::from(
        gnomcode_platform::executables::background_agent_command(agent),
    );
    command
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let Ok(mut child) = command.spawn() else {
        return AgentCliStatus {
            agent: agent.to_owned(),
            detected: true,
            binary: Some(binary),
            version: None,
            error: Some("failed to spawn --version".to_owned()),
        };
    };
    let mut out = Vec::new();
    let mut err = Vec::new();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let probe = tokio::time::timeout(CLI_PROBE_TIMEOUT, async {
        if let Some(s) = stdout {
            let _ = s.take(CLI_OUTPUT_CAP).read_to_end(&mut out).await;
        }
        if let Some(s) = stderr {
            let _ = s.take(CLI_OUTPUT_CAP).read_to_end(&mut err).await;
        }
        child.wait().await
    })
    .await;

    match probe {
        Ok(Ok(status)) if status.success() => {
            let text = String::from_utf8_lossy(&out);
            let line = text
                .lines()
                .map(str::trim)
                .find(|l| !l.is_empty())
                .map(|l| l.chars().take(120).collect::<String>());
            AgentCliStatus {
                agent: agent.to_owned(),
                detected: true,
                binary: Some(binary),
                version: line,
                error: None,
            }
        }
        Ok(Ok(status)) => AgentCliStatus {
            agent: agent.to_owned(),
            detected: true,
            binary: Some(binary),
            version: None,
            error: Some(format!("exited with status {status}")),
        },
        Ok(Err(_)) => AgentCliStatus {
            agent: agent.to_owned(),
            detected: true,
            binary: Some(binary),
            version: None,
            error: Some("version probe I/O error".to_owned()),
        },
        Err(_) => {
            let _ = child.kill().await;
            AgentCliStatus {
                agent: agent.to_owned(),
                detected: true,
                binary: Some(binary),
                version: None,
                error: Some("version probe timed out".to_owned()),
            }
        }
    }
}

async fn probe_agent_clis() -> Vec<AgentCliStatus> {
    let (claude, codex, gemini, agy) = tokio::join!(
        probe_one_cli("claude"),
        probe_one_cli("codex"),
        probe_one_cli("gemini"),
        probe_one_cli("agy"),
    );
    vec![claude, codex, gemini, agy]
}

fn keep_awake_enabled_env() -> bool {
    std::env::var_os("GNOMCODE_KEEP_AWAKE").is_none_or(|v| v != "0")
}

/// Runs diagnostics from inside the daemon process.
pub async fn run_for_daemon(
    db: &std::sync::Arc<tokio::sync::Mutex<Database>>,
    db_path: &Path,
    repo_dir: Option<PathBuf>,
) -> DoctorReport {
    let agent_clis = probe_agent_clis().await;
    let provider_issues = crate::vendor_limits::PROVIDERS
        .iter()
        .filter_map(|p| crate::vendor_limits::issue(p).map(|msg| ((*p).to_owned(), msg)))
        .collect();
    let keep_awake_holds = Some(crate::automation::keep_awake_holds());
    let db_guard = db.lock().await;
    let inputs = DoctorInputs {
        gnomcode_home: gnomcode_platform::gnomcode_home(),
        claude_config_dir: crate::vendor_limits::claude_config_dir(),
        db_path: db_path.to_path_buf(),
        repo_dir: repo_dir.or_else(|| std::env::current_dir().ok()),
        db: Some(&db_guard),
        db_error: None,
        now: Utc::now(),
        in_daemon: true,
        daemon_reachable: None,
        keep_awake_enabled: keep_awake_enabled_env(),
        keep_awake_supported: gnomcode_platform::keepawake::is_supported(),
        keep_awake_holds,
        agent_clis,
        provider_issues,
    };
    evaluate(&inputs)
}

async fn query_running_daemon(
    home: &Path,
    repo_dir: Option<&Path>,
) -> (Option<DoctorReport>, Option<Result<String, String>>) {
    let info_path = home.join("daemon.json");
    if !info_path.exists() {
        return (None, None);
    }
    let Ok(bytes) = std::fs::read(&info_path) else {
        return (
            None,
            Some(Err("daemon.json exists but could not be read".to_owned())),
        );
    };
    let Ok(info) = serde_json::from_slice::<crate::DaemonInfo>(&bytes) else {
        return (
            None,
            Some(Err(
                "daemon.json exists but contains invalid JSON".to_owned()
            )),
        );
    };
    let Ok(client) = reqwest::Client::builder()
        .timeout(Duration::from_millis(1500))
        .build()
    else {
        return (
            None,
            Some(Err(
                "could not build HTTP client for daemon check".to_owned()
            )),
        );
    };
    let base = format!("http://127.0.0.1:{}/api/v1/doctor", info.port);
    let url = match repo_dir {
        Some(repo) => {
            let repo_str = repo.to_string_lossy();
            reqwest::Url::parse_with_params(&base, &[("repo", repo_str.as_ref())])
                .map_or(base, |u| u.into())
        }
        None => base,
    };
    let request = client
        .get(&url)
        .header("Host", "127.0.0.1")
        .bearer_auth(&info.token);
    match request.send().await {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(report) = resp.json::<DoctorReport>().await {
                return (
                    Some(report),
                    Some(Ok(format!(
                        "Daemon reachable on 127.0.0.1:{} (PID {})",
                        info.port, info.pid
                    ))),
                );
            }
            (
                None,
                Some(Ok(format!(
                    "Daemon reachable on 127.0.0.1:{} (PID {})",
                    info.port, info.pid
                ))),
            )
        }
        _ => (
            None,
            Some(Err(format!(
                "Daemon metadata exists (PID {}, port {}) but daemon is unreachable",
                info.pid, info.port
            ))),
        ),
    }
}

/// Runs diagnostics for the CLI, querying the running daemon when reachable.
pub async fn run_for_cli(db_path: &Path, repo_dir: Option<PathBuf>) -> DoctorReport {
    let home = gnomcode_platform::gnomcode_home();
    let effective_repo = repo_dir.or_else(|| std::env::current_dir().ok());
    let (live_report, daemon_reachable) =
        query_running_daemon(&home, effective_repo.as_deref()).await;
    if let Some(report) = live_report {
        return report;
    }
    let agent_clis = probe_agent_clis().await;
    let (db_handle, db_error) = if db_path.exists() {
        match Database::open(db_path) {
            Ok(db) => (Some(db), None),
            Err(err) => (None, Some(err.to_string())),
        }
    } else {
        (None, None)
    };
    let inputs = DoctorInputs {
        gnomcode_home: home,
        claude_config_dir: crate::vendor_limits::claude_config_dir(),
        db_path: db_path.to_path_buf(),
        repo_dir: effective_repo,
        db: db_handle.as_ref(),
        db_error,
        now: Utc::now(),
        in_daemon: false,
        daemon_reachable,
        keep_awake_enabled: keep_awake_enabled_env(),
        keep_awake_supported: gnomcode_platform::keepawake::is_supported(),
        keep_awake_holds: None,
        agent_clis,
        provider_issues: Vec::new(),
    };
    evaluate(&inputs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gnomcode_core::{QuotaSnapshot, QuotaWindow, TrustLevel};

    fn base_inputs<'a>(home: &Path, claude: &Path, db: Option<&'a Database>) -> DoctorInputs<'a> {
        DoctorInputs {
            gnomcode_home: home.to_path_buf(),
            claude_config_dir: claude.to_path_buf(),
            db_path: home.join("state.db"),
            repo_dir: Some(home.to_path_buf()),
            db,
            db_error: None,
            now: Utc::now(),
            in_daemon: true,
            daemon_reachable: None,
            keep_awake_enabled: true,
            keep_awake_supported: true,
            keep_awake_holds: Some(0),
            agent_clis: vec![AgentCliStatus {
                agent: "claude".into(),
                detected: true,
                binary: Some("/usr/local/bin/claude".into()),
                version: Some("1.0.0".into()),
                error: None,
            }],
            provider_issues: Vec::new(),
        }
    }

    fn check<'a>(report: &'a DoctorReport, id: &str) -> &'a DoctorCheck {
        report
            .checks
            .iter()
            .find(|c| c.id == id)
            .expect("missing check")
    }

    #[test]
    fn claude_signin_reads_only_expiry_and_never_leaks_tokens() {
        let dir = tempfile::tempdir().expect("tempdir");
        let claude = dir.path().join("claude");
        std::fs::create_dir_all(&claude).expect("mkdir");
        let now = Utc::now();
        let valid_exp = (now + chrono::Duration::hours(4)).timestamp_millis();
        std::fs::write(
            claude.join(".credentials.json"),
            serde_json::json!({
                "claudeAiOauth": {
                    "accessToken": "secret-access-token-xyz",
                    "refreshToken": "secret-refresh-token-abc",
                    "expiresAt": valid_exp
                }
            })
            .to_string(),
        )
        .expect("write");

        let mut inputs = base_inputs(dir.path(), &claude, None);
        inputs.now = now;
        let report = evaluate(&inputs);
        let c = check(&report, "claude_signin");
        assert_eq!(c.status, CheckStatus::Ok);
        let serialized = serde_json::to_string(&report).expect("json");
        assert!(!serialized.contains("secret-access-token-xyz"));
        assert!(!serialized.contains("secret-refresh-token-abc"));

        let soon_exp = (now + chrono::Duration::minutes(10)).timestamp_millis();
        std::fs::write(
            claude.join(".credentials.json"),
            serde_json::json!({"claudeAiOauth": {"accessToken": "t", "expiresAt": soon_exp}})
                .to_string(),
        )
        .expect("write");
        assert_eq!(
            check(&evaluate(&inputs), "claude_signin").status,
            CheckStatus::Warn
        );

        let expired_exp = (now - chrono::Duration::minutes(5)).timestamp_millis();
        std::fs::write(
            claude.join(".credentials.json"),
            serde_json::json!({"claudeAiOauth": {"accessToken": "t", "expiresAt": expired_exp}})
                .to_string(),
        )
        .expect("write");
        let expired_report = evaluate(&inputs);
        assert_eq!(
            check(&expired_report, "claude_signin").status,
            CheckStatus::Fail
        );
        assert_eq!(expired_report.status, CheckStatus::Fail);
        assert!(!expired_report.ok);
    }

    #[test]
    fn claude_integration_detects_hook_and_statusline() {
        let dir = tempfile::tempdir().expect("tempdir");
        let claude = dir.path().join("claude");
        std::fs::create_dir_all(&claude).expect("mkdir");
        let inputs = base_inputs(dir.path(), &claude, None);

        assert_eq!(
            check(&evaluate(&inputs), "claude_integration").status,
            CheckStatus::Warn
        );

        std::fs::write(
            claude.join("settings.json"),
            serde_json::json!({
                "hooks": {
                    "PreToolUse": [{
                        "hooks": [{"type": "command", "command": "gnomcode", "args": ["hook"]}]
                    }]
                },
                "statusLine": {"type": "command", "command": "\"/usr/bin/gnomcode\" statusline"}
            })
            .to_string(),
        )
        .expect("write");
        assert_eq!(
            check(&evaluate(&inputs), "claude_integration").status,
            CheckStatus::Ok
        );

        std::fs::write(claude.join("settings.json"), "{bad json").expect("write");
        assert_eq!(
            check(&evaluate(&inputs), "claude_integration").status,
            CheckStatus::Fail
        );
    }

    #[test]
    fn limit_sources_reports_enabled_and_reading_ages() {
        let dir = tempfile::tempdir().expect("tempdir");
        let claude = dir.path().join("claude");
        std::fs::create_dir_all(&claude).expect("mkdir");
        let mut db = Database::open_in_memory().expect("db");
        let now = Utc::now();
        std::fs::write(
            dir.path().join("vendor-limits.json"),
            serde_json::json!({"claude": true}).to_string(),
        )
        .expect("write");
        let snap = QuotaSnapshot {
            provider: "claude-code".into(),
            limit_id: "five_hour".into(),
            window: QuotaWindow::FiveHour,
            used_pct: 25.0,
            resets_at: Some(now + chrono::Duration::hours(2)),
            plan: None,
            basis: TrustLevel::Exact,
            observed_at: now - chrono::Duration::seconds(45),
        };
        let cp = gnomcode_store::SourceCheckpoint {
            file_path: "vendor:claude".into(),
            last_read_offset: 0,
            file_size: 0,
            fingerprint: "1".into(),
            last_ingested_at: now,
        };
        db.commit_batch(&[], &[], &[], &[], &[snap], &cp)
            .expect("commit");

        let mut inputs = base_inputs(dir.path(), &claude, Some(&db));
        inputs.now = now;
        let report = evaluate(&inputs);
        let c = check(&report, "limit_sources");
        assert_eq!(c.status, CheckStatus::Ok);
        let details = c.details.as_ref().expect("details");
        let providers = details["providers"].as_array().expect("providers");
        let claude_entry = providers
            .iter()
            .find(|p| p["provider"] == "claude-code")
            .expect("claude entry");
        assert_eq!(claude_entry["enabled"], true);
        assert_eq!(claude_entry["age_seconds"], 45);
    }

    #[test]
    fn disk_space_classifies_thresholds_correctly() {
        assert_eq!(classify_disk_bytes(6 * 1024 * 1024 * 1024), CheckStatus::Ok);
        assert_eq!(
            classify_disk_bytes(4 * 1024 * 1024 * 1024),
            CheckStatus::Warn
        );
        assert_eq!(classify_disk_bytes(50 * 1024 * 1024), CheckStatus::Fail);
    }

    #[test]
    fn keep_awake_and_daemon_and_clis_cover_warn_and_fail_states() {
        let dir = tempfile::tempdir().expect("tempdir");
        let claude = dir.path().join("claude");
        std::fs::create_dir_all(&claude).expect("mkdir");
        let mut inputs = base_inputs(dir.path(), &claude, None);
        inputs.in_daemon = false;
        inputs.daemon_reachable = Some(Err("connection refused".into()));
        inputs.keep_awake_enabled = false;
        inputs.agent_clis = Vec::new();

        let report = evaluate(&inputs);
        assert_eq!(check(&report, "daemon").status, CheckStatus::Fail);
        assert_eq!(check(&report, "keep_awake").status, CheckStatus::Warn);
        assert_eq!(check(&report, "agent_clis").status, CheckStatus::Warn);
        assert!(!report.ok);
    }
}
