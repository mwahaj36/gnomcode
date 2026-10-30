//! Spawnt CLI entry point (`SPEC §3.6`, `§4.6`).
//!
//! Provides read-only CLI commands:
//! - `spawnt status`: live & recent agent sessions, state, model, surface.
//! - `spawnt report`: usage & cost breakdown by project/agent/model + quota pools + Plan Value ROI.
//! - `spawnt doctor`: environment diagnostics, detected agent roots, and network requirements.

#![allow(clippy::print_stdout, clippy::print_stderr)]

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use spawnt_core::pricing::calculate_plan_roi;
use spawnt_ingest::ingest_all;
use spawnt_platform::{attach_parent_console, default_db_path, discover_agent_roots, spawnt_home};
use spawnt_store::Database;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(
    name = "spawnt",
    version,
    about = "Live mission control for AI coding agents (Claude Code, Codex, Antigravity, Gemini CLI)"
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
    },
    /// Start the spawnt background daemon and local web console.
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
    /// Backwards compatible alias for `spawnt console`.
    #[command(hide = true)]
    Dashboard {
        /// Port to bind the local HTTP server to.
        #[arg(long, default_value_t = 41738)]
        port: u16,
    },
    /// Install spawnt to the local system, create Desktop/Start Menu shortcuts, and configure autostart.
    Install,
    /// Add or remove the optional Claude Code guardrail hook.
    Integrate {
        /// Agent to integrate (currently `claude`).
        agent: String,
        /// Remove spawnt's hook without changing other hooks.
        #[arg(long)]
        remove: bool,
    },
    /// Manage spawnt offline Pro / Team license.
    License {
        #[command(subcommand)]
        action: LicenseCommands,
    },
    /// Internal fail-open hook callback.
    #[command(hide = true)]
    Hook,
    /// Launch an agent in a workspace using an independently owned PTY session.
    Launch {
        /// Agent to launch (claude, codex, gemini).
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
    /// List supported models with live quota headroom and pricing.
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

#[derive(Subcommand, Debug)]
enum LicenseCommands {
    /// Show current local license tier and status.
    Status,
    /// Activate an Ed25519-signed license key from a file or JSON string.
    Activate {
        /// File path or raw JSON string of the license key.
        key: String,
    },
    /// Remove the saved offline license. Local features remain free.
    Remove,
}

#[tokio::main]
async fn main() -> Result<()> {
    attach_parent_console();

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
        Commands::Doctor { network } => cmd_doctor(&db_path, network),
        Commands::Daemon { port, open } => cmd_daemon(&db_path, port, open).await,
        Commands::Console { port } => cmd_daemon(&db_path, port, true).await,
        Commands::Open { port } => cmd_daemon(&db_path, port, true).await,
        Commands::Dashboard { port } => cmd_daemon(&db_path, port, true).await,
        Commands::Install => cmd_install(&db_path),
        Commands::Integrate { agent, remove } => cmd_integrate(&agent, remove),
        Commands::License { action } => cmd_license(action),
        Commands::Hook => cmd_hook(),
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

fn cmd_license(action: LicenseCommands) -> Result<()> {
    match action {
        LicenseCommands::Status => {
            let key_path = spawnt_home().join("license.key");
            if !key_path.exists() {
                println!("Tier: Free (freeware, forever)");
                println!(
                    "All local features are free, including guardrails, Resume Queue, and export."
                );
                println!("No local license is required.");
                return Ok(());
            }
            let bytes = std::fs::read(&key_path)?;
            let license: spawnt_core::license::LicenseFile = serde_json::from_slice(&bytes)?;
            match spawnt_core::license::verify_license(
                &license,
                spawnt_core::license::SPAWNT_LICENSE_PUBKEY_HEX,
            ) {
                Ok(tier) => {
                    println!(
                        "Tier: {}",
                        match tier {
                            spawnt_core::license::Tier::Pro => "Pro ($19 perpetual)",
                            spawnt_core::license::Tier::Team => "Team",
                            spawnt_core::license::Tier::Free => "Free",
                        }
                    );
                    println!("Licensee: {}", license.payload.licensee);
                    println!("Issued At: {}", license.payload.issued_at);
                    if let Some(ref exp) = license.payload.expires_at {
                        println!("Expires: {}", exp);
                    } else {
                        println!("Status: Active (perpetual)");
                    }
                }
                Err(err) => {
                    println!("Tier: Free (invalid or expired license file: {err})");
                }
            }
            Ok(())
        }
        LicenseCommands::Activate { key } => {
            let content = if std::path::Path::new(&key).is_file() {
                std::fs::read_to_string(&key)?
            } else {
                key
            };
            let license: spawnt_core::license::LicenseFile =
                serde_json::from_str(content.trim()).context("invalid license JSON format")?;
            let tier = spawnt_core::license::verify_license(
                &license,
                spawnt_core::license::SPAWNT_LICENSE_PUBKEY_HEX,
            )
            .context("license cryptographic verification failed")?;
            anyhow::ensure!(
                matches!(
                    tier,
                    spawnt_core::license::Tier::Pro | spawnt_core::license::Tier::Team
                ),
                "License is for Free tier; Pro or Team required."
            );
            std::fs::create_dir_all(spawnt_home())?;
            std::fs::write(
                spawnt_home().join("license.key"),
                serde_json::to_vec_pretty(&license)?,
            )?;
            println!(
                "License successfully activated for {} ({:?})!",
                license.payload.licensee, tier
            );
            Ok(())
        }
        LicenseCommands::Remove => {
            let key_path = spawnt_home().join("license.key");
            if key_path.exists() {
                std::fs::remove_file(key_path)?;
            }
            println!("License removed. Reverted to Free tier.");
            Ok(())
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
    let Some(id) = serde_json::from_str::<serde_json::Value>(&input)
        .ok()
        .and_then(|value| {
            value
                .get("session_id")
                .and_then(|id| id.as_str())
                .map(str::to_owned)
        })
    else {
        return Ok(());
    };

    let session_key = format!("claude-code:{id}");

    // Invariant 3: Fail open. Check whether daemon is alive and blocked-sessions.json is fresh (< 30s)
    let block_file = spawnt_home().join("blocked-sessions.json");
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
            "spawnt guardrail: session threshold reached. Remove or raise the rule to continue."
        );
        std::process::exit(2);
    }
    Ok(())
}

fn cmd_integrate(agent: &str, remove: bool) -> Result<()> {
    anyhow::ensure!(agent == "claude", "only Claude Code hooks are supported");
    let config_dir = std::env::var("CLAUDE_CONFIG_DIR")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map_or_else(
            || spawnt_platform::user_home_dir().join(".claude"),
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
    let exe = std::env::current_exe()?.to_string_lossy().into_owned();
    let entries = settings.pointer_mut("/hooks/PreToolUse");
    if let Some(entries) = entries {
        let hooks = entries
            .as_array_mut()
            .context("Claude PreToolUse hooks must be an array")?;
        hooks.retain(|entry| {
            !entry
                .get("hooks")
                .and_then(|h| h.as_array())
                .is_some_and(|handlers| {
                    handlers.iter().any(|handler| {
                        let cmd = handler
                            .get("command")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let is_spawnt_hook = cmd == exe
                            || std::path::Path::new(cmd)
                                .file_name()
                                .and_then(|name| name.to_str())
                                .is_some_and(|name| ["spawnt", "spawnt.exe"].contains(&name));
                        is_spawnt_hook && handler.get("args") == Some(&serde_json::json!(["hook"]))
                    })
                })
        });
    }
    if !remove {
        let entry = serde_json::json!({"hooks": [{"type": "command", "command": exe, "args": ["hook"], "timeout": 1, "onFailure": "continue"}]});
        if settings.get("hooks").is_none() {
            settings["hooks"] = serde_json::json!({});
        }
        if settings["hooks"].get("PreToolUse").is_none() {
            settings["hooks"]["PreToolUse"] = serde_json::json!([]);
        }
        settings["hooks"]["PreToolUse"]
            .as_array_mut()
            .context("Claude PreToolUse hooks must be an array")?
            .push(entry);
    }
    if path.exists() {
        std::fs::copy(&path, config_dir.join("settings.json.spawnt.bak"))?;
    }
    std::fs::write(&path, serde_json::to_vec_pretty(&settings)?)?;
    println!(
        "Claude Code guardrail hook {}.",
        if remove { "removed" } else { "installed" }
    );
    Ok(())
}

fn cmd_status(db_path: &std::path::Path, limit: usize) -> Result<()> {
    let mut db = Database::open(db_path).context("failed to open database")?;
    let _ = ingest_all(&mut db);

    let (sessions, _) = db
        .list_sessions_paginated(None, Some("ongoing"), 0, limit)
        .context("failed to query active sessions")?;

    println!("=== SPAWNT ACTIVE SESSIONS ===");
    if sessions.is_empty() {
        println!("No active or recent agent sessions found.");
        println!("Run an agent (Claude Code, Codex, Antigravity, Gemini CLI) to populate.");
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
            "=== SPAWNT USAGE & COST REPORT ({}) ===",
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
                    spawnt_core::QuotaWindow::FiveHour => "5-hour",
                    spawnt_core::QuotaWindow::Weekly => "weekly",
                    spawnt_core::QuotaWindow::Monthly => "monthly",
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

    println!("=== SPAWNT USAGE & COST REPORT ===");
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
                spawnt_core::QuotaWindow::FiveHour => "5-hour",
                spawnt_core::QuotaWindow::Weekly => "weekly",
                spawnt_core::QuotaWindow::Monthly => "monthly",
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

fn cmd_doctor(db_path: &std::path::Path, network: bool) -> Result<()> {
    println!("=== SPAWNT DOCTOR DIAGNOSTICS ===");
    println!("Home Directory: {}", spawnt_home().display());
    println!("Database Path:  {}", db_path.display());

    let db_status = if db_path.exists() {
        "EXISTS (OK)"
    } else {
        "NOT FOUND (Will be initialized on first run)"
    };
    println!("Database File:  {}", db_status);

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
        println!("\nNetwork Endpoints (All Loopback or Outbound Opt-in):");
        println!("  127.0.0.1:41738           Local Webhook / IPC Loopback Server");
        println!("  https://*.supabase.co     Opt-in Multi-Device Cloud Sync (M6)");
    }

    println!("\nDiagnostics complete.");
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
    let (lock, info) = match spawnt_daemon::DaemonLock::acquire(port) {
        Ok(res) => res,
        Err(spawnt_daemon::Error::AlreadyRunning) => {
            if let Some(existing) = spawnt_daemon::read_daemon_info() {
                let local_url = format!(
                    "http://127.0.0.1:{}/?token={}",
                    existing.port, existing.token
                );
                let console_url =
                    std::env::var("SPAWNT_CONSOLE_URL").unwrap_or_else(|_| local_url.clone());
                println!("spawnt is already active (PID {}).", existing.pid);
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
        spawnt_daemon::DaemonOrchestrator::new(db_path, std::time::Duration::from_secs(2))
            .context("failed to initialize daemon orchestrator")?;
    let _reconciler_handle = orchestrator.spawn_reconciler();
    let remote = Arc::new(spawnt_daemon::remote::RemoteManager::new());
    let restore_remote = remote.clone();
    let restore_db = orchestrator.db();
    tokio::spawn(async move {
        if restore_remote.restore(restore_db).await.is_err() {
            tracing::warn!("Saved remote connection could not be restored");
        }
    });
    let _cloud_sync_handle =
        spawnt_daemon::sync::spawn_cloud_sync(orchestrator.db(), remote.clone());

    let state = spawnt_daemon::api::AppState {
        db: orchestrator.db(),
        bearer_token: info.token.clone(),
        orchestrator: Some(Arc::new(orchestrator)),
        remote,
    };

    let router = spawnt_daemon::api::build_router(state);
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context(format!("failed to bind loopback TCP listener on {addr}"))?;

    let local_url = format!("http://127.0.0.1:{port}/?token={}", info.token);
    let console_url = std::env::var("SPAWNT_CONSOLE_URL").unwrap_or_else(|_| local_url.clone());
    println!("=== SPAWNT DAEMON STARTED ===");
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
        let install_dir = std::path::PathBuf::from(&local_app_data).join("Spawnt");

        // Ensure tray background service is running
        let run_tray = install_dir.join("run-tray.vbs");
        if run_tray.exists() {
            let _ = std::process::Command::new("wscript.exe")
                .args(["//B", &run_tray.to_string_lossy()])
                .creation_flags(CREATE_NO_WINDOW)
                .spawn();
        }

        let app_arg = format!("--app={url}");
        let profile_arg = format!("--user-data-dir={local_app_data}\\Spawnt\\AppProfile");
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
                    "--window-name=Spawnt",
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
                .spawn();
        }
    }
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
}

#[cfg(target_os = "windows")]
const SPAWNT_ICO: &[u8] = include_bytes!("../../../scripts/spawnt.ico");
#[cfg(target_os = "windows")]
const TRAY_PS1: &str = include_str!("../../../scripts/tray.ps1");
#[cfg(target_os = "windows")]
const RUN_TRAY_VBS: &str = include_str!("../../../scripts/run-tray.vbs");
#[cfg(target_os = "windows")]
const OPEN_CONSOLE_VBS: &str = include_str!("../../../scripts/open-console.vbs");
#[cfg(target_os = "windows")]
const OPEN_DASH_VBS: &str = include_str!("../../../scripts/open-dashboard.vbs");

fn cmd_install(db_path: &std::path::Path) -> Result<()> {
    println!("=== SPAWNT NATIVE INSTALLER ===");
    if let Ok(mut db) = Database::open(db_path) {
        let _ = ingest_all(&mut db);
    }

    let current_exe = std::env::current_exe().context("failed to locate running executable")?;

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt as _;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;

        let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| {
            spawnt_home()
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .join("AppData")
                .join("Local")
                .to_string_lossy()
                .into_owned()
        });
        let install_dir = PathBuf::from(local_app_data).join("spawnt");
        std::fs::create_dir_all(&install_dir).context("failed to create installation directory")?;

        let dest_exe = install_dir.join("spawnt.exe");
        if current_exe != dest_exe {
            let my_pid = std::process::id();
            let kill_cmd = format!(
                "Get-Process -Name spawnt -ErrorAction SilentlyContinue | Where-Object {{ $_.Id -ne {my_pid} }} | Stop-Process -Force -ErrorAction SilentlyContinue"
            );
            let _ = std::process::Command::new("powershell")
                .args(["-NoProfile", "-Command", &kill_cmd])
                .creation_flags(CREATE_NO_WINDOW)
                .status();
            std::thread::sleep(std::time::Duration::from_millis(300));
            std::fs::copy(&current_exe, &dest_exe)
                .context("failed to copy spawnt.exe to install directory")?;
        }

        std::fs::write(install_dir.join("spawnt.ico"), SPAWNT_ICO)?;
        std::fs::write(install_dir.join("tray.ps1"), TRAY_PS1)?;
        std::fs::write(install_dir.join("run-tray.vbs"), RUN_TRAY_VBS)?;
        std::fs::write(install_dir.join("open-console.vbs"), OPEN_CONSOLE_VBS)?;
        std::fs::write(install_dir.join("open-dashboard.vbs"), OPEN_DASH_VBS)?;

        let dir_str = install_dir.to_string_lossy();
        let ps_setup = format!(
            r#"
$InstallDir = '{dir_str}'
$ExeDest = "$InstallDir\spawnt.exe"
$IcoDest = "$InstallDir\spawnt.ico"
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$InstallDir*") {{
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
}}
$RunKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
Set-ItemProperty -Path $RunKey -Name "Spawnt" -Value "wscript.exe `"$InstallDir\run-tray.vbs`"" -Force
$WshShell = New-Object -ComObject WScript.Shell
$DesktopDir = [Environment]::GetFolderPath("Desktop")
foreach ($lnkPath in @("$DesktopDir\Spawnt Console.lnk", "$DesktopDir\Spawnt.lnk", "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Spawnt Console.lnk")) {{
    $Shortcut = $WshShell.CreateShortcut($lnkPath)
    $Shortcut.TargetPath = "wscript.exe"
    $Shortcut.Arguments = "`"$InstallDir\open-console.vbs`""
    $Shortcut.WorkingDirectory = $InstallDir
    $Shortcut.IconLocation = "$IcoDest,0"
    $Shortcut.Description = "spawnt console"
    $Shortcut.Save()
}}
Start-Process -FilePath "wscript.exe" -ArgumentList "//B `"$InstallDir\run-tray.vbs`"" -WindowStyle Hidden
Start-Process -FilePath "wscript.exe" -ArgumentList "//B `"$InstallDir\open-console.vbs`"" -WindowStyle Hidden
Add-Type -AssemblyName System.Windows.Forms
[System.Windows.Forms.MessageBox]::Show("spawnt has been installed to $InstallDir.`n`n- Desktop & Start Menu shortcuts created for spawnt Console`n- Configured to start automatically on login`n- Mission Control is now running in your system tray.", "spawnt Setup Complete", [System.Windows.Forms.MessageBoxButtons]::OK, [System.Windows.Forms.MessageBoxIcon]::Information) | Out-Null
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

        println!("Installed spawnt to {}", install_dir.display());
    }

    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_owned());
        let bin_dir = PathBuf::from(home).join(".local").join("bin");
        std::fs::create_dir_all(&bin_dir)?;
        let dest = bin_dir.join("spawnt");
        if current_exe != dest {
            std::fs::copy(&current_exe, &dest)?;
        }
        println!("Installed spawnt to {}", dest.display());
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
        spawnt_pty::launch_session(&session_id, agent, &repo_path, model.as_deref()).await?;

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
    println!("  spawnt attach {}", record.session_id);
    println!("To stop the session:");
    println!("  spawnt stop {}", record.session_id);

    Ok(())
}

