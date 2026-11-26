//! OS-specific integration: paths, shared-mode file reading, process scanning,
//! window focus, and autostart (`SPEC §3.5`, `§3.12`, `§4.5`).

pub mod executables;
pub mod keepawake;
pub mod workspaces;
pub mod wsl;

use gnomcode_core::AgentId;
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Platform-layer error type.
#[derive(Debug, Error)]
pub enum Error {
    /// I/O failure when inspecting paths or reading log files.
    #[error("platform I/O error: {0}")]
    Io(#[from] io::Error),
}

/// Discovered root directory for an AI coding agent on the current machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRoot {
    /// Agent identifier.
    pub agent: AgentId,
    /// Root directory where the agent stores session logs.
    pub path: PathBuf,
    /// Whether this root directory currently exists on disk.
    pub exists: bool,
}

/// Resolves the data directory, preserving access to legacy local history.
#[must_use]
pub fn gnomcode_home() -> PathBuf {
    ["GNOMCODE_HOME", "SPAWNT_HOME"]
        .iter()
        .filter_map(|key| env::var(key).ok())
        .find(|value| !value.trim().is_empty())
        .map(|value| PathBuf::from(value.trim()))
        .unwrap_or_else(|| {
            let home = user_home_dir();
            let legacy = home.join(".spawnt");
            if legacy.is_dir() {
                legacy
            } else {
                home.join(".gnomcode")
            }
        })
}

/// Resolves the default SQLite database path (`~/.gnomcode/state.db`).
#[must_use]
pub fn default_db_path() -> PathBuf {
    gnomcode_home().join("state.db")
}

/// Lists modified and untracked repository paths for a handoff.
///
/// # Errors
/// Returns an error when Git cannot read the repository or a path is not UTF-8.
pub fn handoff_files(repo: &Path) -> Result<Vec<String>, Error> {
    if !repo.is_dir() {
        return Err(io::Error::other("workspace directory is unavailable").into());
    }
    let mut git_workspace = false;
    for ancestor in repo.ancestors() {
        if ancestor.join(".git").try_exists()? {
            git_workspace = true;
            break;
        }
    }
    if !git_workspace {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for args in [
        &["diff", "--name-only", "-z"][..],
        &["diff", "--cached", "--name-only", "-z"][..],
        &["ls-files", "--others", "--exclude-standard", "-z"][..],
    ] {
        let mut command = std::process::Command::new("git");
        command.args(args).current_dir(repo);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt as _;
            command.creation_flags(0x0800_0000);
        }
        let output = command.output()?;
        if !output.status.success() {
            return Err(io::Error::other("git could not inspect repository").into());
        }
        for path in output.stdout.split(|b| *b == 0).filter(|p| !p.is_empty()) {
            files.push(
                std::str::from_utf8(path)
                    .map_err(|_| io::Error::other("repository path is not UTF-8"))?
                    .to_owned(),
            );
        }
    }
    files.sort();
    files.dedup();
    Ok(files)
}

/// Returns whether the operating system reports a process as alive. An exited process that has not
/// been reaped yet (a zombie) is not alive.
#[must_use]
pub fn process_is_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    let mut system = sysinfo::System::new();
    system.refresh_processes(
        sysinfo::ProcessesToUpdate::Some(&[sysinfo::Pid::from_u32(pid)]),
        true,
    );
    // A zombie has exited and only awaits its parent's wait(); it can no longer run or be stopped.
    system
        .process(sysinfo::Pid::from_u32(pid))
        .is_some_and(|process| process.status() != sysinfo::ProcessStatus::Zombie)
}

/// Stops a process that was explicitly launched and owned by Gnomcode.
///
/// # Errors
/// Returns an error if the process cannot be stopped.
pub fn stop_owned_process(pid: u32) -> Result<(), Error> {
    if pid == 0 {
        return Err(io::Error::other("invalid process ID").into());
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        let pid_arg = pid.to_string();
        let status = std::process::Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid_arg])
            .creation_flags(0x0800_0000)
            .status()?;
        if status.success() || !process_is_alive(pid) {
            return Ok(());
        }
        // Fallback without `/T` in case a child process exited mid-tree-kill.
        let direct = std::process::Command::new("taskkill")
            .args(["/F", "/PID", &pid_arg])
            .creation_flags(0x0800_0000)
            .status()?;
        if direct.success() || !process_is_alive(pid) {
            Ok(())
        } else {
            Err(io::Error::other("process termination failed").into())
        }
    }
    #[cfg(unix)]
    {
        unix_stop::stop_process_group(pid)
    }
}

#[cfg(any(unix, test))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnixStopSignal {
    Interrupt,
    Terminate,
    Kill,
}

#[cfg(any(unix, test))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnixStopTarget {
    ProcessGroup(i32),
    SingleProcess(i32),
}

#[cfg(any(unix, test))]
const UNIX_STOP_ESCALATION: [UnixStopSignal; 3] = [
    UnixStopSignal::Interrupt,
    UnixStopSignal::Terminate,
    UnixStopSignal::Kill,
];

#[cfg(any(unix, test))]
fn select_unix_stop_target(pid: i32, target_pgid: i32, own_pgid: i32) -> UnixStopTarget {
    if target_pgid > 0 && target_pgid != own_pgid {
        UnixStopTarget::ProcessGroup(target_pgid)
    } else {
        UnixStopTarget::SingleProcess(pid)
    }
}

#[cfg(unix)]
mod unix_stop {
    #![allow(unsafe_code)]

    use super::{
        UNIX_STOP_ESCALATION, UnixStopSignal, UnixStopTarget, process_is_alive,
        select_unix_stop_target,
    };
    use crate::Error;
    use std::io;
    use std::time::{Duration, Instant};

    const STAGE_WAIT: Duration = Duration::from_millis(250);
    const POLL_INTERVAL: Duration = Duration::from_millis(25);

    fn libc_signal(signal: UnixStopSignal) -> libc::c_int {
        match signal {
            UnixStopSignal::Interrupt => libc::SIGINT,
            UnixStopSignal::Terminate => libc::SIGTERM,
            UnixStopSignal::Kill => libc::SIGKILL,
        }
    }

