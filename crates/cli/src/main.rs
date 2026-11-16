//! Gnomcode CLI entry point (`SPEC §3.6`, `§4.6`).
//!
//! Provides read-only CLI commands:
//! - `gnomcode status`: live & recent agent sessions, state, model, surface.
//! - `gnomcode report`: usage & cost breakdown by project/agent/model + quota pools + Plan Value ROI.
//! - `gnomcode doctor`: environment diagnostics, detected agent roots, and network requirements.

#![allow(clippy::print_stdout, clippy::print_stderr)]

mod evals;
mod integrate;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use gnomcode_core::pricing::calculate_plan_roi;
use gnomcode_ingest::ingest_all;
use gnomcode_platform::{
    attach_parent_console, default_db_path, discover_agent_roots, gnomcode_home,
};
use gnomcode_store::Database;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(
    name = "gnomcode",
    version,
    about = "Live mission control for AI coding agents (Claude Code, Codex, Antigravity)"
)]
struct Cli {
    /// Custom path to the SQLite database.
    #[arg(long, global = true)]
    db: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Show live and recent active agent sessions and current status.
    Status {
        /// Maximum number of sessions to display.
        #[arg(long, default_value_t = 10)]
        limit: usize,
    },
    /// Show token usage, API-equivalent spend, prompt cache savings, and quota pools.
    Report {
        /// Optional monthly subscription price in USD to calculate Plan Value ROI (e.g. 20.0).
        #[arg(long)]
        plan_usd: Option<f64>,
        /// Optional timeline range filter (`today`, `7d`, `30d`, `all`).
        #[arg(long, default_value = "all")]
        range: String,
    },
    /// Run diagnostic checks on local agent log directories and system environment.
    Doctor {
        /// Display all network endpoints required for licensing, cloud sync, and pricing updates.
        #[arg(long)]
        network: bool,
        /// Output diagnostic report as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Start the gnomcode background daemon and local web console.
    Daemon {
        /// Port to bind the local HTTP server to.
        #[arg(long, default_value_t = 41738)]
        port: u16,
        /// Open the web console in the default browser.
        #[arg(long)]
        open: bool,
    },
    /// Convenience shortcut: start the daemon and open the local console in your browser.
    Console {
        /// Port to bind the local HTTP server to.
        #[arg(long, default_value_t = 41738)]
        port: u16,
    },
    /// Open the local console, starting the daemon when needed.
    Open {
        /// Port to bind the local HTTP server to.
        #[arg(long, default_value_t = 41738)]
        port: u16,
    },
    /// Backwards compatible alias for `gnomcode console`.
    #[command(hide = true)]
    Dashboard {
        /// Port to bind the local HTTP server to.
        #[arg(long, default_value_t = 41738)]
        port: u16,
    },
    /// Install gnomcode to the local system, create Desktop/Start Menu shortcuts, and configure autostart.
    Install,
    /// Add or remove the optional Claude Code hooks (guardrail and phone approvals).
    Integrate {
        /// Agent to integrate (currently `claude`).
        agent: String,
        /// Remove gnomcode's hooks without changing other hooks.
        #[arg(long)]
        remove: bool,
        /// Do not install the approval hooks (phone and console approve/deny).
        #[arg(long)]
        no_approvals: bool,
    },
    /// Run repeatable quality evaluations (reports only).
    Evals {
        #[command(subcommand)]
        action: evals::EvalsCommands,
    },
    /// Internal fail-open hook callback.
    #[command(hide = true)]
    Hook,
    /// Opt in or out of exact limits read from a provider API with the agent's own sign-in.
    Limits {
        /// `enable`, `disable`, `status` or `check` (fetch once and print).
        action: String,
        /// Provider (`claude`).
        provider: String,
        /// With `enable claude`: do not run the official `claude` CLI to refresh an expiring sign-in.
        #[arg(long)]
        no_refresh: bool,
    },
    /// Opt Codex or Antigravity launches in or out of protocol mode, where approvals go to the
    /// console and phone instead of the terminal (off by default).
    ProtocolApprovals {
        /// `enable`, `disable` or `status`.
        action: String,
        /// Agent (`codex` or `antigravity`).
        agent: String,
    },
    /// Internal Claude Code status line: records account limits and prints them.
    #[command(hide = true)]
    Statusline,
    /// Launch an agent in a workspace using an independently owned PTY session.
    Launch {
        /// Agent to launch (claude, codex, agy).
        agent: String,
        /// Repository directory (defaults to current working directory).
        #[arg(short, long)]
        repo: Option<PathBuf>,
        /// Model identifier.
        #[arg(short, long)]
        model: Option<String>,
    },
    /// Stop an active agent session.
    Stop {
        /// Session ID to stop.
        session_id: String,
    },
    /// Attach to an active agent terminal session.
    Attach {
        /// Session ID to attach to.
        session_id: String,
    },
    /// List models reported by signed-in coding-agent CLI accounts.
    Models,
    /// Switch an ongoing session to another model using zero-token handoff.
    Switch {
        /// Session ID to switch.
        session_id: String,
        /// Target model identifier.
        #[arg(short, long)]
        model: String,
    },
    /// Resume an idle or rate-limited session when quota resets.
    Resume {
        /// Session ID to resume.
        session_id: String,
        /// Optional resume prompt instruction.
        #[arg(short, long)]
        prompt: Option<String>,
    },
    /// View or configure the model failover and routing policy.
    Policy {
        /// Set routing policy: ask_first, automatic, or deny_cross_provider.
        #[arg(long)]
        set: Option<String>,
    },
    /// Internal session host process.
    #[command(hide = true)]
    PtyHost {
        session_id: String,
        agent: String,
        repo: PathBuf,
        model: Option<String>,
        #[arg(long)]
        resume_session: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    if !matches!(
        std::env::args().nth(1).as_deref(),
        Some("daemon" | "pty-host" | "hook" | "statusline")
    ) {
        attach_parent_console();
    }

    let exe_is_installer = std::env::current_exe()
        .ok()
        .and_then(|p| {
            p.file_stem()
                .map(|s| s.to_string_lossy().to_ascii_lowercase())
        })
        .is_some_and(|name| name.contains("install") || name.contains("setup"));

    let cli = Cli::parse();
    let db_path = cli.db.unwrap_or_else(default_db_path);
    let default_cmd = if exe_is_installer {
        Commands::Install
    } else {
        Commands::Console { port: 41738 }
    };
    let command = cli.command.unwrap_or(default_cmd);

    match command {
        Commands::Status { limit } => cmd_status(&db_path, limit),
        Commands::Report { plan_usd, range } => cmd_report(&db_path, plan_usd, &range),
        Commands::Doctor { network, json } => cmd_doctor(&db_path, network, json).await,
        Commands::Daemon { port, open } => cmd_daemon(&db_path, port, open).await,
        Commands::Console { port } => cmd_daemon(&db_path, port, true).await,
        Commands::Open { port } => cmd_daemon(&db_path, port, true).await,
        Commands::Dashboard { port } => cmd_daemon(&db_path, port, true).await,
        Commands::Install => cmd_install(&db_path),
        Commands::Integrate {
            agent,
            remove,
            no_approvals,
        } => cmd_integrate(&agent, remove, !no_approvals),
        Commands::Evals { action } => evals::run(action),
        Commands::Hook => cmd_hook(),
        Commands::Limits {
            action,
            provider,
            no_refresh,
        } => cmd_limits(&action, &provider, no_refresh).await,
        Commands::ProtocolApprovals { action, agent } => cmd_protocol_approvals(&action, &agent),
        Commands::Statusline => {
            cmd_statusline();
            Ok(())
        }
        Commands::Launch { agent, repo, model } => cmd_launch(&agent, repo, model).await,
        Commands::Stop { session_id } => cmd_stop(&session_id).await,
        Commands::Attach { session_id } => cmd_attach(&session_id).await,
        Commands::Models => cmd_models().await,
        Commands::Switch { session_id, model } => cmd_switch(&session_id, &model, &db_path).await,
        Commands::Resume { session_id, prompt } => cmd_resume(&session_id, prompt, &db_path).await,
        Commands::Policy { set } => cmd_policy(set, &db_path).await,
        Commands::PtyHost {
            session_id,
            agent,
            repo,
            model,
            resume_session,
        } => {
            cmd_pty_host(
                &session_id,
                &agent,
                &repo,
                model.as_deref(),
                resume_session.as_deref(),
            )
            .await
        }
    }
}

fn cmd_hook() -> Result<()> {
    use std::io::Read as _;
    let mut input = String::new();
    if std::io::stdin()
        .take(1024 * 1024)
        .read_to_string(&mut input)
        .is_err()
    {
        return Ok(());
    }
    let Ok(payload) = serde_json::from_str::<serde_json::Value>(&input) else {
        return Ok(());
    };
    match payload.get("hook_event_name").and_then(|e| e.as_str()) {
        Some("PermissionRequest") => {
            hook_permission_request(&payload);
            return Ok(());
        }
        Some("Notification") => {
            hook_notification(&payload);
            return Ok(());
        }
        _ => {}
    }
    let Some(id) = payload
        .get("session_id")
        .and_then(|id| id.as_str())
        .map(str::to_owned)
    else {
        return Ok(());
    };

    let session_key = format!("claude-code:{id}");

    // Invariant 3: Fail open. Check whether daemon is alive and blocked-sessions.json is fresh (< 30s)
    let block_file = gnomcode_home().join("blocked-sessions.json");
    let is_stale = match std::fs::metadata(&block_file) {
        Ok(meta) => match meta.modified() {
            Ok(mtime) => match std::time::SystemTime::now().duration_since(mtime) {
                Ok(elapsed) => elapsed > std::time::Duration::from_secs(30),
                Err(_) => false,
            },
            Err(_) => true,
        },
        Err(_) => true,
    };

    // If file is stale or absent, fail open immediately
    if is_stale {
        return Ok(());
    }

    let Ok(raw) = std::fs::read(&block_file) else {
        return Ok(());
    };
    let Ok(blocked) = serde_json::from_slice::<Vec<String>>(&raw) else {
        return Ok(());
    };
    if blocked.iter().any(|session| session == &session_key) {
        eprintln!(
            "gnomcode guardrail: session threshold reached. Remove or raise the rule to continue."
        );
        std::process::exit(2);
    }
    Ok(())
}

/// `PermissionRequest` hook: ask the daemon whether a phone or the console can answer.
///
/// Prints a decision only when one arrives in time. Every other path (no daemon, slow
/// daemon, nobody watching, timeout, error) prints nothing and exits 0, so Claude shows its
/// own prompt (invariants 3 and 7). Nothing is ever allowed by default.
fn hook_permission_request(payload: &serde_json::Value) {
    use gnomcode_core::approval::{self, Decision};
    use gnomcode_daemon::approvals::client::{Endpoint, REPLY_BUDGET, call};
    let (Some(session), Some(tool)) = (
        payload.get("session_id").and_then(|v| v.as_str()),
        payload.get("tool_name").and_then(|v| v.as_str()),
    ) else {
        return;
    };
    let Some(endpoint) = Endpoint::from_daemon_info() else {
        return;
    };
    let summary = approval::summarize_tool(
        tool,
        payload
            .get("tool_input")
            .unwrap_or(&serde_json::Value::Null),
    );
    let request = serde_json::json!({
        "agent": "claude-code", "session_id": session, "tool": tool, "summary": summary
    });
    let Some(created) = call(
        &endpoint,
        "POST",
        "/api/v1/approvals",
        Some(&request),
        REPLY_BUDGET,
    ) else {
        return;
    };
    let Some(id) = created.get("id").and_then(|v| v.as_str()).filter(|id| {
        created.get("hold") == Some(&serde_json::Value::Bool(true)) && approval::valid_id(id)
    }) else {
        return;
    };
    let wait = std::time::Duration::from_secs(approval::HOLD_SECS + 5);
    let path = format!("/api/v1/approvals/{id}/wait");
    let (reply_tx, reply_rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = reply_tx.send(call(&endpoint, "GET", &path, None, wait));
    });
    let mut transcript = TranscriptWatch::new(payload.get("transcript_path"));
    let outcome = loop {
        match reply_rx.recv_timeout(std::time::Duration::from_millis(500)) {
            Ok(reply) => {
                break reply
                    .and_then(|r| r.get("outcome").and_then(|v| v.as_str()).map(str::to_owned));
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // Answered at the terminal: stop waiting. Exiting closes the socket, which
                // cancels the request in the daemon. Claude keeps its own result.
                if transcript.answered() {
                    std::process::exit(0);
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break None,
        }
    };
    match outcome.as_deref() {
        Some("allow") => println!("{}", approval::claude_hook_output(Decision::Allow)),
        Some("deny") => println!("{}", approval::claude_hook_output(Decision::Deny)),
        _ => {}
    }
}

/// Follows the agent transcript from where it was when the prompt appeared.
struct TranscriptWatch {
    path: Option<PathBuf>,
    offset: u64,
}

impl TranscriptWatch {
    fn new(path: Option<&serde_json::Value>) -> Self {
        let path = path.and_then(|v| v.as_str()).map(PathBuf::from);
        let offset = path
            .as_ref()
            .and_then(|p| std::fs::metadata(p).ok())
            .map_or(0, |meta| meta.len());
        Self { path, offset }
    }

    /// True once a tool result was appended after the prompt appeared.
    fn answered(&mut self) -> bool {
        use std::io::Seek as _;
        let Some(path) = &self.path else { return false };
        let Ok(mut file) = std::fs::File::open(path) else {
            return false;
        };
        let len = file.metadata().map_or(0, |meta| meta.len());
        if len <= self.offset || file.seek(std::io::SeekFrom::Start(self.offset)).is_err() {
            return false;
        }
        let mut chunk = Vec::new();
        if file.take(256 * 1024).read_to_end(&mut chunk).is_err() {
            return false;
        }
        self.offset += chunk.len() as u64;
        gnomcode_core::approval::transcript_shows_answer(&String::from_utf8_lossy(&chunk))
    }
}

/// `Notification` hook for permission prompts: detection only, no decision.
fn hook_notification(payload: &serde_json::Value) {
    use gnomcode_daemon::approvals::client::{Endpoint, REPLY_BUDGET, call};
    if payload.get("notification_type").and_then(|v| v.as_str()) != Some("permission_prompt") {
        return;
    }
    let (Some(session), Some(endpoint)) = (
        payload.get("session_id").and_then(|v| v.as_str()),
        Endpoint::from_daemon_info(),
    ) else {
        return;
    };
    let _ = call(
        &endpoint,
        "POST",
        "/api/v1/approvals/notify",
        Some(&serde_json::json!({"session_id": session})),
        REPLY_BUDGET,
    );
}

fn cmd_protocol_approvals(action: &str, agent: &str) -> Result<()> {
    match action {
        "enable" | "disable" => {
            gnomcode_pty::protocol_host::set_protocol_approvals(agent, action == "enable")?;
            println!(
                "{agent} protocol approvals {action}d. New {agent} sessions {} run through the agent's JSON-RPC protocol in a plain-text terminal; approvals are answered from the console or phone and anything unanswered is cancelled. Resumed sessions keep the normal terminal.",
                if action == "enable" {
                    "will"
                } else {
                    "will not"
                }
            );
        }
        "status" => println!(
            "{agent} protocol approvals {}",
            if gnomcode_pty::protocol_host::protocol_approvals_enabled(agent) {
                "enabled"
            } else {
                "disabled"
            }
        ),
        _ => anyhow::bail!("action must be enable, disable or status"),
    }
    Ok(())
}

async fn cmd_limits(action: &str, provider: &str, no_refresh: bool) -> Result<()> {
    use gnomcode_daemon::vendor_limits;
    anyhow::ensure!(
        vendor_limits::PROVIDERS.contains(&provider),
        "supported providers: {}",
        vendor_limits::PROVIDERS.join(", ")
    );
    match action {
        "enable" | "disable" => {
            vendor_limits::set_enabled(provider, action == "enable")?;
            if provider == "claude" && action == "enable" {
                vendor_limits::set_refresh_enabled(!no_refresh)?;
            }
            println!(
                "{provider} API limits {action}d. When enabled, Gnomcode reads {provider}'s local sign-in only to request your limits from the provider every 5 minutes; the token is never stored or logged."
            );
            if provider == "claude" && action == "enable" {
                println!(
                    "{}",
                    if no_refresh {
                        "Sign-in refresh is off: run `claude` in a terminal when the sign-in expires."
                    } else {
                        "When the sign-in is about to expire, Gnomcode runs the official `claude auth status` in the background to refresh it (no model usage). Use `--no-refresh` to turn this off."
                    }
                );
            }
        }
        "status" => {
            println!(
                "{provider} API limits {}",
                if vendor_limits::enabled(provider) {
                    "enabled"
                } else {
                    "disabled"
                }
            );
            if provider == "claude" && vendor_limits::enabled(provider) {
                println!(
                    "claude sign-in refresh {}",
                    if vendor_limits::refresh_enabled(provider) {
                        "on"
                    } else {
                        "off"
                    }
                );
            }
        }
        "check" => match vendor_limits::check(provider).await {
            Ok(quota) if quota.is_empty() => {
                println!("{provider}: signed in, but no limit windows reported")
            }
            Ok(quota) => {
                for pool in quota {
                    println!(
                        "{provider} {} {:?}: {:.0}% left{}",
                        pool.limit_id,
                        pool.window,
                        100.0 - pool.used_pct,
                        pool.resets_at
                            .map(|at| format!(
                                ", resets {}",
                                at.with_timezone(&chrono::Local).format("%a %H:%M")
                            ))
                            .unwrap_or_default()
                    );
                }
            }
            Err(reason) => println!("{provider}: {reason}"),
        },
        _ => anyhow::bail!("action must be enable, disable, status or check"),
    }
    Ok(())
}

/// Fail open: any problem prints nothing, so Claude Code's status line never breaks.
fn cmd_statusline() {
    use std::io::Read as _;
    let mut input = Vec::new();
    if std::io::stdin()
        .take(1024 * 1024)
        .read_to_end(&mut input)
        .is_err()
    {
        return;
    }
    let Some(limits) = serde_json::from_slice::<serde_json::Value>(&input)
        .ok()
        .and_then(|status| status.get("rate_limits").cloned())
    else {
        return;
    };
    // Persist only the limits, never the session, cwd or cost fields of the status input.
    let record = serde_json::json!({
        "observed_at": chrono::Utc::now().to_rfc3339(),
        "rate_limits": limits,
    });
    let path = gnomcode_home().join(gnomcode_ingest::CLAUDE_LIMITS_FILE);
    let temp = path.with_extension("json.tmp");
    if std::fs::create_dir_all(gnomcode_home()).is_ok()
        && std::fs::write(&temp, record.to_string()).is_ok()
    {
        let _ = std::fs::rename(&temp, &path);
    }
    let window = |key: &str, label: &str| {
        let used = limits.get(key)?.get("used_percentage")?.as_f64()?;
        Some(format!(
            "{label} {:.0}% left",
            (100.0 - used).clamp(0.0, 100.0)
        ))
    };
    let line: Vec<_> = [window("five_hour", "5h"), window("seven_day", "week")]
        .into_iter()
        .flatten()
        .collect();
    println!("{}", line.join(" · "));
}

/// On Windows, hooks and the status line run through the windowless `gnomcode-hook.exe` beside
/// the engine when it is installed, so GUI agents never flash a console window; elsewhere the
/// engine runs them itself.
fn hook_launcher(exe: &std::path::Path) -> std::path::PathBuf {
    let shim = exe.with_file_name("gnomcode-hook.exe");
    if cfg!(windows) && shim.is_file() {
        shim
    } else {
        exe.to_path_buf()
    }
}

/// Gnomcode owns the status line only when it installed it.
fn is_gnomcode_status_line(settings: &serde_json::Value) -> bool {
    settings
        .pointer("/statusLine/command")
        .and_then(|command| command.as_str())
        .is_some_and(|command| command.contains("gnomcode") && command.ends_with(" statusline"))
}

fn cmd_integrate(agent: &str, remove: bool, approvals: bool) -> Result<()> {
    anyhow::ensure!(agent == "claude", "only Claude Code hooks are supported");
    let config_dir = std::env::var("CLAUDE_CONFIG_DIR")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map_or_else(
            || gnomcode_platform::user_home_dir().join(".claude"),
            PathBuf::from,
        );
    std::fs::create_dir_all(&config_dir)?;
    let path = config_dir.join("settings.json");
    let mut settings: serde_json::Value = if path.exists() {
        serde_json::from_slice(&std::fs::read(&path)?)
            .context("Claude settings contain invalid JSON")?
    } else {
        serde_json::json!({})
    };
    anyhow::ensure!(
        settings.is_object(),
        "Claude settings must be a JSON object"
    );
    // Integrating a WSL distro from Windows writes the interop path the distro can run.
    let exe = match std::env::var("GNOMCODE_HOOK_EXE") {
        Ok(path) if !path.trim().is_empty() => path,
        _ => hook_launcher(&std::env::current_exe()?)
            .to_string_lossy()
            .into_owned(),
    };
    integrate::apply_hooks(&mut settings, &exe, remove, approvals)?;
    let status_line = if remove {
        if is_gnomcode_status_line(&settings) {
            settings
                .as_object_mut()
                .context("Claude settings must be a JSON object")?
                .remove("statusLine");
        }
        "removed"
    } else if settings.get("statusLine").is_none() || is_gnomcode_status_line(&settings) {
        settings["statusLine"] =
            serde_json::json!({"type": "command", "command": format!("\"{exe}\" statusline")});
        "installed"
    } else {
        "skipped: you already have a custom status line, so Claude limits stay unavailable"
    };
    if path.exists() {
        std::fs::copy(&path, config_dir.join("settings.json.gnomcode.bak"))?;
    }
    std::fs::write(&path, serde_json::to_vec_pretty(&settings)?)?;
    println!(
        "Claude Code guardrail hook {}. Limits status line {status_line}.",
        if remove { "removed" } else { "installed" }
    );
    if !remove {
        println!(
            "{}",
            if approvals {
                "Phone and console approvals installed. They wait only while a paired phone or the console is available; otherwise Claude's own prompt appears. Use --no-approvals to skip."
            } else {
                "Phone and console approvals not installed (--no-approvals)."
            }
        );
    }
    Ok(())
}

fn cmd_status(db_path: &std::path::Path, limit: usize) -> Result<()> {
    let mut db = Database::open(db_path).context("failed to open database")?;
    let _ = ingest_all(&mut db);

    let (sessions, _) = db
        .list_sessions_paginated(None, Some("ongoing"), 0, limit)
        .context("failed to query active sessions")?;

    println!("=== GNOMCODE ACTIVE SESSIONS ===");
    if sessions.is_empty() {
        println!("No active or recent agent sessions found.");
        println!("Run an agent (Claude Code, Codex, Antigravity) to populate.");
        return Ok(());
    }

    println!(
        "{:<20} {:<12} {:<10} {:<24} {:<10} {:<20}",
        "SESSION ID", "AGENT", "SURFACE", "MODEL", "STATE", "LAST ACTIVE"
    );
    println!("{}", "-".repeat(100));

    for s in sessions {
        let model = &s.model;
        let last_active = &s.last_event_at;
        println!(
            "{:<20} {:<12} {:<10} {:<24} {:<10} {:<20}",
            truncate_str(&s.id, 20),
            s.agent.as_str(),
            s.surface.as_str(),
            truncate_str(model, 24),
            s.state.as_str(),
            last_active
        );
    }
    println!();
    Ok(())
}

fn cmd_report(db_path: &std::path::Path, plan_usd: Option<f64>, range: &str) -> Result<()> {
    let mut db = Database::open(db_path).context("failed to open database")?;
    let _ = ingest_all(&mut db);

    if range != "all" {
        let tl = db
            .generate_usage_report_timeline(range, None, None)
            .context("failed to generate timeline report")?;

        println!(
            "=== GNOMCODE USAGE & COST REPORT ({}) ===",
            tl.range_label.to_uppercase()
        );
        if tl.providers.is_empty() {
            println!("No usage data recorded for this timeframe.");
            return Ok(());
        }

        println!(
            "{:<20} {:<24} {:>10} {:>10} {:>10} {:>10}",
            "PROVIDER", "MODEL", "IN (TOK)", "CACHED", "OUT (TOK)", "COST ($)"
        );
        println!("{}", "-".repeat(90));
        for prov in &tl.providers {
            for m in &prov.models {
                println!(
                    "{:<20} {:<24} {:>10} {:>10} {:>10} {:>10.2}",
                    truncate_str(&prov.provider, 20),
                    truncate_str(&m.model, 24),
                    format_tokens(m.input_tokens),
                    format_tokens(m.cache_read_tokens),
                    format_tokens(m.output_tokens),
                    m.cost_usd,
                );
            }
        }
        println!("{}", "-".repeat(90));
        println!(
            "TOTAL: API-Equivalent Spend: ${:.2}  |  Prompt Cache Savings: ${:.2} EXACT",
            tl.total_cost_usd, tl.total_cache_savings_usd
        );

        if let Some(price) = plan_usd {
            let (time_factor, period_label): (f64, &str) = match range {
                "today" | "24h" => (1.0 / 30.0, "day"),
                "7d" => (7.0 / 30.0, "7d"),
                "30d" => (1.0, "mo"),
                _ => (1.0, "period"),
            };
            let eff_plan = price * time_factor;
            let roi = calculate_plan_roi(tl.total_cost_usd, eff_plan);
            println!(
                "PLAN VALUE ROI: ${:.2}/{} Plan -> {:.1}x Value (Net Value: +${:.2}) EXACT",
                eff_plan, period_label, roi.multiplier, roi.net_savings_usd
            );
        }

        let quota = db
            .get_latest_quota_snapshots()
            .context("failed to get quota pools")?;
        if !quota.is_empty() {
            println!("\n=== PER-MODEL QUOTA POOLS ===");
            println!(
                "{:<12} {:<16} {:<10} {:>10} {:<25}",
                "PROVIDER", "LIMIT ID", "WINDOW", "USED %", "RESETS AT"
            );
            println!("{}", "-".repeat(78));
            for q in quota {
                let win_str = match q.window {
                    gnomcode_core::QuotaWindow::FiveHour => "5-hour",
                    gnomcode_core::QuotaWindow::Weekly => "weekly",
                    gnomcode_core::QuotaWindow::Monthly => "monthly",
                };
                let reset_str = q.resets_at.map_or_else(
                    || "N/A".to_owned(),
                    |dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
                );
                println!(
                    "{:<12} {:<16} {:<10} {:>9.1}% {:<25}",
                    q.provider, q.limit_id, win_str, q.used_pct, reset_str
                );
            }
        }
        println!();
        return Ok(());
    }

    let report_rows = db
        .generate_usage_report()
        .context("failed to generate usage report")?;

    println!("=== GNOMCODE USAGE & COST REPORT ===");
    if report_rows.is_empty() {
        println!("No usage data recorded yet.");
        return Ok(());
    }

    println!(
        "{:<20} {:<12} {:<22} {:>10} {:>10} {:>10} {:>10} {:>10}",
        "PROJECT", "AGENT", "MODEL", "IN (TOK)", "CACHED", "OUT (TOK)", "COST ($)", "SAVINGS"
    );
    println!("{}", "-".repeat(110));

    let mut total_cost = 0.0;
    let mut total_savings = 0.0;

    for r in &report_rows {
        total_cost += r.total_cost_usd;
        total_savings += r.total_cache_savings_usd;
        println!(
            "{:<20} {:<12} {:<22} {:>10} {:>10} {:>10} {:>10.2} {:>10.2}",
            truncate_str(&r.project_name, 20),
            r.agent.as_str(),
            truncate_str(&r.model, 22),
            format_tokens(r.input_tokens),
            format_tokens(r.cache_read_tokens),
            format_tokens(r.output_tokens),
            r.total_cost_usd,
            r.total_cache_savings_usd
        );
    }
    println!("{}", "-".repeat(110));
    println!(
        "TOTAL: API-Equivalent Spend: ${:.2}  |  Prompt Cache Savings: ${:.2} EXACT",
        total_cost, total_savings
    );

    if let Some(price) = plan_usd {
        let roi = calculate_plan_roi(total_cost, price);
        println!(
            "PLAN VALUE ROI: ${:.2}/mo Plan -> {:.1}x Value (Net Value: +${:.2}) EXACT",
            price, roi.multiplier, roi.net_savings_usd
        );
    }

    let quota = db
        .get_latest_quota_snapshots()
        .context("failed to get quota pools")?;
    if !quota.is_empty() {
        println!("\n=== PER-MODEL QUOTA POOLS ===");
        println!(
            "{:<12} {:<16} {:<10} {:>10} {:<25}",
            "PROVIDER", "LIMIT ID", "WINDOW", "USED %", "RESETS AT"
        );
        println!("{}", "-".repeat(78));
        for q in quota {
            let win_str = match q.window {
                gnomcode_core::QuotaWindow::FiveHour => "5-hour",
                gnomcode_core::QuotaWindow::Weekly => "weekly",
                gnomcode_core::QuotaWindow::Monthly => "monthly",
            };
            let reset_str = q.resets_at.map_or_else(
                || "N/A".to_owned(),
                |dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            );
            println!(
                "{:<12} {:<16} {:<10} {:>9.1}% {:<25}",
                q.provider, q.limit_id, win_str, q.used_pct, reset_str
            );
        }
    }
    println!();
    Ok(())
}

async fn cmd_doctor(db_path: &std::path::Path, network: bool, json: bool) -> Result<()> {
    use gnomcode_daemon::doctor::CheckStatus;

    let report = gnomcode_daemon::doctor::run_for_cli(db_path, std::env::current_dir().ok()).await;
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        if report.status == CheckStatus::Fail {
            std::process::exit(1);
        }
        return Ok(());
    }

    println!("=== GNOMCODE DOCTOR DIAGNOSTICS ===");
    println!("Home Directory: {}", report.gnomcode_home);
    println!("Database Path:  {}", report.db_path);
    let db_status = if db_path.exists() {
        "EXISTS (OK)"
    } else {
        "NOT FOUND (Will be initialized on first run)"
    };
    println!("Database File:  {}", db_status);

    println!("\nChecks:");
    for check in &report.checks {
        let badge = match check.status {
            CheckStatus::Ok => "OK",
            CheckStatus::Warn => "WARN",
            CheckStatus::Fail => "FAIL",
        };
        println!("  [{badge:<4}] {:<20} {}", check.label, check.summary);
        if let Some(fix) = &check.fix {
            println!("         Fix: {fix}");
        }
    }
    println!(
        "\nProvider setup with live checks: console Settings > Providers (#settings/providers)."
    );

    println!("\nDetected Agent Log Roots:");
    let roots = discover_agent_roots();
    for r in roots {
        let status = if r.exists { "FOUND" } else { "NOT FOUND" };
        println!(
            "  [{:<9}] {:<12} : {}",
            status,
            r.agent.as_str(),
            r.path.display()
        );
    }

    if network {
        println!(
            "\nNetwork connections (configuration and user actions determine actual traffic):"
        );
        println!("  127.0.0.1                Local daemon and PTY services; ports vary");
        println!(
            "  Account service          Account authentication, metadata sync and Remote entitlement; configurable"
        );
        println!(
            "  Remote relay             Outbound encrypted Remote traffic after pairing; configurable"
        );
        println!("  raw.githubusercontent.com  Optional pricing refresh when requested");
        println!(
            "  Browser / agent CLI      Visited websites and installed agents use their own services"
        );
        println!("  No connections are tested by this report.");
    }

    println!("\nDiagnostics complete.");
    if report.status == CheckStatus::Fail {
        std::process::exit(1);
    }
    Ok(())
}

fn format_tokens(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        return s.to_owned();
    }
    let keep = max_len.saturating_sub(3);
    let mut out: String = s.chars().take(keep).collect();
    out.push_str("...");
    out
}

async fn cmd_daemon(db_path: &std::path::Path, port: u16, open: bool) -> Result<()> {
    let (mut lock, info) = match gnomcode_daemon::DaemonLock::hold(port) {
        Ok(res) => res,
        Err(gnomcode_daemon::Error::AlreadyRunning) => {
            if let Some(existing) = gnomcode_daemon::read_daemon_info() {
                let local_url = format!(
                    "http://127.0.0.1:{}/?token={}",
                    existing.port, existing.token
                );
                let console_url =
                    std::env::var("GNOMCODE_CONSOLE_URL").unwrap_or_else(|_| local_url.clone());
                println!("gnomcode is already active (PID {}).", existing.pid);
                if open {
                    open_browser(&console_url);
                    println!("Opened the local console.");
                }
                return Ok(());
            }
            return Err(anyhow::anyhow!(
                "daemon is already running but daemon.json could not be read"
            ));
        }
        Err(e) => return Err(e.into()),
    };

    let orchestrator =
        gnomcode_daemon::DaemonOrchestrator::new(db_path, std::time::Duration::from_secs(2))
            .context("failed to initialize daemon orchestrator")?;
    let _reconciler_handle = orchestrator.spawn_reconciler();
    let _automation_handle = orchestrator.spawn_automation();
    let _vendor_limits_handle = gnomcode_daemon::vendor_limits::spawn(orchestrator.db());
    let remote = Arc::new(gnomcode_daemon::remote::RemoteManager::new());
    let restore_remote = remote.clone();
    let restore_db = orchestrator.db();
    tokio::spawn(async move {
        if restore_remote.restore(restore_db).await.is_err() {
            tracing::warn!("Saved remote connection could not be restored");
        }
    });
    let _cloud_sync_handle =
        gnomcode_daemon::sync::spawn_cloud_sync(orchestrator.db(), remote.clone());
    let _update_handle = gnomcode_daemon::update::spawn(orchestrator.db());

    let state = gnomcode_daemon::api::AppState {
        db: orchestrator.db(),
        bearer_token: info.token.clone(),
        orchestrator: Some(Arc::new(orchestrator)),
        remote,
    };

    let router = gnomcode_daemon::api::build_router(state);
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context(format!("failed to bind loopback TCP listener on {addr}"))?;
    // Publish daemon.json only after SQLite initialization and TCP bind succeed so /health is ready immediately.
    lock.publish(&info)?;

    let local_url = format!("http://127.0.0.1:{port}/?token={}", info.token);
    let console_url = std::env::var("GNOMCODE_CONSOLE_URL").unwrap_or_else(|_| local_url.clone());
    println!("=== GNOMCODE DAEMON STARTED ===");
    println!("Status: Running");
    println!("Loopback Port: {port}");
    println!("PID: {}", info.pid);
    println!("Local console is available on loopback.");
    println!("Press Ctrl+C to terminate the daemon.");

    if open {
        open_browser(&console_url);
    }

    axum::serve(listener, router)
        .await
        .context("daemon HTTP server terminated unexpectedly")?;
    drop(lock);
    Ok(())
}

fn open_browser(url: &str) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt as _;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;