async fn cmd_stop(session_id: &str) -> Result<()> {
    println!("[WAIT] Stopping session {}...", session_id);
    spawnt_pty::stop_session(session_id).await?;
    println!("[IDLE] Session {} stopped", session_id);
    Ok(())
}

async fn cmd_attach(session_id: &str) -> Result<()> {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::connect_async;
    use tokio_tungstenite::tungstenite::Message;

    let record = spawnt_pty::load_session_record(session_id)?;
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
    let models = tokio::task::spawn_blocking(spawnt_daemon::api::available_models).await?;
    if models.is_empty() {
        println!(
            "No fresh CLI model catalog is available. Open Codex CLI to refresh its catalog; use other agents' own model selectors."
        );
    } else {
        println!(
            "{:<12} {:<28} {:<20} OBSERVED (UTC)",
            "AGENT", "MODEL", "SOURCE"
        );
        for model in models {
            println!(
                "{:<12} {:<28} {:<20} {}",
                model.agent, model.model, model.availability_source, model.observed_at
            );
        }
    }
    Ok(())
}

async fn cmd_switch(session_id: &str, model: &str, db_path: &Path) -> Result<()> {
    let db = spawnt_store::Database::open(db_path)?;
    let record =
        spawnt_daemon::switch_session(&tokio::sync::Mutex::new(db), session_id, model, None)
            .await?;
    println!(
        "[RUN] Switched to {} in session {}",
        model, record.session_id
    );
    println!("Attach with: spawnt attach {}", record.session_id);
    Ok(())
}