    fn send_signal(target: UnixStopTarget, signal: UnixStopSignal) -> Result<(), io::Error> {
        let sig = libc_signal(signal);
        let rc = match target {
            // SAFETY: `pgid` is a positive process-group ID distinct from our own process group.
            UnixStopTarget::ProcessGroup(pgid) => unsafe { libc::killpg(pgid, sig) },
            // SAFETY: `raw_pid` is a positive process ID validated from `u32`.
            UnixStopTarget::SingleProcess(raw_pid) => unsafe { libc::kill(raw_pid, sig) },
        };
        if rc == 0 {
            return Ok(());
        }
        let err = io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::ESRCH) {
            return Ok(());
        }
        Err(err)
    }

    fn wait_for_exit(pid: u32, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        while process_is_alive(pid) {
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(POLL_INTERVAL);
        }
        true
    }

    pub(super) fn stop_process_group(pid: u32) -> Result<(), Error> {
        let raw_pid = i32::try_from(pid).map_err(|_| io::Error::other("invalid process ID"))?;
        if raw_pid <= 0 {
            return Err(io::Error::other("invalid process ID").into());
        }
        if !process_is_alive(pid) {
            return Err(io::Error::other("process not found").into());
        }
        // SAFETY: `getpgid` and `getpgrp` take a validated PID / no arguments and read kernel process-group IDs.
        let (target_pgid, own_pgid) = unsafe { (libc::getpgid(raw_pid), libc::getpgrp()) };
        let target = select_unix_stop_target(raw_pid, target_pgid, own_pgid);

        for signal in UNIX_STOP_ESCALATION {
            send_signal(target, signal)?;
            if wait_for_exit(pid, STAGE_WAIT) {
                return Ok(());
            }
        }

        if !process_is_alive(pid) {
            Ok(())
        } else {
            Err(io::Error::other("process termination failed").into())
        }
    }
}

/// Resolves the user's home directory across Windows, macOS, and Linux.
#[must_use]
pub fn user_home_dir() -> PathBuf {
    if let Ok(home) = env::var("HOME")
        && !home.trim().is_empty()
    {
        return PathBuf::from(home);
    }
    if let Ok(profile) = env::var("USERPROFILE")
        && !profile.trim().is_empty()
    {
        return PathBuf::from(profile);
    }
    PathBuf::from(".")
}

/// Provider-owned document output folders, never credential or configuration roots.
#[must_use]
pub fn agent_document_roots() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for root in discover_agent_roots() {
        match root.agent {
            AgentId::ClaudeCode | AgentId::Codex => {
                if let Some(home) = root.path.parent() {
                    paths.push(home.join("plans"));
                    paths.push(home.join("artifacts"));
                }
            }
            AgentId::Antigravity => paths.push(root.path),
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

/// Resolves the current machine's hostname, or returns "local" if unknown.
#[must_use]
pub fn hostname() -> String {
    env::var("COMPUTERNAME")
        .or_else(|_| env::var("HOSTNAME"))
        .unwrap_or_else(|_| "local".to_owned())
}

/// Returns the candidate log root directories for supported agents, respecting
/// environment variable overrides (`CLAUDE_CONFIG_DIR`, `CODEX_HOME`, and `GEMINI_HOME`, which locates Antigravity data).
#[must_use]
pub fn discover_agent_roots() -> Vec<AgentRoot> {
    let home = user_home_dir();
    let claude_root = env::var("CLAUDE_CONFIG_DIR")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map_or_else(|| home.join(".claude"), PathBuf::from)
        .join("projects");

    let codex_home = env::var("CODEX_HOME")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map_or_else(|| home.join(".codex"), PathBuf::from);
    let codex_root = codex_home.join("sessions");
    let codex_archive_root = codex_home.join("archived_sessions");

    let gemini_base = env::var("GEMINI_HOME")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map_or_else(|| home.join(".gemini"), PathBuf::from);

    let antigravity_root = gemini_base.join("antigravity").join("brain");
    // Older Antigravity IDE builds kept history in a separate home.
    let antigravity_ide_root = gemini_base.join("antigravity-ide").join("brain");
    let antigravity_cli_root = gemini_base.join("antigravity-cli").join("brain");

    let mut roots = vec![
        AgentRoot {
            agent: AgentId::ClaudeCode,
            exists: claude_root.exists(),
            path: claude_root,
        },
        AgentRoot {
            agent: AgentId::Codex,
            exists: codex_root.exists(),
            path: codex_root,
        },
        AgentRoot {
            agent: AgentId::Antigravity,
            exists: antigravity_root.exists(),
            path: antigravity_root,
        },
        AgentRoot {
            agent: AgentId::Codex,
            exists: codex_archive_root.exists(),
            path: codex_archive_root,
        },
        AgentRoot {
            agent: AgentId::Antigravity,
            exists: antigravity_ide_root.exists(),
            path: antigravity_ide_root,
        },
        AgentRoot {
            agent: AgentId::Antigravity,
            exists: antigravity_cli_root.exists(),
            path: antigravity_cli_root,
        },
    ];
    // Running WSL distros only (ADR 0060); stopped ones are never touched.
    roots.extend(wsl::discover_wsl_roots());
    wsl::dedupe_roots(roots)
}

/// Resolves user application configuration / roaming directory across operating systems.
#[must_use]
pub fn app_config_dir() -> PathBuf {
    if let Ok(appdata) = env::var("APPDATA")
        && !appdata.trim().is_empty()
    {
        return PathBuf::from(appdata);
    }
    let home = user_home_dir();
    #[cfg(target_os = "macos")]
    {
        home.join("Library").join("Application Support")
    }
    #[cfg(not(target_os = "macos"))]
    {
        home.join(".config")
    }
}

/// Canonicalizes a path by resolving the git repository root and remote origin URL.
/// Handles standard git clones as well as git worktrees (`.git` pointing to `gitdir:`).
#[must_use]
pub fn resolve_git_repo_and_remote(path: &Path) -> (Option<PathBuf>, Option<String>) {
    let mut curr = if path.is_file() {
        path.parent()
    } else {
        Some(path)
    };

    while let Some(dir) = curr {
        let dot_git = dir.join(".git");
        if dot_git.is_dir() {
            let remote = read_git_remote_url(&dot_git.join("config"));
            return (
                Some(dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf())),
                remote,
            );
        } else if dot_git.is_file()
            && let Ok(content) = fs::read_to_string(&dot_git)
            && let Some(gitdir_line) = content
                .lines()
                .find(|l| l.trim_start().starts_with("gitdir:"))
        {
            let gitdir_str = gitdir_line.trim_start()["gitdir:".len()..].trim();
            let gitdir = if Path::new(gitdir_str).is_absolute() {
                PathBuf::from(gitdir_str)
            } else {
                dir.join(gitdir_str)
            };
            let commondir_file = gitdir.join("commondir");
            let main_git_dir = if let Ok(common) = fs::read_to_string(&commondir_file) {
                let common_trimmed = common.trim();
                if Path::new(common_trimmed).is_absolute() {
                    PathBuf::from(common_trimmed)
                } else {
                    gitdir.join(common_trimmed)
                }
            } else {
                gitdir.clone()
            };
            let canonical_git_dir = main_git_dir.canonicalize().unwrap_or(main_git_dir);
            let remote = read_git_remote_url(&canonical_git_dir.join("config"));
            let main_repo_root = canonical_git_dir
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| dir.to_path_buf());
            return (Some(main_repo_root), remote);
        }
        curr = dir.parent();
    }
    (None, None)
}

