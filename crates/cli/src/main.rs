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
use fleethud_platform::{
    attach_parent_console, default_db_path, discover_agent_roots, fleethud_home,
};
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
    },
    /// Run diagnostic checks on local agent log directories and system environment.
    Doctor {
        /// Display all network endpoints required for licensing, cloud sync, and pricing updates.
        #[arg(long)]
        network: bool,
    },
    /// Start the FleetHUD background daemon and local web console.
    Daemon {
        /// Port to bind the local HTTP server to.
        #[arg(long, default_value_t = 41738)]
        port: u16,
        /// Open the web console in the default browser.
        #[arg(long)]
        open: bool,
    },
    /// Convenience shortcut: start the daemon and open the Next.js Mission Control Console in your browser.
    Console {
        /// Port to bind the local HTTP server to.
        #[arg(long, default_value_t = 41738)]
        port: u16,
    },
    /// Backwards compatible alias for `fleethud console`.
    #[command(hide = true)]
    Dashboard {
        /// Port to bind the local HTTP server to.
        #[arg(long, default_value_t = 41738)]
        port: u16,
    },
    /// Install FleetHUD to the local system, create Desktop/Start Menu shortcuts, and configure autostart.
    Install,
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
        Commands::Report { plan_usd } => cmd_report(&db_path, plan_usd),
        Commands::Doctor { network } => cmd_doctor(&db_path, network),
        Commands::Daemon { port, open } => cmd_daemon(&db_path, port, open).await,
        Commands::Console { port } => cmd_daemon(&db_path, port, true).await,
        Commands::Dashboard { port } => cmd_daemon(&db_path, port, true).await,
        Commands::Install => cmd_install(&db_path),
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

