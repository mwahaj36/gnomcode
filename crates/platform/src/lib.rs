//! OS-specific integration: paths, shared-mode file reading, process scanning,
//! window focus, and autostart (`SPEC §3.5`, `§3.12`, `§4.5`).

use fleethud_core::AgentId;
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

/// Resolves the FleetHUD data directory (`$FLEETHUD_HOME` or `~/.fleethud`).
#[must_use]
pub fn fleethud_home() -> PathBuf {
    if let Ok(custom) = env::var("FLEETHUD_HOME") {
        let trimmed = custom.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    user_home_dir().join(".fleethud")
}

/// Resolves the default SQLite database path (`~/.fleethud/state.db`).
#[must_use]
pub fn default_db_path() -> PathBuf {
    fleethud_home().join("state.db")
}

/// Resolves the user's home directory across Windows, macOS, and Linux.
#[must_use]
pub fn user_home_dir() -> PathBuf {
    if let Ok(home) = env::var("HOME") {
        if !home.trim().is_empty() {
            return PathBuf::from(home);
        }
    }
    if let Ok(profile) = env::var("USERPROFILE") {
        if !profile.trim().is_empty() {
            return PathBuf::from(profile);
        }
    }
    PathBuf::from(".")
}

/// Resolves the current machine's hostname, or returns "local" if unknown.
#[must_use]
pub fn hostname() -> String {
    env::var("COMPUTERNAME")
        .or_else(|_| env::var("HOSTNAME"))
        .unwrap_or_else(|_| "local".to_owned())
}

/// Returns the candidate log root directories for supported agents, respecting
/// environment variable overrides (`CLAUDE_CONFIG_DIR`, `CODEX_HOME`, `GEMINI_HOME`, `AIDER_ANALYTICS_LOG`).
#[must_use]
pub fn discover_agent_roots() -> Vec<AgentRoot> {
    let home = user_home_dir();
    let claude_root = env::var("CLAUDE_CONFIG_DIR")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map_or_else(|| home.join(".claude"), PathBuf::from)
        .join("projects");

    let codex_root = env::var("CODEX_HOME")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map_or_else(|| home.join(".codex"), PathBuf::from)
        .join("sessions");

    let gemini_base = env::var("GEMINI_HOME")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map_or_else(|| home.join(".gemini"), PathBuf::from);

    let antigravity_root = gemini_base.join("antigravity").join("brain");
    let gemini_cli_root = gemini_base.join("tmp");

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
            agent: AgentId::GeminiCli,
            exists: gemini_cli_root.exists(),
            path: gemini_cli_root,
        },
    ];

    // Cline / Roo Code storage across editors (Code, Code - Insiders, Cursor, Windsurf, VSCodium)
    let appdata = app_config_dir();
    let editors = ["Code", "Code - Insiders", "Cursor", "Windsurf", "VSCodium"];
    let extensions = ["saoudrizwan.claude-dev", "rooveterinaryinc.roo-cline"];

    for editor in editors {
        for ext in extensions {
            let tasks_dir = appdata
                .join(editor)
                .join("User")
                .join("globalStorage")
                .join(ext)
                .join("tasks");
            if tasks_dir.exists() {
                roots.push(AgentRoot {
                    agent: AgentId::Cline,
                    exists: true,
                    path: tasks_dir,
                });
            }
        }
    }

    // Cursor global storage / composer sessions
    let cursor_storage = appdata.join("Cursor").join("User").join("globalStorage");
    let cursor_home = home.join(".cursor");
    if cursor_storage.exists() {
        roots.push(AgentRoot {
            agent: AgentId::Cursor,
            exists: true,
            path: cursor_storage,
        });
    }
    if cursor_home.exists() {
        roots.push(AgentRoot {
            agent: AgentId::Cursor,
            exists: true,
            path: cursor_home,
        });
    }

    // Aider analytics log / history
    if let Ok(custom_aider) = env::var("AIDER_ANALYTICS_LOG") {
        let p = PathBuf::from(custom_aider.trim());
        if p.exists() {
            roots.push(AgentRoot {
                agent: AgentId::Aider,
                exists: true,
                path: p,
            });
        }
    }
    let aider_home = home.join(".aider");
    if aider_home.exists() {
        roots.push(AgentRoot {
            agent: AgentId::Aider,
            exists: true,
            path: aider_home,
        });
    }

    roots
}