fn read_git_remote_url(config_path: &Path) -> Option<String> {
    let content = fs::read_to_string(config_path).ok()?;
    let mut in_remote_origin = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_remote_origin = trimmed.eq_ignore_ascii_case("[remote \"origin\"]");
        } else if in_remote_origin
            && trimmed.starts_with("url")
            && let Some((_, val)) = trimmed.split_once('=')
        {
            let clean = val.trim();
            if !clean.is_empty() {
                return Some(clean.to_owned());
            }
        }
    }
    None
}

/// Locally detected subscription plan prices and tier labels.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DetectedSubscriptions {
    /// Detected Anthropic monthly plan USD and tier label (e.g. `(100.0, "Claude Max 5x")`).
    pub anthropic: Option<(f64, String)>,
    /// Detected Google monthly plan USD and tier label (e.g. `(20.0, "Google AI Pro")`).
    pub google: Option<(f64, String)>,
}

/// Detects the existing Antigravity installation marker without reading credentials.
/// Claude plans must be configured manually; its credentials file is never read.
#[must_use]
pub fn detect_local_subscriptions() -> DetectedSubscriptions {
    let home = user_home_dir();
    let anthropic = None;

    let gemini_base = env::var("GEMINI_HOME")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map_or_else(|| home.join(".gemini"), PathBuf::from);
    let google = if gemini_base
        .join("antigravity")
        .join("installation_id")
        .exists()
    {
        Some((20.0, "Google AI Pro".to_owned()))
    } else {
        None
    };

    DetectedSubscriptions { anthropic, google }
}

/// Reads the fallback active model from `~/.gemini/antigravity/antigravity_state.pbtxt`
/// (`last_selected_agent_model: "..."`) if present (`SPEC §3.5`).
#[must_use]
pub fn read_antigravity_fallback_model() -> Option<String> {
    let gemini_base = env::var("GEMINI_HOME")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map_or_else(|| user_home_dir().join(".gemini"), PathBuf::from);
    let pbtxt = gemini_base
        .join("antigravity")
        .join("antigravity_state.pbtxt");
    let content = fs::read_to_string(pbtxt).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("last_selected_agent_model:") {
            let unquoted = rest.trim().trim_matches('"').trim();
            if !unquoted.is_empty() && !unquoted.contains("MODEL_PLACEHOLDER") {
                return Some(unquoted.to_owned());
            }
        }
    }
    None
}

/// Opens a file with shared read/write/delete permissions on Windows so Gnomcode
/// never blocks an active agent from appending or rotating its log (`SPEC §3.12`).
///
/// # Errors
/// Returns [`Error::Io`] if the file cannot be opened.
pub fn open_shared_read(path: &Path) -> Result<File, Error> {
    let mut opts = OpenOptions::new();
    opts.read(true);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::OpenOptionsExt as _;
        // FILE_SHARE_READ (1) | FILE_SHARE_WRITE (2) | FILE_SHARE_DELETE (4) = 7
        opts.share_mode(0x0000_0007);
    }
    Ok(opts.open(path)?)
}

/// Result of reading a byte range from a log file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileReadChunk {
    /// Bytes read starting at `start_offset`.
    pub bytes: Vec<u8>,
    /// Current total file size on disk when opened.
    pub file_size: u64,
    /// Effective offset from which reading started (`0` if file was truncated).
    pub effective_start_offset: u64,
}

/// Opens `path` using [`open_shared_read`], detects truncation (`file_size < offset`),
/// reads from `effective_start_offset` to EOF, and closes the handle immediately.
///
/// # Errors
/// Returns [`Error::Io`] if opening, seeking, or reading fails.
pub fn read_from_offset(path: &Path, offset: u64) -> Result<FileReadChunk, Error> {
    let mut file = open_shared_read(path)?;
    let meta = file.metadata()?;
    let file_size = meta.len();
    let effective_start_offset = if file_size < offset { 0 } else { offset };

    file.seek(SeekFrom::Start(effective_start_offset))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;

    Ok(FileReadChunk {
        bytes,
        file_size,
        effective_start_offset,
    })
}