async fn cmd_daemon(db_path: &std::path::Path, port: u16, open: bool) -> Result<()> {
    let (lock, info) = match fleethud_daemon::DaemonLock::acquire(port) {
        Ok(res) => res,
        Err(fleethud_daemon::Error::AlreadyRunning) => {
            if let Some(existing) = fleethud_daemon::read_daemon_info() {
                let local_url = format!(
                    "http://127.0.0.1:{}/?token={}",
                    existing.port, existing.token
                );
                let console_url =
                    std::env::var("FLEETHUD_CONSOLE_URL").unwrap_or_else(|_| local_url.clone());
                println!("FleetHUD is already active (PID {}).", existing.pid);
                println!("Opened Native Mission Control Console: {console_url}");
                if open {
                    open_browser(&console_url);
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
        fleethud_daemon::DaemonOrchestrator::new(db_path, std::time::Duration::from_secs(2))
            .context("failed to initialize daemon orchestrator")?;
    let _reconciler_handle = orchestrator.spawn_reconciler();

    let state = fleethud_daemon::api::AppState {
        db: orchestrator.db(),
        bearer_token: info.token.clone(),
    };

    let router = fleethud_daemon::api::build_router(state);
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context(format!("failed to bind loopback TCP listener on {addr}"))?;

    let local_url = format!("http://127.0.0.1:{port}/?token={}", info.token);
    let console_url = std::env::var("FLEETHUD_CONSOLE_URL").unwrap_or_else(|_| local_url.clone());
    println!("=== FLEETHUD DAEMON STARTED ===");
    println!("Status: Running");
    println!("Loopback Port: {port}");
    println!("PID: {}", info.pid);
    println!("Native Console: {console_url}");
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
        let install_dir = std::path::PathBuf::from(&local_app_data).join("FleetHUD");

        // Ensure tray background service is running
        let run_tray = install_dir.join("run-tray.vbs");
        if run_tray.exists() {
            let _ = std::process::Command::new("wscript.exe")
                .args(["//B", &run_tray.to_string_lossy()])
                .creation_flags(CREATE_NO_WINDOW)
                .spawn();
        }

        let app_arg = format!("--app={url}");
        let profile_arg = format!("--user-data-dir={local_app_data}\\FleetHUD\\AppProfile");
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
                    "--window-name=FleetHUD",
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

const FLEETHUD_ICO: &[u8] = include_bytes!("../../../scripts/fleethud.ico");
const TRAY_PS1: &str = include_str!("../../../scripts/tray.ps1");
const RUN_TRAY_VBS: &str = include_str!("../../../scripts/run-tray.vbs");
const OPEN_CONSOLE_VBS: &str = include_str!("../../../scripts/open-console.vbs");
const OPEN_DASH_VBS: &str = include_str!("../../../scripts/open-dashboard.vbs");

fn cmd_install(db_path: &std::path::Path) -> Result<()> {
    println!("=== FLEETHUD NATIVE INSTALLER ===");
    if let Ok(mut db) = Database::open(db_path) {
        let _ = ingest_all(&mut db);
    }

    let current_exe = std::env::current_exe().context("failed to locate running executable")?;

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt as _;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;

        let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| {
            fleethud_home()
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .join("AppData")
                .join("Local")
                .to_string_lossy()
                .into_owned()
        });
        let install_dir = PathBuf::from(local_app_data).join("fleethud");
        std::fs::create_dir_all(&install_dir).context("failed to create installation directory")?;

        let dest_exe = install_dir.join("fleethud.exe");
        if current_exe != dest_exe {
            let my_pid = std::process::id();
            let kill_cmd = format!(
                "Get-Process -Name fleethud -ErrorAction SilentlyContinue | Where-Object {{ $_.Id -ne {my_pid} }} | Stop-Process -Force -ErrorAction SilentlyContinue"
            );
            let _ = std::process::Command::new("powershell")
                .args(["-NoProfile", "-Command", &kill_cmd])
                .creation_flags(CREATE_NO_WINDOW)
                .status();
            std::thread::sleep(std::time::Duration::from_millis(300));
            std::fs::copy(&current_exe, &dest_exe)
                .context("failed to copy fleethud.exe to install directory")?;
        }

        std::fs::write(install_dir.join("fleethud.ico"), FLEETHUD_ICO)?;
        std::fs::write(install_dir.join("tray.ps1"), TRAY_PS1)?;
        std::fs::write(install_dir.join("run-tray.vbs"), RUN_TRAY_VBS)?;
        std::fs::write(install_dir.join("open-console.vbs"), OPEN_CONSOLE_VBS)?;
        std::fs::write(install_dir.join("open-dashboard.vbs"), OPEN_DASH_VBS)?;

        let dir_str = install_dir.to_string_lossy();
        let ps_setup = format!(
            r#"
$InstallDir = '{dir_str}'
$ExeDest = "$InstallDir\fleethud.exe"
$IcoDest = "$InstallDir\fleethud.ico"
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$InstallDir*") {{
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
}}
$RunKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
Set-ItemProperty -Path $RunKey -Name "FleetHUD" -Value "wscript.exe `"$InstallDir\run-tray.vbs`"" -Force
$WshShell = New-Object -ComObject WScript.Shell
$DesktopDir = [Environment]::GetFolderPath("Desktop")
foreach ($lnkPath in @("$DesktopDir\FleetHUD Console.lnk", "$DesktopDir\FleetHUD.lnk", "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\FleetHUD Console.lnk")) {{
    $Shortcut = $WshShell.CreateShortcut($lnkPath)
    $Shortcut.TargetPath = "wscript.exe"
    $Shortcut.Arguments = "`"$InstallDir\open-console.vbs`""
    $Shortcut.WorkingDirectory = $InstallDir
    $Shortcut.IconLocation = "$IcoDest,0"
    $Shortcut.Description = "FleetHUD Mission Control Console"
    $Shortcut.Save()
}}
Start-Process -FilePath "wscript.exe" -ArgumentList "//B `"$InstallDir\run-tray.vbs`"" -WindowStyle Hidden
Start-Process -FilePath "wscript.exe" -ArgumentList "//B `"$InstallDir\open-console.vbs`"" -WindowStyle Hidden
Add-Type -AssemblyName System.Windows.Forms
[System.Windows.Forms.MessageBox]::Show("FleetHUD has been installed to $InstallDir.`n`n- Desktop & Start Menu shortcuts created for FleetHUD Console`n- Configured to start automatically on login`n- Mission Control is now running in your system tray.", "FleetHUD Setup Complete", [System.Windows.Forms.MessageBoxButtons]::OK, [System.Windows.Forms.MessageBoxIcon]::Information) | Out-Null
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

        println!("Installed FleetHUD to {}", install_dir.display());
    }

    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_owned());
        let bin_dir = PathBuf::from(home).join(".local").join("bin");
        std::fs::create_dir_all(&bin_dir)?;
        let dest = bin_dir.join("fleethud");
        if current_exe != dest {
            std::fs::copy(&current_exe, &dest)?;
        }
        println!("Installed FleetHUD to {}", dest.display());
    }

    Ok(())
}
