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

/// Returns the candidate log root directories for M1 agents, respecting
/// environment variable overrides (`CLAUDE_CONFIG_DIR`, `CODEX_HOME`, `GEMINI_HOME`).
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

    vec![
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
    ]
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
            if !unquoted.is_empty() {
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
}