/// Attaches to the parent console on Windows if invoked from a command prompt,
/// allowing CLI output while still running as a windowless GUI subsystem binary.
#[cfg(target_os = "windows")]
#[allow(unsafe_code)]
pub fn attach_parent_console() {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn AttachConsole(dwProcessId: u32) -> i32;
    }
    const ATTACH_PARENT_PROCESS: u32 = 0xFFFF_FFFF;
    // SAFETY: AttachConsole attaches the current process to its parent console session.
    unsafe {
        let _ = AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

/// No-op on non-Windows platforms.
#[cfg(not(target_os = "windows"))]
pub fn attach_parent_console() {}

#[cfg(target_os = "windows")]
fn windows_focus_prelude() -> &'static str {
    r#"$ErrorActionPreference='Stop'; Add-Type 'using System; using System.Runtime.InteropServices; public class GnomcodeFocus { [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr h); [DllImport("user32.dll")] public static extern bool ShowWindowAsync(IntPtr h,int command); [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h); [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow(); }'; $shell=New-Object -ComObject WScript.Shell; $focus={ param([int]$targetId) $window=Get-Process -Id $targetId -ErrorAction SilentlyContinue; if(!$window -or $window.MainWindowHandle -eq 0){return $false}; if([GnomcodeFocus]::IsIconic($window.MainWindowHandle)){[GnomcodeFocus]::ShowWindowAsync($window.MainWindowHandle,9)|Out-Null}; $null=$shell.AppActivate($targetId); [GnomcodeFocus]::SetForegroundWindow($window.MainWindowHandle)|Out-Null; return ([GnomcodeFocus]::GetForegroundWindow() -eq $window.MainWindowHandle) }; "#
}

/// Dispatches a window focus request for a process ID across OS platforms (`SPEC §3.13`).
///
/// Returns `true` if a focus attempt was dispatched, or `false` if unsupported.
#[must_use]
pub fn bring_to_front(pid: u32) -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt as _;
        let prelude = windows_focus_prelude();
        let script = format!(
            "{prelude} $target={pid}; for($i=0;$i -lt 8 -and $target -gt 0;$i++) {{ if((& $focus ([int]$target))) {{ exit 0 }}; $process=Get-CimInstance Win32_Process -Filter \"ProcessId=$target\" -ErrorAction SilentlyContinue; if(!$process) {{ break }}; $target=$process.ParentProcessId }}; exit 1"
        );
        std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .creation_flags(0x0800_0000)
            .status()
            .is_ok_and(|status| status.success())
    }
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "tell application \"System Events\" to set frontmost of (every process whose unix id is {pid}) to true"
        );
        std::process::Command::new("osascript")
            .args(["-e", &script])
            .status()
            .is_ok_and(|status| status.success())
    }
    #[cfg(target_os = "linux")]
    {
        let Ok(windows) = std::process::Command::new("wmctrl").args(["-lp"]).output() else {
            return false;
        };
        if !windows.status.success() {
            return false;
        }
        let pid_text = pid.to_string();
        let window = String::from_utf8_lossy(&windows.stdout)
            .lines()
            .find_map(|line| {
                let mut fields = line.split_whitespace();
                let id = fields.next()?;
                fields.next()?;
                (fields.next()? == pid_text).then(|| id.to_owned())
            });
        window.is_some_and(|id| {
            std::process::Command::new("wmctrl")
                .args(["-ia", &id])
                .status()
                .is_ok_and(|status| status.success())
        })
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = pid;
        false
    }
}

/// Activates a known desktop agent application without launching a new session.
/// Returns false for unsupported surfaces or unavailable windows.
#[must_use]
pub fn focus_agent_application(agent: &str, surface: &str) -> bool {
    let Some(app) = desktop_agent_name(agent, surface) else {
        return false;
    };
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt as _;
        let prelude = windows_focus_prelude();
        let script = format!(
            "{prelude} foreach($process in (Get-Process -Name '{app}' -ErrorAction SilentlyContinue | Where-Object {{ $_.MainWindowHandle -ne 0 }})) {{ if((& $focus $process.Id)) {{ exit 0 }} }}; exit 1"
        );
        std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .creation_flags(0x0800_0000)
            .status()
            .is_ok_and(|status| status.success())
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("osascript")
            .args(["-e", &format!("if application \"{app}\" is running then\ntell application \"{app}\" to activate\nelse\nerror \"Agent app is not running\"\nend if")])
            .status()
            .is_ok_and(|status| status.success())
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("wmctrl")
            .args(["-xa", app])
            .status()
            .is_ok_and(|status| status.success())
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = app;
        false
    }
}

fn desktop_agent_name(agent: &str, surface: &str) -> Option<&'static str> {
    match (agent, surface) {
        ("codex", "desktop_app") => Some(if cfg!(target_os = "windows") {
            "ChatGPT"
        } else {
            "Codex"
        }),
        ("claude-code" | "claude", "desktop_app") => Some("Claude"),
        // The IDE (1.x) and the 2.0 app are separate programs with separate process names.
        ("antigravity", "ide") => Some("Antigravity IDE"),
        ("antigravity", "desktop_app") => Some("Antigravity"),
        _ => None,
    }
}

#[cfg(test)]
mod focus_target_tests {
    #[test]
    fn desktop_targets_do_not_match_cli_or_untrusted_names() {
        assert_eq!(
            super::desktop_agent_name("codex", "desktop_app"),
            Some(if cfg!(target_os = "windows") {
                "ChatGPT"
            } else {
                "Codex"
            })
        );
        assert_eq!(super::desktop_agent_name("codex", "cli"), None);
        assert_eq!(
            super::desktop_agent_name("antigravity", "ide"),
            Some("Antigravity IDE")
        );
        assert_eq!(
            super::desktop_agent_name("antigravity", "desktop_app"),
            Some("Antigravity")
        );
        assert_eq!(super::desktop_agent_name("antigravity", "cli"), None);
        assert_eq!(
            super::desktop_agent_name("unknown;command", "desktop_app"),
            None
        );
    }
}