        let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".to_owned());
        let install_dir = std::path::PathBuf::from(&local_app_data).join("Gnomcode");

        let desktop = std::env::current_exe()
            .ok()
            .and_then(|exe| {
                exe.parent()
                    .map(|base| base.join("desktop").join("gnomcode-desktop.exe"))
            })
            .filter(|exe| exe.is_file())
            .unwrap_or_else(|| install_dir.join("desktop").join("gnomcode-desktop.exe"));
        if desktop.is_file()
            && std::process::Command::new(desktop)
                .args(
                    url.find('#')
                        .map(|index| vec!["--gnomcode-open-hash", &url[index..]])
                        .unwrap_or_default(),
                )
                .creation_flags(CREATE_NO_WINDOW)
                .spawn()
                .is_ok()
        {
            return;
        }
        // Ensure tray background service is running
        let run_tray = install_dir.join("run-tray.vbs");
        if run_tray.exists() {
            let _ = std::process::Command::new("wscript.exe")
                .args(["//B", &run_tray.to_string_lossy()])
                .creation_flags(CREATE_NO_WINDOW)
                .spawn();
        }

        let app_arg = format!("--app={url}");
        let profile_arg = format!("--user-data-dir={local_app_data}\\Gnomcode\\AppProfile");
        let candidate_paths = [
            r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
            r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
            "msedge",
            r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
            "chrome",
        ];
        let mut opened = false;
        for exe in candidate_paths {
            if std::process::Command::new(exe)
                .args([
                    &app_arg,
                    &profile_arg,
                    "--window-name=Gnomcode",
                    "--window-size=1280,850",
                    "--no-first-run",
                    "--no-default-browser-check",
                ])
                .spawn()
                .is_ok()
            {
                opened = true;
                break;
            }
        }
        if !opened {
            let _ = std::process::Command::new("cmd")
                .args(["/C", "start", url])
                .creation_flags(CREATE_NO_WINDOW)
                .spawn();
        }
    }
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
}