async fn cmd_resume(session_id: &str, prompt: Option<String>, db_path: &Path) -> Result<()> {
    anyhow::ensure!(
        prompt.as_deref().is_none_or(|p| p == "continue"),
        "Only the default resume prompt is supported"
    );
    println!("[WAIT] Arming resume queue for session {}...", session_id);
    let db = spawnt_store::Database::open(db_path)?;
    let target = db
        .get_session_focus_target(session_id)?
        .context("Session not found")?;
    let agent = match target.agent.as_str() {
        "claude-code" => spawnt_core::AgentId::ClaudeCode,
        "codex" => spawnt_core::AgentId::Codex,
        _ => anyhow::bail!("Agent does not support resume"),
    };
    let cwd = target.repo_root.context("Session has no repository path")?;
    let entry = spawnt_core::rules::ResumeQueueEntry {
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
    let db = spawnt_store::Database::open(db_path)?;
    if let Some(new_policy) = set {
        let policy = spawnt_core::RoutingPolicy::parse_policy(&new_policy).ok_or_else(|| {
            anyhow::anyhow!(
                "Invalid policy: {new_policy}. Valid options: ask_first, automatic, deny_cross_provider"
            )
        })?;
        db.save_routing_policy(policy)?;
        println!("[RUN] Routing policy updated to: {}", policy.as_str());
    } else {
        let policy = db
            .get_routing_policy()
            .unwrap_or(spawnt_core::RoutingPolicy::AskFirst);
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
    spawnt_pty::run_pty_host_resuming(session_id, agent, repo, model, None, resume_session)
        .await
        .map_err(|e| anyhow::anyhow!("Session host failed: {e}"))
}