/// Maps an agent identifier to the `gnomcode launch` argument, or `None` when it cannot be launched
/// in a terminal. Claude Code, Codex and Antigravity (`agy`) are all relay-launchable targets.
#[must_use]
pub fn relay_launch_agent(agent: &str) -> Option<&'static str> {
    match agent {
        "claude" | "claude-code" => Some("claude"),
        "codex" => Some("codex"),
        "antigravity" | "agy" => Some("agy"),
        _ => None,
    }
}

/// Starts a CLI agent through the independent PTY owner without shell interpolation (`SPEC §3.14.1`).
/// Returns false for identifiers [`relay_launch_agent`] does not recognise.
#[must_use]
pub fn spawn_agent_in_terminal(agent: &str, cwd: &str) -> bool {
    let Some(clean_agent) = relay_launch_agent(agent) else {
        return false;
    };
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let mut command = std::process::Command::new(exe);
    command
        .args(["launch", clean_agent, "--repo", cwd])
        .current_dir(cwd);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt as _;
        command.creation_flags(0x0800_0000);
    }
    command.status().is_ok_and(|status| status.success())
}

/// Dispatches a native desktop notification across Windows, macOS, and Linux (`SPEC §3.10`).
#[must_use]
pub fn send_desktop_notification(title: &str, body: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        // Only deliver through the installed shell if the desktop application is ALREADY RUNNING.
        // Never spawn gnomcode-desktop.exe when it is closed, as that resurrects the desktop shell
        // after the user has quit.
        let shell_running = {
            let mut system = sysinfo::System::new();
            system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
            system.processes().values().any(|p| {
                let name = p.name().to_string_lossy();
                name.eq_ignore_ascii_case("gnomcode-desktop.exe")
                    || name.eq_ignore_ascii_case("gnomcode-desktop")
            })
        };

        if shell_running
            && let Some(shell) = std::env::current_exe().ok().and_then(|exe| {
                exe.ancestors()
                    .skip(1)
                    .take(4)
                    .flat_map(|base| {
                        [
                            base.join("desktop").join("gnomcode-desktop.exe"),
                            base.join("gnomcode-desktop.exe"),
                        ]
                    })
                    .find(|path| path.is_file())
            })
        {
            use std::os::windows::process::CommandExt as _;
            return std::process::Command::new(shell)
                .creation_flags(0x0800_0000)
                .args(["--gnomcode-notification", body])
                .spawn()
                .is_ok();
        }
        let _ = (title, body);
        false
    }
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "display notification \"{}\" with title \"{}\"",
            body.replace('\"', "\\\""),
            title.replace('\"', "\\\"")
        );
        let _ = std::process::Command::new("osascript")
            .args(["-e", &script])
            .spawn();
        true
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("notify-send")
            .args([title, body])
            .spawn();
        true
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = (title, body);
        false
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn relay_launches_all_three_supported_agents_and_rejects_others() {
        assert_eq!(super::relay_launch_agent("claude"), Some("claude"));
        assert_eq!(super::relay_launch_agent("claude-code"), Some("claude"));
        assert_eq!(super::relay_launch_agent("codex"), Some("codex"));
        assert_eq!(super::relay_launch_agent("antigravity"), Some("agy"));
        assert_eq!(super::relay_launch_agent("agy"), Some("agy"));
        assert_eq!(super::relay_launch_agent("gemini"), None);
    }

    #[test]
    fn antigravity_cli_and_ide_roots_respect_gemini_home() {
        const CHILD: &str = "GNOMCODE_TEST_AGY_ROOTS";
        if let Some(root) = std::env::var_os(CHILD) {
            let root = std::path::PathBuf::from(root);
            let roots = super::discover_agent_roots();
            for product in ["antigravity", "antigravity-cli"] {
                assert!(
                    roots
                        .iter()
                        .any(|entry| entry.agent == gnomcode_core::AgentId::Antigravity
                            && entry.path == root.join(product).join("brain"))
                );
            }
            return;
        }
        let root = tempfile::tempdir().expect("root");
        let status = std::process::Command::new(std::env::current_exe().expect("test binary"))
            .args([
                "--exact",
                "tests::antigravity_cli_and_ide_roots_respect_gemini_home",
            ])
            .env(CHILD, root.path())
            .env("GEMINI_HOME", root.path())
            .env("HOME", root.path())
            .env("USERPROFILE", root.path())
            .env("APPDATA", root.path())
            .env("LOCALAPPDATA", root.path())
            .status()
            .expect("isolated test");
        assert!(status.success());
    }
    #[test]
    fn process_liveness_reports_current_process() {
        assert!(super::process_is_alive(std::process::id()));
        assert!(!super::process_is_alive(0));
    }

    #[test]
    fn stopping_invalid_process_is_rejected() {
        assert!(super::stop_owned_process(0).is_err());
    }

    #[test]
    fn unix_stop_plan_escalates_and_guards_own_process_group() {
        assert_eq!(
            super::UNIX_STOP_ESCALATION,
            [
                super::UnixStopSignal::Interrupt,
                super::UnixStopSignal::Terminate,
                super::UnixStopSignal::Kill,
            ]
        );
        assert_eq!(
            super::select_unix_stop_target(42, 42, 10),
            super::UnixStopTarget::ProcessGroup(42)
        );
        assert_eq!(
            super::select_unix_stop_target(42, 10, 10),
            super::UnixStopTarget::SingleProcess(42)
        );
        assert_eq!(
            super::select_unix_stop_target(42, -1, 10),
            super::UnixStopTarget::SingleProcess(42)
        );
    }

    #[cfg(unix)]
    #[test]
    fn unix_stop_escalates_past_ignored_sigint() {
        use std::os::unix::process::CommandExt as _;
        let mut child = std::process::Command::new("sh")
            .args(["-c", "trap '' INT; sleep 30"])
            .process_group(0)
            .spawn()
            .expect("spawn sh");
        let pid = child.id();
        assert!(super::process_is_alive(pid));
        super::stop_owned_process(pid).expect("stop owned process group");
        let _ = child.wait();
        assert!(!super::process_is_alive(pid));
    }

    #[cfg(windows)]
    #[test]
    fn windows_stop_succeeds_for_already_exited_process() {
        let mut child = std::process::Command::new("cmd.exe")
            .args(["/c", "exit 0"])
            .spawn()
            .expect("spawn cmd");
        let pid = child.id();
        let _ = child.wait().expect("wait exited");
        // Close our handle; Windows can list an exited process briefly while handles remain.
        drop(child);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while super::process_is_alive(pid) && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        assert!(!super::process_is_alive(pid));
        super::stop_owned_process(pid).expect("stopping already-exited process should succeed");
    }
    #[test]
    fn handoff_files_include_modified_and_untracked_paths() {
        let repo = tempfile::tempdir().expect("repo");
        let git = |args: &[&str]| {
            std::process::Command::new("git")
                .args(args)
                .current_dir(repo.path())
                .output()
                .expect("git")
        };
        assert!(git(&["init", "-q"]).status.success());
        std::fs::write(repo.path().join("tracked file.txt"), "old").expect("write");
        assert!(git(&["add", "."]).status.success());
        assert!(
            git(&[
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.invalid",
                "commit",
                "-qm",
                "initial"
            ])
            .status
            .success()
        );
        std::fs::write(repo.path().join("tracked file.txt"), "new").expect("write");
        std::fs::write(repo.path().join("new file.txt"), "new").expect("write");
        let files = super::handoff_files(repo.path()).expect("files");
        assert_eq!(files, ["new file.txt", "tracked file.txt"]);
    }

    #[test]
    fn handoff_files_support_repository_without_commits() {
        let repo = tempfile::tempdir().expect("repo");
        assert!(
            std::process::Command::new("git")
                .args(["init", "-q"])
                .current_dir(repo.path())
                .status()
                .expect("git")
                .success()
        );
        std::fs::write(repo.path().join("staged.txt"), "x").expect("write");
        assert!(
            std::process::Command::new("git")
                .args(["add", "staged.txt"])
                .current_dir(repo.path())
                .status()
                .expect("git")
                .success()
        );
        assert_eq!(
            super::handoff_files(repo.path()).expect("files"),
            ["staged.txt"]
        );
    }
    use super::{open_shared_read, read_from_offset};
    use std::io::Write as _;
    use tempfile::NamedTempFile;

    #[test]
    fn shared_read_from_offset_handles_append_and_truncation() {
        let mut tmp = NamedTempFile::new().expect("create temp file");
        tmp.write_all(b"line1\n").expect("write line1");
        tmp.flush().expect("flush");

        let chunk1 = read_from_offset(tmp.path(), 0).expect("read chunk 1");
        assert_eq!(chunk1.bytes, b"line1\n");
        assert_eq!(chunk1.file_size, 6);
        assert_eq!(chunk1.effective_start_offset, 0);

        // Append while another shared handle is open
        let _held_read = open_shared_read(tmp.path()).expect("open concurrent shared read");
        tmp.write_all(b"line2\n")
            .expect("append line2 while shared handle open");
        tmp.flush().expect("flush");

        let chunk2 = read_from_offset(tmp.path(), chunk1.file_size).expect("read chunk 2");
        assert_eq!(chunk2.bytes, b"line2\n");
        assert_eq!(chunk2.effective_start_offset, 6);

        // Simulate truncation (offset > file_size resets to 0)
        let chunk3 = read_from_offset(tmp.path(), 9999).expect("read after simulated truncation");
        assert_eq!(chunk3.effective_start_offset, 0);
        assert_eq!(chunk3.bytes, b"line1\nline2\n");
    }

    #[test]
    fn git_repo_and_worktree_canonicalization_resolves_common_root_and_remote() {
        use super::resolve_git_repo_and_remote;
        use std::fs;
        use tempfile::tempdir;

        let dir = tempdir().expect("create temp dir");
        let repo_root = dir.path().join("my-repo");
        let dot_git = repo_root.join(".git");
        fs::create_dir_all(&dot_git).expect("create .git dir");

        let git_config = "[core]\n\trepositoryformatversion = 0\n[remote \"origin\"]\n\turl = git@github.com:mwahaj36/gnomcode.git\n\tfetch = +refs/heads/*:refs/remotes/origin/*\n";
        fs::write(dot_git.join("config"), git_config).expect("write git config");

        // 1. Direct repo lookup
        let subfolder = repo_root.join("src").join("inner");
        fs::create_dir_all(&subfolder).expect("create subfolder");
        let (resolved_root, remote) = resolve_git_repo_and_remote(&subfolder);
        let expected_root = repo_root.canonicalize().unwrap_or(repo_root.clone());
        let clean_path = |p: &std::path::Path| {
            p.to_str()
                .unwrap_or_default()
                .strip_prefix(r"\\?\")
                .unwrap_or(p.to_str().unwrap_or_default())
                .to_ascii_lowercase()
        };
        assert_eq!(
            resolved_root.as_deref().map(clean_path),
            Some(clean_path(expected_root.as_path()))
        );
        assert_eq!(
            remote.as_deref(),
            Some("git@github.com:mwahaj36/gnomcode.git")
        );

        #[cfg(unix)]
        {
            let alias = dir.path().join("repo-alias");
            std::os::unix::fs::symlink(&repo_root, &alias).expect("symlink");
            let (alias_root, _) = resolve_git_repo_and_remote(&alias.join("src"));
            assert_eq!(alias_root, Some(expected_root.clone()));
        }

        // 2. Worktree simulation
        let worktree_dir = dir.path().join("my-repo-worktree");
        fs::create_dir_all(&worktree_dir).expect("create worktree dir");
        let wt_gitdir = dot_git.join("worktrees").join("feature-branch");
        fs::create_dir_all(&wt_gitdir).expect("create wt gitdir");
        fs::write(wt_gitdir.join("commondir"), "../..\n").expect("write commondir");
        fs::write(
            worktree_dir.join(".git"),
            format!("gitdir: {}\n", wt_gitdir.display()),
        )
        .expect("write .git file");

        let (wt_root, wt_remote) = resolve_git_repo_and_remote(&worktree_dir);
        assert_eq!(
            wt_root.as_deref().map(clean_path),
            Some(clean_path(expected_root.as_path()))
        );
        assert_eq!(
            wt_remote.as_deref(),
            Some("git@github.com:mwahaj36/gnomcode.git")
        );
    }
}