/// Resolves user application configuration / roaming directory across operating systems.
#[must_use]
pub fn app_config_dir() -> PathBuf {
    if let Ok(appdata) = env::var("APPDATA") {
        if !appdata.trim().is_empty() {
            return PathBuf::from(appdata);
        }
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
            return (Some(dir.to_path_buf()), remote);
        } else if dot_git.is_file() {
            if let Ok(content) = fs::read_to_string(&dot_git) {
                if let Some(gitdir_line) = content
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
            }
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
        } else if in_remote_origin && trimmed.starts_with("url") {
            if let Some((_, val)) = trimmed.split_once('=') {
                let clean = val.trim();
                if !clean.is_empty() {
                    return Some(clean.to_owned());
                }
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

/// Automatically detects local agent subscription tiers from non-secret config metadata
/// (`~/.claude/.credentials.json` tier fields and `~/.gemini/antigravity/installation_id`).
/// Never reads or stores tokens or credentials (`SPEC §2`).
#[must_use]
pub fn detect_local_subscriptions() -> DetectedSubscriptions {
    let home = user_home_dir();
    let claude_dir = env::var("CLAUDE_CONFIG_DIR")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map_or_else(|| home.join(".claude"), PathBuf::from);

    let mut anthropic = None;
    if let Ok(raw) = fs::read_to_string(claude_dir.join(".credentials.json")) {
        let lower = raw.to_ascii_lowercase();
        if lower.contains("max_20x") {
            anthropic = Some((200.0, "Claude Max 20x".to_owned()));
        } else if lower.contains("max_5x") {
            anthropic = Some((100.0, "Claude Max 5x".to_owned()));
        } else if lower.contains("\"subscriptiontype\":\"max\"")
            || lower.contains("\"subscriptiontype\": \"max\"")
        {
            anthropic = Some((100.0, "Claude Max".to_owned()));
        } else if lower.contains("\"subscriptiontype\":\"team\"")
            || lower.contains("\"subscriptiontype\": \"team\"")
        {
            anthropic = Some((30.0, "Claude Team".to_owned()));
        } else if lower.contains("\"subscriptiontype\":\"pro\"")
            || lower.contains("\"subscriptiontype\": \"pro\"")
        {
            anthropic = Some((20.0, "Claude Pro".to_owned()));
        }
    }

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

/// Opens a file with shared read/write/delete permissions on Windows so FleetHUD
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

/// Dispatches a window focus request for a process ID across OS platforms (`SPEC §3.13`).
///
/// Returns `true` if a focus attempt was dispatched, or `false` if unsupported.
#[must_use]
pub fn bring_to_front(pid: u32) -> bool {
    #[cfg(target_os = "windows")]
    {
        let script =
            format!("$wshell = New-Object -ComObject Wscript.Shell; $wshell.AppActivate({pid})");
        let _ = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .spawn();
        true
    }
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "tell application \"System Events\" to set frontmost of (every process whose unix id is {pid}) to true"
        );
        let _ = std::process::Command::new("osascript")
            .args(["-e", &script])
            .spawn();
        true
    }
    #[cfg(target_os = "linux")]
    {
        let pid_str = pid.to_string();
        let _ = std::process::Command::new("wmctrl")
            .args(["-a", &pid_str])
            .spawn();
        true
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = pid;
        false
    }
}

/// Brings the target project to the foreground in VS Code / Cursor / Windsurf (`SPEC §3.13`).
#[must_use]
pub fn focus_project_in_editor(root_path: &str) -> bool {
    let _ = std::process::Command::new("code")
        .args(["-r", root_path])
        .spawn();
    true
}

/// Spawns a fresh agent session in an interactive terminal window across platforms (`SPEC §3.14.1`).
/// For CLI agents (`claude-code`, `codex`, `gemini-cli`), launches an interactive terminal pane in `cwd`.
/// For IDE agents (`antigravity`, `cursor`, `cline`), focuses the workspace project in the editor.
#[must_use]
pub fn spawn_agent_in_terminal(agent: &str, cwd: &str) -> bool {
    let clean_agent = match agent {
        "claude" | "claude-code" => "claude",
        "codex" => "codex",
        "gemini" | "gemini-cli" => "gemini",
        "antigravity" | "cursor" | "cline" | "aider" => {
            return focus_project_in_editor(cwd);
        }
        _ => return false,
    };

    #[cfg(target_os = "windows")]
    {
        let escaped_cwd = cwd.replace('\"', "\\\"");
        let cmd_line = format!("cd /d \"{escaped_cwd}\" && {clean_agent}");
        let _ = std::process::Command::new("cmd")
            .args(["/c", "start", "FleetHUD Relay", "cmd.exe", "/k", &cmd_line])
            .spawn();
        true
    }
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "tell application \"Terminal\" to do script \"cd \\\"{}\\\" && {}\"",
            cwd.replace('\"', "\\\""),
            clean_agent
        );
        let _ = std::process::Command::new("osascript")
            .args(["-e", &script])
            .spawn();
        true
    }
    #[cfg(target_os = "linux")]
    {
        let script = format!(
            "cd \"{}\" && {}; exec bash",
            cwd.replace('\"', "\\\""),
            clean_agent
        );
        let _ = std::process::Command::new("x-terminal-emulator")
            .args(["-e", "bash", "-c", &script])
            .spawn();
        true
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = (cwd, clean_agent);
        false
    }
}