#[cfg(target_os = "windows")]
const GNOMCODE_ICO: &[u8] = include_bytes!("../../../scripts/gnomcode.ico");
#[cfg(target_os = "windows")]
const TRAY_PS1: &str = include_str!("../../../scripts/tray.ps1");
#[cfg(target_os = "windows")]
const RUN_TRAY_VBS: &str = include_str!("../../../scripts/run-tray.vbs");
#[cfg(target_os = "windows")]
const OPEN_CONSOLE_VBS: &str = include_str!("../../../scripts/open-console.vbs");
#[cfg(target_os = "windows")]
const OPEN_DASH_VBS: &str = include_str!("../../../scripts/open-dashboard.vbs");

fn cmd_install(db_path: &std::path::Path) -> Result<()> {
    println!("=== GNOMCODE NATIVE INSTALLER ===");
    if let Ok(mut db) = Database::open(db_path) {
        let _ = ingest_all(&mut db);
    }

    let current_exe = std::env::current_exe().context("failed to locate running executable")?;

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt as _;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;

        let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| {
            gnomcode_home()
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .join("AppData")
                .join("Local")
                .to_string_lossy()
                .into_owned()
        });
        let install_dir = PathBuf::from(local_app_data).join("gnomcode");
        std::fs::create_dir_all(&install_dir).context("failed to create installation directory")?;

        let dest_exe = install_dir.join("gnomcode.exe");
        if current_exe != dest_exe {
            let my_pid = std::process::id();
            let kill_cmd = format!(
                "Get-Process -Name gnomcode -ErrorAction SilentlyContinue | Where-Object {{ $_.Id -ne {my_pid} }} | Stop-Process -Force -ErrorAction SilentlyContinue"
            );
            let _ = std::process::Command::new("powershell")
                .args(["-NoProfile", "-Command", &kill_cmd])
                .creation_flags(CREATE_NO_WINDOW)
                .status();
            std::thread::sleep(std::time::Duration::from_millis(300));
            std::fs::copy(&current_exe, &dest_exe)
                .context("failed to copy gnomcode.exe to install directory")?;
        }

        std::fs::write(install_dir.join("gnomcode.ico"), GNOMCODE_ICO)?;
        std::fs::write(install_dir.join("tray.ps1"), TRAY_PS1)?;
        std::fs::write(install_dir.join("run-tray.vbs"), RUN_TRAY_VBS)?;
        std::fs::write(install_dir.join("open-console.vbs"), OPEN_CONSOLE_VBS)?;
        std::fs::write(install_dir.join("open-dashboard.vbs"), OPEN_DASH_VBS)?;

        let dir_str = install_dir.to_string_lossy();
        let ps_setup = format!(
            r#"
$InstallDir = '{dir_str}'
$ExeDest = "$InstallDir\gnomcode.exe"
$IcoDest = "$InstallDir\gnomcode.ico"
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$InstallDir*") {{
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
}}
$RunKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
$DesktopExe = "$InstallDir\desktop\gnomcode-desktop.exe"
$AutostartCmd = if (Test-Path $DesktopExe) {{ "`"$DesktopExe`" --background" }} else {{ "wscript.exe `"$InstallDir\run-tray.vbs`"" }}
Set-ItemProperty -Path $RunKey -Name "Gnomcode" -Value $AutostartCmd -Force
$ApprovedKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run"
if (Test-Path $ApprovedKey) {{ Remove-ItemProperty -Path $ApprovedKey -Name "Gnomcode" -ErrorAction SilentlyContinue }}
$WshShell = New-Object -ComObject WScript.Shell
$DesktopDir = [Environment]::GetFolderPath("Desktop")
foreach ($lnkPath in @("$DesktopDir\Gnomcode Console.lnk", "$DesktopDir\Gnomcode.lnk", "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Gnomcode Console.lnk")) {{
    $Shortcut = $WshShell.CreateShortcut($lnkPath)
    $Shortcut.TargetPath = "wscript.exe"
    $Shortcut.Arguments = "`"$InstallDir\open-console.vbs`""
    $Shortcut.WorkingDirectory = $InstallDir
    $Shortcut.IconLocation = "$IcoDest,0"
    $Shortcut.Description = "gnomcode console"
    $Shortcut.Save()
}}
Start-Process -FilePath "wscript.exe" -ArgumentList "//B `"$InstallDir\run-tray.vbs`"" -WindowStyle Hidden
Start-Process -FilePath "wscript.exe" -ArgumentList "//B `"$InstallDir\open-console.vbs`"" -WindowStyle Hidden
Add-Type -AssemblyName System.Windows.Forms
[System.Windows.Forms.MessageBox]::Show("gnomcode has been installed to $InstallDir.`n`n- Desktop & Start Menu shortcuts created for gnomcode Console`n- Configured to start automatically on login`n- Mission Control is now running in your system tray.", "gnomcode Setup Complete", [System.Windows.Forms.MessageBoxButtons]::OK, [System.Windows.Forms.MessageBoxIcon]::Information) | Out-Null
"#
        );

        let _ = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                &ps_setup,
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .status();

        println!("Installed gnomcode to {}", install_dir.display());
    }

    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_owned());
        let bin_dir = PathBuf::from(home).join(".local").join("bin");
        std::fs::create_dir_all(&bin_dir)?;
        let dest = bin_dir.join("gnomcode");
        if current_exe != dest {
            std::fs::copy(&current_exe, &dest)?;
        }
        println!("Installed gnomcode to {}", dest.display());
    }

    Ok(())
}

async fn cmd_launch(agent: &str, repo: Option<PathBuf>, model: Option<String>) -> Result<()> {
    let repo_path = match repo {
        Some(r) => r,
        None => std::env::current_dir()?,
    };

    let session_id = format!(
        "{}-{}",
        agent,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    );

    println!(
        "[WAIT] Launching {} session in {}...",
        agent,
        repo_path.display()
    );

    let record =
        gnomcode_pty::launch_session(&session_id, agent, &repo_path, model.as_deref()).await?;

    println!("[RUN] Session launched successfully");
    println!("Session ID: {}", record.session_id);
    println!("Agent:      {}", record.agent);
    if let Some(m) = &record.model {
        println!("Model:      {}", m);
    }
    println!("Host PID:   {}", record.host_pid);
    println!("Child PID:  {}", record.child_pid);
    println!("Stream:     ws://127.0.0.1:{}/stream", record.port);
    println!();
    println!("To attach to the terminal:");
    println!("  gnomcode attach {}", record.session_id);
    println!("To stop the session:");
    println!("  gnomcode stop {}", record.session_id);

    Ok(())
}

async fn cmd_stop(session_id: &str) -> Result<()> {
    println!("[WAIT] Stopping session {}...", session_id);
    gnomcode_pty::stop_session(session_id).await?;
    println!("[IDLE] Session {} stopped", session_id);
    Ok(())
}

async fn cmd_attach(session_id: &str) -> Result<()> {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::connect_async;
    use tokio_tungstenite::tungstenite::Message;

    let record = gnomcode_pty::load_session_record(session_id)?;
    let url = format!(
        "ws://127.0.0.1:{}/stream?token={}",
        record.port, record.token
    );

    println!("Connecting to session {}...", session_id);
    println!("Press Ctrl+C to detach without stopping the agent.\n");

    let (ws_stream, _) = connect_async(&url).await?;
    let (mut write_half, mut read_half) = ws_stream.split();

    // Spawn task to read stdout from WS
    let read_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = read_half.next().await {
            match msg {
                Message::Binary(bytes) => {
                    let _ = std::io::stdout().write_all(&bytes);
                    let _ = std::io::stdout().flush();
                }
                Message::Text(text) => {
                    let _ = std::io::stdout().write_all(text.as_bytes());
                    let _ = std::io::stdout().flush();
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    // Read user stdin on a blocking thread
    let (stdin_tx, mut stdin_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(32);
    std::thread::spawn(move || {
        let mut buffer = [0u8; 1024];
        let mut stdin = std::io::stdin();
        while let Ok(n) = stdin.read(&mut buffer) {
            if n == 0 || stdin_tx.blocking_send(buffer[..n].to_vec()).is_err() {
                break;
            }
        }
    });

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                println!("\nDetached from session {}. The agent process is still running.", session_id);
                break;
            }
            Some(input) = stdin_rx.recv() => {
                if write_half.send(Message::Binary(input.into())).await.is_err() {
                    break;
                }
            }
        }
    }

    read_task.abort();
    Ok(())
}

async fn cmd_models() -> Result<()> {
    for provider in gnomcode_daemon::model_catalog::discover().await {
        println!(
            "{} [{}]: {}",
            provider.name, provider.status, provider.message
        );
        for model in provider.models {
            println!("  {:<28} {}", model.model, model.availability_source);
        }
    }
    Ok(())
}

async fn cmd_switch(session_id: &str, model: &str, db_path: &Path) -> Result<()> {
    let db = gnomcode_store::Database::open(db_path)?;
    let record =
        gnomcode_daemon::switch_session(&tokio::sync::Mutex::new(db), session_id, model, None)
            .await?;
    println!(
        "[RUN] Switched to {} in session {}",
        model, record.session_id
    );
    println!("Attach with: gnomcode attach {}", record.session_id);
    Ok(())
}

async fn cmd_resume(session_id: &str, prompt: Option<String>, db_path: &Path) -> Result<()> {
    anyhow::ensure!(
        prompt.as_deref().is_none_or(|p| p == "continue"),
        "Only the default resume prompt is supported"
    );
    println!("[WAIT] Arming resume queue for session {}...", session_id);
    let db = gnomcode_store::Database::open(db_path)?;
    let target = db
        .get_session_focus_target(session_id)?
        .context("Session not found")?;
    let agent = match target.agent.as_str() {
        "claude-code" => gnomcode_core::AgentId::ClaudeCode,
        "codex" => gnomcode_core::AgentId::Codex,
        _ => anyhow::bail!("Agent does not support resume"),
    };
    let cwd = target.repo_root.context(
        "Session has no recorded folder to resume in (Codex chats, temporary and unassigned sessions cannot be resumed)",
    )?;
    let entry = gnomcode_core::rules::ResumeQueueEntry {
        session_id: session_id.to_owned(),
        agent,
        cwd,
        resume_prompt: prompt.unwrap_or_else(|| "continue".to_owned()),
        max_runs: 1,
        runs_completed: 0,
        eligible_after: chrono::Utc::now(),
    };
    db.arm_resume_queue(&entry)?;
    println!("[RUN] Session {} armed in Resume Queue", session_id);
    println!("Session will resume automatically upon quota reset.");
    Ok(())
}

async fn cmd_policy(set: Option<String>, db_path: &Path) -> Result<()> {
    let db = gnomcode_store::Database::open(db_path)?;
    if let Some(new_policy) = set {
        let policy = gnomcode_core::RoutingPolicy::parse_policy(&new_policy).ok_or_else(|| {
            anyhow::anyhow!(
                "Invalid policy: {new_policy}. Valid options: ask_first, automatic, deny_cross_provider"
            )
        })?;
        db.save_routing_policy(policy)?;
        println!("[RUN] Routing policy updated to: {}", policy.as_str());
    } else {
        let policy = db
            .get_routing_policy()
            .unwrap_or(gnomcode_core::RoutingPolicy::AskFirst);
        println!("Current routing policy: {}", policy.as_str());
        println!("Available policies:     ask_first, automatic, deny_cross_provider");
    }
    Ok(())
}

async fn cmd_pty_host(
    session_id: &str,
    agent: &str,
    repo: &Path,
    model: Option<&str>,
    resume_session: Option<&str>,
) -> Result<()> {
    // Opt-in protocol mode (ADR 0057) applies to new launches only; the terminal stays the default.
    if resume_session.is_none() && gnomcode_pty::protocol_host::protocol_approvals_enabled(agent) {
        let daemon: gnomcode_pty::protocol_host::DaemonEndpoint = std::sync::Arc::new(|| {
            gnomcode_daemon::approvals::client::Endpoint::from_daemon_info()
                .map(|endpoint| (endpoint.port, endpoint.token))
        });
        return gnomcode_pty::protocol_host::run_protocol_host(
            session_id, agent, repo, model, daemon,
        )
        .await
        .map_err(|e| anyhow::anyhow!("Protocol session host failed: {e}"));
    }
    gnomcode_pty::run_pty_host_resuming(session_id, agent, repo, model, None, resume_session)
        .await
        .map_err(|e| anyhow::anyhow!("Session host failed: {e}"))
}