/// Atomically writes account credentials with owner-only file access.
///
/// # Errors
/// Fails without replacing the existing file if permissions or writing fail.
pub fn write_private_file(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    use std::io::Write as _;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT_FILE: AtomicU64 = AtomicU64::new(0);
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("Missing parent directory"))?;
    fs::create_dir_all(parent)?;
    let pending = parent.join(format!(
        ".gnomcode-private-{}-{}.tmp",
        std::process::id(),
        NEXT_FILE.fetch_add(1, Ordering::Relaxed)
    ));
    let mut opts = OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        opts.mode(0o600);
    }
    let result = (|| -> io::Result<()> {
        let mut file = opts.open(&pending)?;
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt as _;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            let account = match (env::var("USERDOMAIN"), env::var("USERNAME")) {
                (Ok(domain), Ok(user)) if !domain.is_empty() && !user.is_empty() => {
                    format!(r"{domain}\{user}:(F)")
                }
                (_, Ok(user)) if !user.is_empty() => format!("{user}:(F)"),
                _ => String::new(),
            };
            // Reset explicit grants before removing inheritance. Some Windows profiles
            // create files with explicit grants that /inheritance:r alone preserves.
            let fast_ok = !account.is_empty()
                && std::process::Command::new("icacls.exe")
                    .arg(&pending)
                    .args(["/reset", "/Q"])
                    .creation_flags(CREATE_NO_WINDOW)
                    .output()
                    .is_ok_and(|out| out.status.success())
                && std::process::Command::new("icacls.exe")
                    .arg(&pending)
                    .args(["/inheritance:r", "/grant:r", &account, "/Q"])
                    .creation_flags(CREATE_NO_WINDOW)
                    .output()
                    .is_ok_and(|out| out.status.success());
            if !fast_ok {
                let output = std::process::Command::new("powershell.exe")
                    .args(["-NoProfile", "-NonInteractive", "-Command", "$ErrorActionPreference='Stop'; $acl=New-Object System.Security.AccessControl.FileSecurity; $sid=[System.Security.Principal.WindowsIdentity]::GetCurrent().User; $acl.SetOwner($sid); $acl.SetAccessRuleProtection($true,$false); $acl.AddAccessRule((New-Object System.Security.AccessControl.FileSystemAccessRule($sid,'FullControl','Allow'))); [IO.File]::SetAccessControl($env:GNOMCODE_PRIVATE_FILE,$acl)"])
                    .env("GNOMCODE_PRIVATE_FILE", &pending)
                    .creation_flags(CREATE_NO_WINDOW).output()?;
                if !output.status.success() {
                    return Err(io::Error::other("Cannot restrict credential file access"));
                }
            }
        }
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&pending, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&pending);
    }
    result.map_err(Error::from)
}

