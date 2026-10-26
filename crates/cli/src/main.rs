//! FleetHUD CLI entry point (`SPEC §3.6`, `§4.6`).
//!
//! Provides read-only CLI commands:
//! - `fleethud status`: live & recent agent sessions, state, model, surface.
//! - `fleethud report`: usage & cost breakdown by project/agent/model + quota pools + Plan Value ROI.
//! - `fleethud doctor`: environment diagnostics, detected agent roots, and network requirements.

#![allow(clippy::print_stdout)]

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use fleethud_core::pricing::calculate_plan_roi;
use fleethud_ingest::ingest_all;
use fleethud_platform::{default_db_path, discover_agent_roots, fleethud_home};
use fleethud_store::Database;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "fleethud",
    version,
    about = "Live mission control for AI coding agents (Claude Code, Codex, Antigravity, Gemini CLI)"
)]
struct Cli {
    /// Custom path to the SQLite database.
    #[arg(long, global = true)]
    db: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
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
    },
    /// Run diagnostic checks on local agent log directories and system environment.
    Doctor {
        /// Display all network endpoints required for licensing, cloud sync, and pricing updates.
        #[arg(long)]
        network: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let db_path = cli.db.unwrap_or_else(default_db_path);

    match cli.command {
        Commands::Status { limit } => cmd_status(&db_path, limit),
        Commands::Report { plan_usd } => cmd_report(&db_path, plan_usd),
        Commands::Doctor { network } => cmd_doctor(&db_path, network),
    }
}

fn cmd_status(db_path: &std::path::Path, limit: usize) -> Result<()> {
    let mut db = Database::open(db_path).context("failed to open database")?;
    let _ = ingest_all(&mut db);

    let sessions = db
        .list_active_sessions(limit)
        .context("failed to query active sessions")?;

    println!("=== FLEETHUD ACTIVE SESSIONS ===");
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
        let model = s.model.as_deref().unwrap_or("unknown");
        let last_active = s.last_event_at.format("%Y-%m-%d %H:%M:%S UTC");
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

fn cmd_report(db_path: &std::path::Path, plan_usd: Option<f64>) -> Result<()> {
    let mut db = Database::open(db_path).context("failed to open database")?;
    let _ = ingest_all(&mut db);

    let report_rows = db
        .generate_usage_report()
        .context("failed to generate usage report")?;

    println!("=== FLEETHUD USAGE & COST REPORT ===");
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
        "TOTAL: API-Equivalent Spend: ${:.2}  |  Prompt Cache Savings: ${:.2} [EXACT]",
        total_cost, total_savings
    );

    if let Some(price) = plan_usd {
        let roi = calculate_plan_roi(total_cost, price);
        println!(
            "PLAN VALUE ROI: ${:.2}/mo Plan -> {:.1}x Value (Net Value: +${:.2}) [EXACT]",
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
                fleethud_core::QuotaWindow::FiveHour => "5-hour",
                fleethud_core::QuotaWindow::Weekly => "weekly",
                fleethud_core::QuotaWindow::Monthly => "monthly",
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
    println!("=== FLEETHUD DOCTOR DIAGNOSTICS ===");
    println!("Home Directory: {}", fleethud_home().display());
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
        println!("  https://api.fleethud.pro  Opt-in Pricing Catalog & License Check");
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