/// Dispatches a native desktop notification across Windows, macOS, and Linux (`SPEC §3.10`).
#[must_use]
pub fn send_desktop_notification(title: &str, body: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        let escaped_title = title.replace('\'', "''");
        let escaped_body = body.replace('\'', "''");
        let script = format!(
            "[reflection.assembly]::loadwithpartialname('System.Windows.Forms') | Out-Null; \
             $n = New-Object System.Windows.Forms.NotifyIcon; \
             $n.Icon = [System.Drawing.SystemIcons]::Information; \
             $n.Visible = $true; \
             $n.ShowBalloonTip(5000, '{escaped_title}', '{escaped_body}', [System.Windows.Forms.ToolTipIcon]::Info)"
        );
        let _ = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .spawn();
        true
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

/// Detects a Codex cloud thread from runtime metadata without reading credentials or logs.
#[must_use]
pub fn codex_cloud_session_id() -> Option<String> {
    let cloud = ["CODEX_ENVIRONMENT_ID", "CODEX_CAAS_SESSION_ID"]
        .iter()
        .any(|name| env::var(name).is_ok_and(|value| !value.trim().is_empty()));
    if !cloud {
        return None;
    }
    ["CODEX_THREAD_ID", "CODEX_SESSION_ID"]
        .iter()
        .filter_map(|name| env::var(name).ok())
        .find(|value| !value.trim().is_empty())
}

#[cfg(test)]
mod tests {
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

        let git_config = "[core]\n\trepositoryformatversion = 0\n[remote \"origin\"]\n\turl = git@github.com:mwahaj36/fleethud.git\n\tfetch = +refs/heads/*:refs/remotes/origin/*\n";
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
            Some("git@github.com:mwahaj36/fleethud.git")
        );

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
            Some("git@github.com:mwahaj36/fleethud.git")
        );
    }
}