#[cfg(target_os = "windows")]
mod disk_free {
    #![allow(unsafe_code)]

    use std::os::windows::ffi::OsStrExt as _;
    use std::path::Path;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetDiskFreeSpaceExW(
            directory_name: *const u16,
            free_bytes_available_to_caller: *mut u64,
            total_number_of_bytes: *mut u64,
            total_number_of_free_bytes: *mut u64,
        ) -> i32;
    }

    #[allow(clippy::useless_conversion, clippy::unnecessary_fallible_conversions)]
    pub(super) fn available_bytes(existing: &Path) -> Option<u64> {
        let wide: Vec<u16> = existing
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let mut available = 0u64;
        // SAFETY: `wide` is NUL-terminated UTF-16 for an existing path and `available` is a valid out-pointer.
        let ok = unsafe {
            GetDiskFreeSpaceExW(
                wide.as_ptr(),
                &mut available,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        (ok != 0).then_some(available)
    }
}

#[cfg(unix)]
mod disk_free {
    #![allow(unsafe_code)]

    use std::ffi::CString;
    use std::mem::MaybeUninit;
    use std::os::unix::ffi::OsStrExt as _;
    use std::path::Path;

    #[allow(clippy::useless_conversion, clippy::unnecessary_fallible_conversions)]
    pub(super) fn available_bytes(existing: &Path) -> Option<u64> {
        let c_path = CString::new(existing.as_os_str().as_bytes()).ok()?;
        let mut stats = MaybeUninit::<libc::statvfs>::uninit();
        // SAFETY: `c_path` is a NUL-terminated path and `stats` is valid uninitialized `statvfs` storage.
        let rc = unsafe { libc::statvfs(c_path.as_ptr(), stats.as_mut_ptr()) };
        if rc != 0 {
            return None;
        }
        // SAFETY: `statvfs` returned 0, so `stats` is initialized.
        let stats = unsafe { stats.assume_init() };
        let unit = if stats.f_frsize > 0 {
            stats.f_frsize
        } else {
            stats.f_bsize
        };
        u64::try_from(stats.f_bavail)
            .ok()
            .zip(u64::try_from(unit).ok())
            .and_then(|(blocks, size)| blocks.checked_mul(size))
    }
}

/// Returns available free bytes on the filesystem containing `path` (or its nearest existing ancestor).
#[must_use]
pub fn disk_free_bytes(path: &Path) -> Option<u64> {
    // Query only the target volume directly; enumerating all volumes hangs on virtual/cloud mounts.
    let existing = path.ancestors().find(|candidate| candidate.exists())?;
    disk_free::available_bytes(existing)
}

/// Resolves symlinks on the existing prefix of a watched path so OS file-watcher events match.
#[must_use]
pub fn canonical_watch_path(path: &Path) -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        if let Some(existing) = path.ancestors().find(|candidate| candidate.exists())
            && let Ok(canon) = std::fs::canonicalize(existing)
            && let Ok(suffix) = path.strip_prefix(existing)
        {
            return if suffix.as_os_str().is_empty() {
                canon
            } else {
                canon.join(suffix)
            };
        }
    }
    path.to_path_buf()
}

/// Opens an OS-held exclusive file lock, released when the returned file closes.
///
/// # Errors
/// Returns an error when another process owns the lock or the file cannot be opened.
pub fn open_exclusive_lock(path: &Path) -> io::Result<File> {
    let mut opts = OpenOptions::new();
    opts.write(true).create(true).truncate(false);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::OpenOptionsExt as _;
        opts.share_mode(0);
    }
    let file = opts.open(path)?;
    #[cfg(unix)]
    file_lock::lock(&file)?;
    Ok(file)
}

#[cfg(unix)]
mod file_lock {
    #![allow(unsafe_code)]
    use std::os::fd::AsRawFd as _;
    pub(super) fn lock(file: &std::fs::File) -> std::io::Result<()> {
        // SAFETY: the file descriptor is valid for the call; flock retains no Rust pointer.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }
}

#[cfg(test)]
mod subscription_privacy_tests {
    #[test]
    fn claude_credentials_do_not_set_subscription_plan() {
        if std::env::var_os("GNOMCODE_TEST_CREDENTIAL_CHILD").is_some() {
            assert!(super::detect_local_subscriptions().anthropic.is_none());
            return;
        }
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join(".credentials.json"),
            br#"{"subscriptionType":"pro","token":"test-only"}"#,
        )
        .expect("fixture");
        let output = std::process::Command::new(std::env::current_exe().expect("test executable"))
            .args([
                "subscription_privacy_tests::claude_credentials_do_not_set_subscription_plan",
                "--exact",
            ])
            .env("GNOMCODE_TEST_CREDENTIAL_CHILD", "1")
            .env("CLAUDE_CONFIG_DIR", dir.path())
            .env("GEMINI_HOME", dir.path())
            .output()
            .expect("child test");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
}
#[cfg(test)]
mod private_file_tests {
    #[test]
    fn replaces_credentials_without_leaving_partial_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("session.json");
        super::write_private_file(&path, b"old").expect("write");
        super::write_private_file(&path, b"new").expect("replace");
        assert_eq!(std::fs::read(&path).expect("read"), b"new");
        assert_eq!(std::fs::read_dir(dir.path()).expect("files").count(), 1);
        #[cfg(target_os = "windows")]
        {
            let output = std::process::Command::new("powershell.exe").args(["-NoProfile","-NonInteractive","-Command", "$acl=[IO.File]::GetAccessControl($env:GNOMCODE_PRIVATE_FILE,[Security.AccessControl.AccessControlSections]::Access); $sid=[Security.Principal.WindowsIdentity]::GetCurrent().User; $rules=@($acl.GetAccessRules($true,$true,[Security.Principal.SecurityIdentifier])); if(!$acl.AreAccessRulesProtected -or $rules.Count -ne 1 -or $rules[0].IdentityReference.Value -ne $sid.Value){exit 1}; exit 0"]).env("GNOMCODE_PRIVATE_FILE",&path).output().expect("inspect private ACL");
            assert!(
                output.status.success(),
                "credentials must retain one owner-only protected access rule"
            );
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            assert_eq!(
                std::fs::metadata(path)
                    .expect("metadata")
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
    }
}

#[cfg(test)]
mod daemon_lock_tests {
    #[test]
    fn second_daemon_cannot_replace_a_running_daemons_credentials() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("daemon.lock");
        let first = super::open_exclusive_lock(&path).expect("first daemon");
        assert!(super::open_exclusive_lock(&path).is_err());
        drop(first);
        assert!(super::open_exclusive_lock(&path).is_ok());
    }
}

#[cfg(test)]
mod gnomcode_home_tests {
    #[test]
    fn honors_override_and_uses_gnomcode_default() {
        if let Ok(expected) = std::env::var("GNOMCODE_TEST_EXPECTED_HOME") {
            assert_eq!(super::gnomcode_home(), std::path::PathBuf::from(expected));
            return;
        }
        let root = tempfile::tempdir().expect("home");
        for (custom, legacy, legacy_override) in [
            (false, false, false),
            (true, false, false),
            (false, true, false),
            (false, false, true),
            (true, true, true),
        ] {
            let legacy_home = root.path().join(".spawnt");
            if legacy {
                std::fs::create_dir_all(&legacy_home).expect("legacy history");
            }
            let expected = if custom {
                root.path().join("custom")
            } else if legacy {
                legacy_home.clone()
            } else if legacy_override {
                root.path().join("legacy-custom")
            } else {
                root.path().join(".gnomcode")
            };
            let mut child =
                std::process::Command::new(std::env::current_exe().expect("test executable"));
            child
                .args([
                    "gnomcode_home_tests::honors_override_and_uses_gnomcode_default",
                    "--exact",
                ])
                .env("GNOMCODE_TEST_EXPECTED_HOME", &expected)
                .env("HOME", root.path())
                .env("USERPROFILE", root.path())
                .env_remove("GNOMCODE_HOME")
                .env_remove("SPAWNT_HOME");
            if legacy_override {
                child.env("SPAWNT_HOME", root.path().join("legacy-custom"));
            }
            if custom {
                child.env("GNOMCODE_HOME", &expected);
            } else {
                child.env_remove("GNOMCODE_HOME");
            }
            let output = child.output().expect("child test");
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stdout)
            );
            if legacy {
                std::fs::remove_dir(&legacy_home).expect("remove empty fixture");
            }
        }
    }
}

#[cfg(test)]
mod disk_space_tests {
    #[test]
    fn reports_free_space_for_existing_and_nonexistent_child_paths() {
        let dir = tempfile::tempdir().expect("tempdir");
        let existing = super::disk_free_bytes(dir.path()).expect("disk free bytes");
        assert!(existing > 0);
        let nested = dir.path().join("does-not-exist-yet").join("state");
        let nested_bytes = super::disk_free_bytes(&nested).expect("ancestor disk free bytes");
        assert!(nested_bytes > 0);
    }
}
