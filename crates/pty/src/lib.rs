//! Agent launch validation, independent session hosting, and terminal sessions.

pub mod acp_approvals;
pub mod codex_app_server;
pub mod protocol_host;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::io::{Read, Write};
#[cfg(windows)]
use std::os::windows::process::CommandExt as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::broadcast;
#[cfg(test)]
mod environment_tests;

/// Error type for PTY operations.
#[derive(Debug, Error)]
pub enum Error {
    /// Native startup or approval must finish before accepting chat input.
    #[error(
        "The agent is starting or needs native approval. Open Agent terminal, finish setup, then retry. Your draft is preserved."
    )]
    NotReady,
    /// Workspace does not exist.
    #[error("workspace directory does not exist: {0}")]
    WorkspaceNotFound(String),
    /// Agent is not supported.
    #[error("unsupported agent: {0}")]
    UnsupportedAgent(String),
    /// Model argument is invalid or unsafe.
    #[error("invalid or unsafe model argument: {0}")]
    InvalidModel(String),
    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Serialization error.
    #[error("serialization error: {0}")]
    Json(#[from] serde_json::Error),
    /// Session not found.
    #[error("session not found: {0}")]
    SessionNotFound(String),
    /// Process failure.
    #[error("process error: {0}")]
    Process(String),
}

/// A validated agent executable and its arguments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaunchCommand {
    /// Executable name.
    pub executable: String,
    /// Arguments supplied to the executable.
    pub arguments: Vec<String>,
}

/// Validates a launch request and constructs the official agent command.
///
/// # Errors
/// Returns [`Error`] if workspace does not exist, agent is unsupported, or model contains unsafe flags.
pub fn launch_command(
    agent: &str,
    repo: &Path,
    model: Option<&str>,
) -> Result<LaunchCommand, Error> {
    if !repo.is_dir() {
        return Err(Error::WorkspaceNotFound(repo.display().to_string()));
    }
    let executable = match agent {
        "claude" | "claude-code" => "claude".to_owned(),
        "codex" => "codex".to_owned(),
        "antigravity" | "agy" => "agy".to_owned(),
        "gemini" | "gemini-cli" => "gemini".to_owned(),
        "terminal" | "shell" => {
            return Ok({
                let (executable, arguments) = gnomcode_platform::executables::terminal_shell();
                LaunchCommand {
                    executable,
                    arguments,
                }
            });
        }
        other => return Err(Error::UnsupportedAgent(other.to_owned())),
    };
    let mut arguments = Vec::new();
    if let Some(model) = model {
        if model.is_empty() || model.starts_with('-') {
            return Err(Error::InvalidModel(model.to_owned()));
        }
        arguments.extend(["--model".to_owned(), model.to_owned()]);
    }
    Ok(LaunchCommand {
        executable,
        arguments,
    })
}

/// Information about an independently running PTY session.
///
/// # Errors
/// Returns an error for unsupported agents or unsafe session identifiers.
pub fn resume_command(agent: &str, repo: &Path, native_id: &str) -> Result<LaunchCommand, Error> {
    if native_id.is_empty()
        || native_id.starts_with('-')
        || native_id.len() > 128
        || !native_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte))
    {
        return Err(Error::SessionNotFound(native_id.to_owned()));
    }
    let mut command = launch_command(agent, repo, None)?;
    let verb = match agent {
        "codex" => "resume",
        "claude" | "claude-code" | "gemini" | "gemini-cli" => "--resume",
        "antigravity" | "agy" => "--conversation",
        _ => return Err(Error::UnsupportedAgent(agent.to_owned())),
    };
    command.arguments = vec![verb.to_owned(), native_id.to_owned()];
    // Native agy resume defaults to Auto rather than restoring the previous permission mode.
    if matches!(agent, "antigravity" | "agy") {
        command
            .arguments
            .extend(["--mode".to_owned(), "plan".to_owned()]);
    }
    Ok(command)
}

/// Information about an independently running PTY session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionRecord {
    /// Unique session identifier.
    pub session_id: String,
    /// Agent identifier.
    pub agent: String,
    /// Repository path.
    pub repo_path: String,
    /// Target model name.
    pub model: Option<String>,
    /// Session host process ID (owner).
    pub host_pid: u32,
    /// Child process ID (agent).
    pub child_pid: u32,
    /// Loopback port the host is listening on.
    pub port: u16,
    /// Bearer authentication token for this session.
    pub token: String,
    /// Timestamp when started (RFC3339).
    pub started_at: String,
}

/// Directory where active session records are stored.
#[must_use]
pub fn sessions_dir() -> PathBuf {
    gnomcode_platform::gnomcode_home().join("sessions")
}

/// Path to a specific session's JSON record file.
#[must_use]
pub fn session_file_path(session_id: &str) -> PathBuf {
    sessions_dir().join(format!("{session_id}.json"))
}

fn valid_session_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}

/// Saves a session record to disk with owner-only access.
///
/// # Errors
/// Returns [`Error`] if creating the directory or writing fails.
pub fn save_session_record(record: &SessionRecord) -> Result<(), Error> {
    if !valid_session_id(&record.session_id) {
        return Err(Error::SessionNotFound(record.session_id.clone()));
    }
    let dir = sessions_dir();
    fs::create_dir_all(&dir)?;
    let path = session_file_path(&record.session_id);
    let bytes = serde_json::to_vec_pretty(record)?;
    gnomcode_platform::write_private_file(&path, &bytes)
        .map_err(|e| Error::Process(e.to_string()))?;
    Ok(())
}

/// Loads a session record by ID.
///
/// # Errors
/// Returns [`Error::SessionNotFound`] or [`Error`] if reading fails.
pub fn load_session_record(session_id: &str) -> Result<SessionRecord, Error> {
    if !valid_session_id(session_id) {
        return Err(Error::SessionNotFound(session_id.to_owned()));
    }
    let path = session_file_path(session_id);
    if !path.is_file() {
        return Err(Error::SessionNotFound(session_id.to_owned()));
    }
    let content = fs::read_to_string(path)?;
    let record: SessionRecord = serde_json::from_str(&content)?;
    Ok(record)
}

/// Removes a session record from disk.
pub fn remove_session_record(session_id: &str) {
    if !valid_session_id(session_id) {
        return;
    }
    let path = session_file_path(session_id);
    let _ = fs::remove_file(path);
}

/// Returns all active session records whose host processes are currently running.
#[must_use]
pub fn list_active_sessions() -> Vec<SessionRecord> {
    let dir = sessions_dir();
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut sessions = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "json")
            && let Ok(content) = fs::read_to_string(&path)
            && let Ok(record) = serde_json::from_str::<SessionRecord>(&content)
        {
            if is_pid_alive(record.host_pid) {
                sessions.push(record);
            } else {
                let _ = fs::remove_file(&path);
            }
        }
    }
    sessions
}

/// Checks whether a process ID is currently alive.
#[must_use]
pub fn is_pid_alive(pid: u32) -> bool {
    gnomcode_platform::process_is_alive(pid)
}

/// Bounded in-memory terminal output buffer plus rendered VT100 screen.
pub struct Scrollback {
    capacity: usize,
    bytes: VecDeque<u8>,
    parser: vt100::Parser,
}

impl std::fmt::Debug for Scrollback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scrollback")
            .field("capacity", &self.capacity)
            .field("len", &self.bytes.len())
            .finish()
    }
}

impl Scrollback {
    /// Creates a scrollback buffer with a given byte limit and default 24x160 screen.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            bytes: VecDeque::new(),
            parser: vt100::Parser::new(24, 160, 0),
        }
    }

    /// Appends output to the buffer and advances the rendered VT100 screen.
    pub fn append(&mut self, data: &[u8]) {
        self.parser.process(data);
        let keep = data.len().min(self.capacity);
        let remove = self
            .bytes
            .len()
            .saturating_add(keep)
            .saturating_sub(self.capacity);
        self.bytes.drain(..remove);
        self.bytes.extend(&data[data.len() - keep..]);
    }

    /// Resizes the tracked VT100 screen.
    pub fn resize(&mut self, rows: u16, cols: u16) {
        self.parser.set_size(rows, cols);
    }

    /// Returns a copy of the buffered bytes in sequence.
    #[must_use]
    pub fn bytes(&self) -> Vec<u8> {
        self.bytes.iter().copied().collect()
    }
}

/// Query parameters for WebSocket connection.
#[derive(Debug, Deserialize)]
struct WsQuery {
    token: Option<String>,
}

/// Shared state for the session host server.
struct HostState {
    agent: String,
    token: String,
    session_id: String,
    child_pid: u32,
    scrollback: Arc<Mutex<Scrollback>>,
    output_tx: broadcast::Sender<Vec<u8>>,
    input_tx: std::sync::mpsc::Sender<PtyInput>,
    stopping: Arc<AtomicBool>,
    resize: Arc<dyn Fn(u16, u16) + Send + Sync>,
    protocol: bool,
}

pub(crate) struct PtyInput {
    pub(crate) bytes: Vec<u8>,
    pub(crate) written: Option<tokio::sync::oneshot::Sender<bool>>,
}

/// Runs the independent PTY session host server.
///
/// This holds the PTY master, starts the child process, and serves
/// authenticated WebSocket streaming and termination endpoints on loopback.
///
/// # Errors
/// Returns [`Error`] if spawning or listening fails.
pub async fn run_pty_host(
    session_id: &str,
    agent: &str,
    repo: &Path,
    model: Option<&str>,
    override_command: Option<(&str, &[&str])>,
) -> Result<(), Error> {
    run_pty_host_resuming(session_id, agent, repo, model, override_command, None).await
}

/// Runs a host with an optional official native resume command.
///
/// # Errors
/// Returns an error if validation, process creation or host startup fails.
pub async fn run_pty_host_resuming(
    session_id: &str,
    agent: &str,
    repo: &Path,
    model: Option<&str>,
    override_command: Option<(&str, &[&str])>,
    resume_id: Option<&str>,
) -> Result<(), Error> {
    if !valid_session_id(session_id) {
        return Err(Error::SessionNotFound(session_id.to_owned()));
    }
    let token = {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).map_err(|e| Error::Process(e.to_string()))?;
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
    };

    let scrollback = Arc::new(Mutex::new(Scrollback::new(SCROLLBACK_BYTES)));
    let (output_tx, _) = broadcast::channel(128);
    let (input_tx, input_rx) = std::sync::mpsc::channel::<PtyInput>();
    let stopping = Arc::new(AtomicBool::new(false));

    #[cfg(windows)]
    let (child_pid, mut writer, session_handle) = {
        let (cmd, args) = if let Some((c, a)) = override_command {
            (c.to_owned(), a.iter().map(|s| (*s).to_owned()).collect())
        } else {
            let launch = match resume_id {
                Some(id) => resume_command(agent, repo, id)?,
                None => launch_command(agent, repo, model)?,
            };
            let (executable, mut arguments) =
                gnomcode_platform::executables::agent_command(&launch.executable);
            arguments.extend(launch.arguments);
            (executable, arguments)
        };

        let mut command = conpty_oxide::blocking::Command::new(cmd);
        command.args(&args);
        command.current_dir(repo);
        if matches!(agent, "claude" | "claude-code") {
            for key in gnomcode_platform::executables::CLAUDE_PARENT_ENV_VARS {
                command.env_remove(key);
            }
        }
        let session = command.spawn().map_err(|e| Error::Process(e.to_string()))?;
        let parts = session.into_parts();
        let pid = parts.child.id();
        (
            pid,
            parts.input,
            (parts.child, parts.output, parts.controller),
        )
    };

    #[cfg(unix)]
    let (child_pid, mut writer, master, mut child) = {
        let (cmd, args) = if let Some((c, a)) = override_command {
            (c.to_owned(), a.iter().map(|s| (*s).to_owned()).collect())
        } else {
            let launch = match resume_id {
                Some(id) => resume_command(agent, repo, id)?,
                None => launch_command(agent, repo, model)?,
            };
            (launch.executable, launch.arguments)
        };

        let pair = portable_pty::native_pty_system()
            .openpty(portable_pty::PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| Error::Process(e.to_string()))?;

        let mut command = portable_pty::CommandBuilder::new(cmd);
        command.args(&args);
        command.cwd(repo);
        if matches!(agent, "claude" | "claude-code") {
            for key in gnomcode_platform::executables::CLAUDE_PARENT_ENV_VARS {
                command.env_remove(key);
            }
        }
        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|e| Error::Process(e.to_string()))?;
        drop(pair.slave);
        let pid = child.process_id().unwrap_or(0);
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| Error::Process(e.to_string()))?;
        (pid, writer, pair.master, child)
    };

    // Background thread: write received input to child PTY stdin
    std::thread::spawn(move || {
        while let Ok(input) = input_rx.recv() {
            let written = writer.write_all(&input.bytes).is_ok() && writer.flush().is_ok();
            if let Some(acknowledge) = input.written {
                let _ = acknowledge.send(written);
            }
            if !written {
                break;
            }
        }
    });

    // Background thread: read child PTY stdout and broadcast to clients
    let sb_clone = Arc::clone(&scrollback);
    let tx_clone = output_tx.clone();
    let stopping_clone = Arc::clone(&stopping);

    #[cfg(windows)]
    let resize: Arc<dyn Fn(u16, u16) + Send + Sync> = {
        let controller = session_handle.2.clone();
        Arc::new(move |cols, rows| {
            if let Ok(size) = conpty_oxide::Size::try_new(cols, rows) {
                let _ = controller.resize(size);
            }
        })
    };
    #[cfg(windows)]
    std::thread::spawn(move || {
        let (_child, mut output, _controller) = session_handle;
        let mut buffer = [0u8; 4096];
        while !stopping_clone.load(Ordering::Relaxed) {
            match output.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => {
                    let chunk = buffer[..count].to_vec();
                    if let Ok(mut sb) = sb_clone.lock() {
                        sb.append(&chunk);
                    }
                    let _ = tx_clone.send(chunk);
                }
                Err(_) => break,
            }
        }
    });

    #[cfg(unix)]
    let reader = master.try_clone_reader();
    #[cfg(unix)]
    let resize: Arc<dyn Fn(u16, u16) + Send + Sync> = {
        let master = Mutex::new(master);
        Arc::new(move |cols, rows| {
            if let Ok(master) = master.lock() {
                let _ = master.resize(portable_pty::PtySize {
                    cols,
                    rows,
                    pixel_width: 0,
                    pixel_height: 0,
                });
            }
        })
    };
    #[cfg(unix)]
    std::thread::spawn(move || {
        if let Ok(mut reader) = reader {
            let mut buffer = [0u8; 4096];
            while !stopping_clone.load(Ordering::Relaxed) {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(count) => {
                        let chunk = buffer[..count].to_vec();
                        if let Ok(mut sb) = sb_clone.lock() {
                            sb.append(&chunk);
                        }
                        let _ = tx_clone.send(chunk);
                    }
                    Err(_) => break,
                }
            }
        }
        let _ = child.kill();
    });

    serve_host(HostParts {
        session_id,
        agent,
        repo,
        model,
        token,
        child_pid,
        scrollback,
        output_tx,
        input_tx,
        stopping,
        resize,
        protocol: false,
    })
    .await
}

/// Everything the loopback host needs once its child (PTY or protocol driver) is running.
pub(crate) struct HostParts<'a> {
    pub(crate) session_id: &'a str,
    pub(crate) agent: &'a str,
    pub(crate) repo: &'a Path,
    pub(crate) model: Option<&'a str>,
    pub(crate) token: String,
    pub(crate) child_pid: u32,
    pub(crate) scrollback: Arc<Mutex<Scrollback>>,
    pub(crate) output_tx: broadcast::Sender<Vec<u8>>,
    pub(crate) input_tx: std::sync::mpsc::Sender<PtyInput>,
    pub(crate) stopping: Arc<AtomicBool>,
    pub(crate) resize: Arc<dyn Fn(u16, u16) + Send + Sync>,
    /// Protocol hosts have no terminal screen, so the PTY readiness checks do not apply.
    pub(crate) protocol: bool,
}

/// Binds the authenticated loopback listener, records the session and serves until the child exits.
pub(crate) async fn serve_host(parts: HostParts<'_>) -> Result<(), Error> {
    let HostParts {
        session_id,
        agent,
        repo,
        model,
        token,
        child_pid,
        scrollback,
        output_tx,
        input_tx,
        stopping,
        resize,
        protocol,
    } = parts;
    // Bind loopback HTTP/WebSocket listener
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    let host_pid = std::process::id();

    let record = SessionRecord {
        session_id: session_id.to_owned(),
        agent: agent.to_owned(),
        repo_path: repo.display().to_string(),
        model: model.map(ToOwned::to_owned),
        host_pid,
        child_pid,
        port,
        token: token.clone(),
        started_at: chrono::Utc::now().to_rfc3339(),
    };
    save_session_record(&record)?;

    let state = Arc::new(HostState {
        agent: agent.to_owned(),
        token,
        session_id: session_id.to_owned(),
        child_pid,
        scrollback,
        output_tx,
        input_tx,
        stopping: Arc::clone(&stopping),
        resize,
        protocol,
    });

    let app = Router::new()
        .route("/health", get(host_health_handler))
        .route("/stream", get(host_stream_handler))
        .route("/input", post(host_input_handler))
        .route("/retry-enter", post(host_retry_enter_handler))
        .layer(axum::extract::DefaultBodyLimit::max(64 * 1024))
        .route("/stop", post(host_stop_handler))
        .with_state(state);

    let server = axum::serve(listener, app);
    let session_id_clean = session_id.to_owned();
    tokio::select! {
        _ = server => {},
        _ = async {
            while !stopping.load(Ordering::Relaxed) {
                tokio::time::sleep(Duration::from_millis(500)).await;
                if !is_pid_alive(child_pid) {
                    break;
                }
            }
        } => {}
    }

    stopping.store(true, Ordering::Relaxed);
    remove_session_record(&session_id_clean);
    Ok(())
}

fn valid_loopback_headers(headers: &HeaderMap) -> bool {
    let host = headers
        .get("host")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<axum::http::uri::Authority>().ok());
    if !host.is_some_and(|h| matches!(h.host(), "localhost" | "127.0.0.1")) {
        return false;
    }
    match headers.get("origin") {
        None => true,
        Some(value) => value
            .to_str()
            .ok()
            .and_then(|v| v.parse::<axum::http::Uri>().ok())
            .is_some_and(|u| {
                matches!(u.scheme_str(), Some("http" | "https"))
                    && matches!(u.host(), Some("localhost" | "127.0.0.1"))
            }),
    }
}

async fn host_health_handler(
    headers: HeaderMap,
    State(state): State<Arc<HostState>>,
) -> impl IntoResponse {
    if !valid_loopback_headers(&headers) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let auth = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    if auth != Some(&state.token) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    Json(serde_json::json!({
        "ok": true,
        "session_id": state.session_id,
        "child_pid": state.child_pid,
    }))
    .into_response()
}

const SCROLLBACK_BYTES: usize = 256 * 1024;
/// Recent characters of rendered text that readiness matching may look at.
const SCREEN_TAIL_CHARS: usize = 8192;

/// Renders terminal bytes through a VT100 screen model and returns each row.
fn render_screen_rows(bytes: &[u8]) -> Vec<String> {
    let mut parser = vt100::Parser::new(24, 160, 0);
    parser.process(bytes);
    parser.screen().rows(0, 160).collect()
}

/// Lower-cased, whitespace-collapsed text of what the rendered VT100 screen shows now.
fn screen_text(bytes: &[u8], _cut: bool) -> String {
    let text = render_screen_rows(bytes)
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    let tail = text
        .char_indices()
        .rev()
        .nth(SCREEN_TAIL_CHARS)
        .map_or(0, |(at, _)| at);
    text[tail..].to_owned()
}

fn last_of(text: &str, marks: &[&str]) -> Option<usize> {
    marks.iter().filter_map(|mark| text.rfind(mark)).max()
}

/// Position of the last Claude Code composer box: a long rule followed by the `>` prompt.
fn last_composer(text: &str) -> Option<usize> {
    let (mut rule, mut found) = (0, None);
    for (at, character) in text.char_indices() {
        match character {
            '─' => rule += 1,
            ' ' => {}
            '>' | '❯' if rule >= 10 => {
                found = Some(at);
                rule = 0;
            }
            _ => rule = 0,
        }
    }
    found
}

/// True while the agent shows a startup, trust, login or choice screen, or has not yet
/// shown its normal input prompt. Chat input must wait; it is never answered for the user.
///
/// Matching is per agent on the recent rendered screen, and the latest marker wins. The
/// strings for agy, Codex and Claude Code were seen live (Claude Code v2.1.322, see
/// `fixtures/claude-code/`); the Gemini CLI strings come from its source and are unverified.
fn startup_needs_setup(agent: &str, bytes: &[u8]) -> bool {
    let (blockers, ready, require_ready): (&[&str], &[&str], bool) = match agent {
        "codex" => (&["hooks are new or changed"], &["? for shortcuts"], false),
        "claude" | "claude-code" => (
            &[
                "quick safety check",
                "yes, i trust this folder",
                "yes, i accept",
                "select login method",
                "paste code here if prompted",
                "choose the text style",
                "press enter to continue",
                "detected a custom api key",
                "enter to confirm",
                "esc to cancel",
            ],
            &["? for shortcuts"],
            true,
        ),
        "gemini" | "gemini-cli" => (
            &[
                "do you trust the files in this folder",
                "how would you like to authenticate",
                "(use enter to select)",
                "select theme",
                "waiting for auth",
                "logging in with google",
            ],
            &["type your message or @path/to/file"],
            true,
        ),
        _ => (
            &["do you trust the contents of this project?"],
            &["? for shortcuts"],
            false,
        ),
    };
    let claude = matches!(agent, "claude" | "claude-code");
    let marks = |text: &str| {
        let composer = if claude { last_composer(text) } else { None };
        last_of(text, ready).max(composer)
    };
    let recent = screen_text(bytes, true);
    if let Some(blocked) = last_of(&recent, blockers) {
        return marks(&recent).is_none_or(|ready| ready < blocked);
    }
    // Once the ring wraps, the startup frames are gone and only blockers can be judged.
    require_ready && bytes.len() < SCROLLBACK_BYTES && marks(&screen_text(bytes, false)).is_none()
}

async fn host_input_handler(
    headers: HeaderMap,
    State(state): State<Arc<HostState>>,
    body: axum::body::Bytes,
) -> StatusCode {
    if !valid_loopback_headers(&headers) {
        return StatusCode::FORBIDDEN;
    }
    let authenticated = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.strip_prefix("Bearer ") == Some(state.token.as_str()));
    if !authenticated {
        return StatusCode::UNAUTHORIZED;
    }
    if body.is_empty() {
        return StatusCode::BAD_REQUEST;
    }
    if !state.protocol
        && matches!(
            state.agent.as_str(),
            "antigravity" | "agy" | "codex" | "claude" | "claude-code" | "gemini" | "gemini-cli"
        )
        && (body.starts_with(b"\x1b[200~") || body.starts_with(b"/model "))
        && state.scrollback.lock().map_or(true, |output| {
            let bytes = output.bytes();
            startup_needs_setup(&state.agent, &bytes)
                || (state.agent == "codex" && !codex_prompt_ready(&bytes))
        })
    {
        return StatusCode::CONFLICT;
    }
    let (written, acknowledged) = tokio::sync::oneshot::channel();
    if state
        .input_tx
        .send(PtyInput {
            bytes: body.to_vec(),
            written: Some(written),
        })
        .is_err()
    {
        return StatusCode::GONE;
    }
    match tokio::time::timeout(Duration::from_secs(2), acknowledged).await {
        Ok(Ok(true)) => StatusCode::OK,
        Ok(_) => StatusCode::GONE,
        Err(_) => StatusCode::GATEWAY_TIMEOUT,
    }
}

fn codex_prompt_ready(bytes: &[u8]) -> bool {
    let text = String::from_utf8_lossy(bytes);
    let idle = text
        .rfind("state=idle:app=codex")
        .into_iter()
        .chain(text.rfind("state=done:app=codex"))
        .max();
    let working = text.rfind("state=working:app=codex");
    if let Some(idle) = idle {
        return working.is_none_or(|working| idle > working);
    }
    let rows = render_screen_rows(bytes);
    let tail = rows
        .into_iter()
        .filter(|row| !row.trim().is_empty())
        .rev()
        .take(8)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    tail.contains("? for shortcuts") && !tail.ends_with("loading")
}

/// True only when the rendered VT100 screen proves the composer still holds `prompt`
/// and no startup/confirmation dialog is blocking input.
#[must_use]
pub fn composer_has_unsubmitted_prompt(agent: &str, bytes: &[u8], prompt: &str) -> bool {
    if startup_needs_setup(agent, bytes) {
        return false;
    }
    let normalized_prompt = prompt
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    if normalized_prompt.chars().count() < 3 {
        return false;
    }
    let needle: String = normalized_prompt.chars().take(48).collect();
    let rows = render_screen_rows(bytes);
    let bottom = rows
        .into_iter()
        .filter(|row| !row.trim().is_empty())
        .rev()
        .take(10)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    let marker_pos = ['>', '❯', '›']
        .iter()
        .filter_map(|mark| bottom.rfind(*mark))
        .max();
    let Some(pos) = marker_pos else {
        return false;
    };
    bottom[pos..].contains(&needle)
}

#[derive(Debug, Deserialize)]
struct RetryEnterRequest {
    prompt: String,
}

async fn host_retry_enter_handler(
    headers: HeaderMap,
    State(state): State<Arc<HostState>>,
    Json(payload): Json<RetryEnterRequest>,
) -> StatusCode {
    if !valid_loopback_headers(&headers) {
        return StatusCode::FORBIDDEN;
    }
    let authenticated = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.strip_prefix("Bearer ") == Some(state.token.as_str()));
    if !authenticated {
        return StatusCode::UNAUTHORIZED;
    }
    let confirmed = !state.protocol
        && state.scrollback.lock().is_ok_and(|output| {
            let bytes = output.bytes();
            composer_has_unsubmitted_prompt(&state.agent, &bytes, &payload.prompt)
        });
    if !confirmed {
        return StatusCode::CONFLICT;
    }
    let (written, acknowledged) = tokio::sync::oneshot::channel();
    if state
        .input_tx
        .send(PtyInput {
            bytes: b"\r".to_vec(),
            written: Some(written),
        })
        .is_err()
    {
        return StatusCode::GONE;
    }
    match tokio::time::timeout(Duration::from_secs(2), acknowledged).await {
        Ok(Ok(true)) => StatusCode::OK,
        Ok(_) => StatusCode::GONE,
        Err(_) => StatusCode::GATEWAY_TIMEOUT,
    }
}

async fn host_stop_handler(
    headers: HeaderMap,
    State(state): State<Arc<HostState>>,
) -> impl IntoResponse {
    if !valid_loopback_headers(&headers) {
        return StatusCode::FORBIDDEN;
    }
    let auth = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    if auth != Some(&state.token) {
        return StatusCode::UNAUTHORIZED;
    }

    if gnomcode_platform::stop_owned_process(state.child_pid).is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    state.stopping.store(true, Ordering::Relaxed);

    StatusCode::OK
}

async fn host_stream_handler(
    headers: HeaderMap,
    Query(query): Query<WsQuery>,
    State(state): State<Arc<HostState>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    if !valid_loopback_headers(&headers) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let token = query.token.or_else(|| {
        headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(ToOwned::to_owned)
    });

    if token.as_deref() != Some(&state.token) {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    let input_tx = state.input_tx.clone();
    let resize = state.resize.clone();
    let mut rx = state.output_tx.subscribe();
    let initial_bytes = state
        .scrollback
        .lock()
        .map(|sb| sb.bytes())
        .unwrap_or_default();

    ws.on_upgrade(move |mut socket: WebSocket| async move {
        if !initial_bytes.is_empty() {
            let _ = socket.send(Message::Binary(initial_bytes.into())).await;
        }

        loop {
            tokio::select! {
                Ok(bytes) = rx.recv() => {
                    if socket.send(Message::Binary(bytes.into())).await.is_err() {
                        break;
                    }
                }
                msg = socket.recv() => {
                    match msg {
                        Some(Ok(Message::Binary(b))) => {
                            let _ = input_tx.send(PtyInput {bytes: b.to_vec(), written: None});
                        }
                        Some(Ok(Message::Text(t))) => {
                            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&t)
                                && let Some(size) = value.get("resize") {
                                    if let Some((cols, rows)) = terminal_dimensions(size) {
                                        if let Ok(mut scrollback) = state.scrollback.lock() {
                                            scrollback.resize(rows, cols);
                                        }
                                        resize(cols, rows);
                                    }
                                    continue;
                                }
                            let _ = input_tx.send(PtyInput {bytes: t.as_bytes().to_vec(), written: None});
                        }
                        Some(Ok(Message::Close(_))) | None => break,
                        _ => {}
                    }
                }
            }
        }
    })
    .into_response()
}

fn terminal_dimensions(value: &serde_json::Value) -> Option<(u16, u16)> {
    let cols = u16::try_from(value.get("cols")?.as_u64()?).ok()?;
    let rows = u16::try_from(value.get("rows")?.as_u64()?).ok()?;
    ((1..=500).contains(&cols) && (1..=200).contains(&rows)).then_some((cols, rows))
}

/// Launches a new session worker in an independent detached process.
///
/// # Errors
/// Returns [`Error`] if validation fails or spawning times out.
pub async fn launch_session(
    session_id: &str,
    agent: &str,
    repo: &Path,
    model: Option<&str>,
) -> Result<SessionRecord, Error> {
    launch_session_with_resume(session_id, agent, repo, model, None).await
}

/// Reopens an exact saved native session in an independent hosted terminal.
///
/// # Errors
/// Returns an error if the agent cannot resume or the host cannot start.
pub async fn resume_session(
    session_id: &str,
    agent: &str,
    repo: &Path,
    native_id: &str,
) -> Result<SessionRecord, Error> {
    launch_session_with_resume(session_id, agent, repo, None, Some(native_id)).await
}

async fn launch_session_with_resume(
    session_id: &str,
    agent: &str,
    repo: &Path,
    model: Option<&str>,
    resume_id: Option<&str>,
) -> Result<SessionRecord, Error> {
    if !valid_session_id(session_id) {
        return Err(Error::SessionNotFound(session_id.to_owned()));
    }
    let _ = launch_command(agent, repo, model)?;
    if let Some(id) = resume_id {
        let _ = resume_command(agent, repo, id)?;
    }
    let exe = std::env::current_exe().map_err(Error::Io)?;

    let mut args = vec![
        "pty-host".to_owned(),
        session_id.to_owned(),
        agent.to_owned(),
        repo.display().to_string(),
    ];
    if let Some(m) = model {
        args.push(m.to_owned());
    }
    if let Some(id) = resume_id {
        args.extend(["--resume-session".to_owned(), id.to_owned()]);
    }

    #[cfg(windows)]
    {
        let mut cmd = std::process::Command::new(exe);
        cmd.args(&args);
        cmd.stdin(std::process::Stdio::null());
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());
        cmd.creation_flags(0x0800_0000 | 0x0000_0200);
        let _child = cmd.spawn().map_err(Error::Io)?;
    }
    #[cfg(unix)]
    {
        let mut cmd = std::process::Command::new(exe);
        cmd.args(&args);
        cmd.stdin(std::process::Stdio::null());
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());
        let _child = cmd.spawn().map_err(Error::Io)?;
    }

    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(150)).await;
        if let Ok(record) = load_session_record(session_id) {
            return Ok(record);
        }
    }

    Err(Error::Process(
        "Session host failed to start within timeout".to_owned(),
    ))
}

/// Stops an ongoing session by ID.
///
/// # Errors
/// Returns [`Error`] if the session cannot be contacted or stopped.
pub async fn stop_session(session_id: &str) -> Result<(), Error> {
    let record = load_session_record(session_id)?;
    send_stop(&record).await?;
    Ok(())
}

async fn send_stop(record: &SessionRecord) -> Result<(), Error> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .map_err(|e| Error::Process(e.to_string()))?;

    let url = format!("http://127.0.0.1:{}/stop", record.port);
    client
        .post(&url)
        .header("Authorization", format!("Bearer {}", record.token))
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|e| Error::Process(e.to_string()))?;
    Ok(())
}

/// Pastes an agent instruction then submits it after the TUI has processed the paste.
/// Shell sessions receive a normal command line.
///
/// # Errors
/// Returns [`Error`] if the host cannot acknowledge either input frame.
pub async fn send_prompt(record: &SessionRecord, text: &str) -> Result<(), Error> {
    if matches!(record.agent.as_str(), "terminal" | "shell") {
        return send_input(record, format!("{text}\r").as_bytes()).await;
    }
    send_input(record, format!("\x1b[200~{text}\x1b[201~").as_bytes()).await?;
    // A separate Enter avoids the CLI's burst-paste detector swallowing submission.
    tokio::time::sleep(Duration::from_millis(150)).await;
    if record.agent == "codex" {
        // Codex flushes Windows paste bursts on non-text input; End preserves the pasted text.
        send_input(record, b"\x1b[F").await?;
    }
    send_input(record, b"\r").await
}

/// Sends a bare Enter key only if the PTY's rendered screen proves `prompt` is still
/// sitting unsubmitted in the composer and no dialog is open (`Ok(false)` otherwise).
///
/// # Errors
/// Returns [`Error`] if the host cannot be reached or rejects authentication.
pub async fn retry_enter(record: &SessionRecord, prompt: &str) -> Result<bool, Error> {
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(3))
        .build()
        .map_err(|e| Error::Process(e.to_string()))?;
    let url = format!("http://127.0.0.1:{}/retry-enter", record.port);
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", record.token))
        .json(&serde_json::json!({ "prompt": prompt }))
        .send()
        .await
        .map_err(|e| Error::Process(e.to_string()))?;
    match response.status() {
        StatusCode::OK => Ok(true),
        StatusCode::CONFLICT => Ok(false),
        other => Err(Error::Process(format!("retry-enter status {other}"))),
    }
}

/// Changes the model through the official CLI without starting a new session.
/// Delivery is acknowledged by the host; model activation still requires observation.
///
/// # Errors
/// Returns an error for unsupported agents, unsafe model names, or failed delivery.
pub async fn switch_model(record: &SessionRecord, model: &str) -> Result<(), Error> {
    if model.is_empty()
        || model.starts_with('-')
        || model.len() > 128
        || !model
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.:/[]".contains(&byte))
    {
        return Err(Error::InvalidModel(model.to_owned()));
    }
    let command = match record.agent.as_str() {
        "codex" | "claude" | "claude-code" => format!("/model {model}"),
        "gemini" | "gemini-cli" => format!("/model set {model}"),
        _ => return Err(Error::UnsupportedAgent(record.agent.clone())),
    };
    send_input(record, command.as_bytes()).await?;
    tokio::time::sleep(Duration::from_millis(150)).await;
    send_input(record, b"\r").await
}

/// Queues terminal input through an authenticated endpoint independently of terminal output.
///
/// # Errors
/// Returns [`Error`] if the host cannot acknowledge the input.
pub async fn send_input(record: &SessionRecord, input: &[u8]) -> Result<(), Error> {
    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .map_err(|error| Error::Process(error.to_string()))?
        .post(format!("http://127.0.0.1:{}/input", record.port))
        .bearer_auth(&record.token)
        .body(input.to_vec())
        .send()
        .await
        .map_err(|error| Error::Process(error.to_string()))?;
    if response.status() == reqwest::StatusCode::CONFLICT {
        return Err(Error::NotReady);
    }
    response
        .error_for_status()
        .map_err(|error| Error::Process(error.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cold_codex_does_not_accept_or_queue_a_chat_prompt() {
        let (input_tx, input_rx) = std::sync::mpsc::channel();
        let (output_tx, _) = broadcast::channel(8);
        let state = Arc::new(HostState {
            protocol: false,
            agent: "codex".into(),
            token: "startup-test".into(),
            session_id: "startup-test".into(),
            child_pid: 0,
            scrollback: Arc::new(Mutex::new(Scrollback::new(1024))),
            output_tx,
            input_tx,
            stopping: Arc::new(AtomicBool::new(false)),
            resize: Arc::new(|_, _| {}),
        });
        let mut headers = HeaderMap::new();
        headers.insert("host", "127.0.0.1:41738".parse().expect("host"));
        headers.insert(
            "authorization",
            "Bearer startup-test".parse().expect("token"),
        );
        for bytes in [
            b"\x1b[200~complete prompt\x1b[201~".as_slice(),
            b"/model gpt-6-luna".as_slice(),
        ] {
            let status = tokio::time::timeout(
                Duration::from_millis(50),
                host_input_handler(
                    headers.clone(),
                    State(state.clone()),
                    axum::body::Bytes::copy_from_slice(bytes),
                ),
            )
            .await
            .expect("cold input must be rejected without waiting for a PTY write");
            assert_eq!(status, StatusCode::CONFLICT);
        }
        assert!(input_rx.try_recv().is_err());
    }

    #[test]
    fn antigravity_startup_requires_explicit_terminal_trust_before_chat_input() {
        let waiting=b"\x1b[1mDo you trust the contents of this project?\x1b[0m\r\n> Yes, I trust this folder\r\n  No, exit";
        assert!(startup_needs_setup("antigravity", waiting));
        assert!(startup_needs_setup(
            "antigravity",
            b"Do you trust the contents of\r\nthis project?"
        ));
        let mut ready = waiting.to_vec();
        ready.extend_from_slice(b"\x1b[2J\x1b[H>\r\n? for shortcuts");
        assert!(
            !startup_needs_setup("antigravity", &ready),
            "an old trust prompt in scrollback must not block a ready CLI"
        );
        ready.extend_from_slice(waiting);
        assert!(
            startup_needs_setup("antigravity", &ready),
            "a new workspace trust prompt requires desktop confirmation"
        );
    }

    #[tokio::test]
    async fn chat_input_cannot_confirm_antigravity_workspace_trust() {
        let (input_tx, input_rx) = std::sync::mpsc::channel();
        let (output_tx, _) = broadcast::channel(8);
        let mut scrollback = Scrollback::new(4096);
        scrollback
            .append(b"Do you trust the contents of this project?\r\n> Yes, I trust this folder");
        let state = Arc::new(HostState {
            protocol: false,
            agent: "antigravity".into(),
            token: "setup-test".into(),
            session_id: "setup-test".into(),
            child_pid: 0,
            scrollback: Arc::new(Mutex::new(scrollback)),
            output_tx,
            input_tx,
            stopping: Arc::new(AtomicBool::new(false)),
            resize: Arc::new(|_, _| {}),
        });
        let mut headers = HeaderMap::new();
        headers.insert("host", "127.0.0.1:41738".parse().expect("host"));
        headers.insert("authorization", "Bearer setup-test".parse().expect("token"));
        assert_eq!(
            host_input_handler(
                headers,
                State(state),
                axum::body::Bytes::from_static(b"\x1b[200~heyo\x1b[201~")
            )
            .await,
            StatusCode::CONFLICT
        );
        assert!(
            input_rx.try_recv().is_err(),
            "a chat message must not type into a trust dialog"
        );
    }

    #[test]
    fn codex_hook_review_stops_blocking_after_prompt_returns() {
        let blocked = b"? for shortcuts\r\n2 hooks are new or changed.\r\nReview hooks";
        assert!(startup_needs_setup("codex", blocked));
        let mut ready = blocked.to_vec();
        ready.extend_from_slice(b"\x1b[2J\r\n? for shortcuts");
        assert!(!startup_needs_setup("codex", &ready));
    }

    const CLAUDE_TRUST: &[u8] =
        include_bytes!("../../../fixtures/claude-code/v2.1.322/trust-dialog.pty");
    const CLAUDE_READY: &[u8] =
        include_bytes!("../../../fixtures/claude-code/v2.1.322/ready-composer.pty");
    const GEMINI_TRUST: &[u8] = b"\x1b[2J\x1b[H\x1b[1mDo\x1b[1Cyou\x1b[1Ctrust\x1b[1Cthe\x1b[1Cfiles\x1b[1Cin\x1b[1Cthis\x1b[1Cfolder?\x1b[0m\r\n1.\x1b[1CTrust\x1b[1Cfolder\r\n2. Trust parent folder\r\n3. Don't trust";
    const GEMINI_READY: &[u8] = b"\x1b[2J\x1b[H\x1b]0;Gemini CLI\x07\x1b[38;5;244m>\x1b[0m\x1b[1C\x1b[7m \x1b[0m\x1b[2mType\x1b[1Cyour\x1b[1Cmessage\x1b[1Cor\x1b[1C@path/to/file\x1b[0m";

    #[test]
    fn claude_trust_dialog_holds_chat_until_the_composer_returns() {
        assert!(startup_needs_setup("claude-code", CLAUDE_TRUST));
        assert!(startup_needs_setup("claude", b""), "no composer yet");
        assert!(!startup_needs_setup("claude-code", CLAUDE_READY));
        let mut accepted = CLAUDE_TRUST.to_vec();
        accepted.extend_from_slice(b"\x1b[?1049h\x1b[2J");
        accepted.extend_from_slice(CLAUDE_READY);
        assert!(!startup_needs_setup("claude-code", &accepted));
        let mut again = CLAUDE_READY.to_vec();
        again.extend_from_slice(CLAUDE_TRUST);
        assert!(startup_needs_setup("claude-code", &again));
    }

    #[test]
    fn claude_stale_dialog_text_above_a_ready_prompt_does_not_block() {
        let mut ring = CLAUDE_TRUST.to_vec();
        for line in 0..200 {
            ring.extend_from_slice(format!("\r\nwork line {line}").as_bytes());
        }
        ring.extend_from_slice(b"\r\n? for shortcuts");
        assert!(!startup_needs_setup("claude", &ring));
        ring.extend_from_slice(b"\r\nOK\x1b[1Cto\x1b[1Cproceed? Enter to confirm");
        assert!(
            startup_needs_setup("claude", &ring),
            "newer choice footer wins"
        );
    }

    #[test]
    fn claude_login_and_bypass_screens_hold_chat() {
        for screen in [
            &b"Select login method:\r\n> 1. Claude account"[..],
            b"Bypass Permissions mode\r\n1. No, exit\r\n2. Yes, I accept",
            b"Choose the text style that looks best",
        ] {
            assert!(startup_needs_setup("claude", screen));
        }
    }

    #[test]
    fn a_wrapped_ring_without_markers_is_not_treated_as_startup() {
        let ring = vec![b'x'; SCROLLBACK_BYTES];
        assert!(!startup_needs_setup("claude", &ring));
        assert!(!startup_needs_setup("gemini", &ring));
    }

    #[test]
    fn gemini_trust_and_auth_screens_hold_chat_until_the_prompt_returns() {
        assert!(startup_needs_setup("gemini-cli", GEMINI_TRUST));
        assert!(startup_needs_setup(
            "gemini",
            b"Get started\r\nHow would you like to authenticate for this project?"
        ));
        assert!(startup_needs_setup("gemini", b""));
        assert!(!startup_needs_setup("gemini", GEMINI_READY));
        let mut accepted = GEMINI_TRUST.to_vec();
        accepted.extend_from_slice(GEMINI_READY);
        assert!(!startup_needs_setup("gemini", &accepted));
        let mut again = GEMINI_READY.to_vec();
        again.extend_from_slice(GEMINI_TRUST);
        assert!(startup_needs_setup("gemini", &again));
    }

    #[test]
    fn codex_stale_shortcuts_text_does_not_count_as_ready() {
        let mut ring = b"? for shortcuts".to_vec();
        ring.extend_from_slice(b"\x1b[2J\x1b[Hloading");
        assert!(!codex_prompt_ready(&ring));
        ring.extend_from_slice(b"\x1b[2J\x1b[H> \r\n? for shortcuts");
        assert!(codex_prompt_ready(&ring));
    }

    #[tokio::test]
    async fn chat_input_cannot_accept_claude_or_gemini_trust() {
        for (agent, screen) in [("claude-code", CLAUDE_TRUST), ("gemini-cli", GEMINI_TRUST)] {
            let (input_tx, input_rx) = std::sync::mpsc::channel();
            let (output_tx, _) = broadcast::channel(8);
            let mut scrollback = Scrollback::new(SCROLLBACK_BYTES);
            scrollback.append(screen);
            let state = Arc::new(HostState {
                protocol: false,
                agent: agent.into(),
                token: "setup-test".into(),
                session_id: "setup-test".into(),
                child_pid: 0,
                scrollback: Arc::new(Mutex::new(scrollback)),
                output_tx,
                input_tx,
                stopping: Arc::new(AtomicBool::new(false)),
                resize: Arc::new(|_, _| {}),
            });
            let mut headers = HeaderMap::new();
            headers.insert("host", "127.0.0.1:41738".parse().expect("host"));
            headers.insert("authorization", "Bearer setup-test".parse().expect("token"));
            assert_eq!(
                host_input_handler(
                    headers,
                    State(state),
                    axum::body::Bytes::from_static(b"\x1b[200~hello\x1b[201~")
                )
                .await,
                StatusCode::CONFLICT,
                "{agent}"
            );
            assert!(input_rx.try_recv().is_err(), "{agent} must receive nothing");
        }
    }

    #[tokio::test]
    async fn chat_input_cannot_operate_codex_hook_review() {
        let (input_tx, input_rx) = std::sync::mpsc::channel();
        let (output_tx, _) = broadcast::channel(8);
        let mut scrollback = Scrollback::new(4096);
        scrollback.append(b"? for shortcuts\r\nHooks need review\r\n2 hooks are new or changed.\r\n1. Review hooks\r\n2. Trust all and continue\r\n3. Continue without trusting");
        let state = Arc::new(HostState {
            protocol: false,
            agent: "codex".into(),
            token: "setup-test".into(),
            session_id: "setup-test".into(),
            child_pid: 0,
            scrollback: Arc::new(Mutex::new(scrollback)),
            output_tx,
            input_tx,
            stopping: Arc::new(AtomicBool::new(false)),
            resize: Arc::new(|_, _| {}),
        });
        let mut headers = HeaderMap::new();
        headers.insert("host", "127.0.0.1:41738".parse().expect("host"));
        headers.insert("authorization", "Bearer setup-test".parse().expect("token"));
        assert_eq!(
            host_input_handler(
                headers,
                State(state),
                axum::body::Bytes::from_static(b"\x1b[200~hello\x1b[201~")
            )
            .await,
            StatusCode::CONFLICT
        );
        assert!(
            input_rx.try_recv().is_err(),
            "chat must not interact with hook approval"
        );
    }

    #[test]
    fn resume_uses_the_exact_native_session_without_permission_bypass() {
        let repo = tempfile::tempdir().expect("workspace");
        assert_eq!(
            resume_command("codex", repo.path(), "saved-123")
                .expect("codex")
                .arguments,
            ["resume", "saved-123"]
        );
        assert_eq!(
            resume_command("claude-code", repo.path(), "saved-123")
                .expect("claude")
                .arguments,
            ["--resume", "saved-123"]
        );
        assert_eq!(
            resume_command("gemini-cli", repo.path(), "saved-123")
                .expect("gemini")
                .arguments,
            ["--resume", "saved-123"]
        );
        for agent in ["antigravity", "agy"] {
            assert_eq!(
                resume_command(agent, repo.path(), "saved-123").expect("agy native resume"),
                LaunchCommand {
                    executable: "agy".into(),
                    arguments: vec![
                        "--conversation".into(),
                        "saved-123".into(),
                        "--mode".into(),
                        "plan".into()
                    ]
                }
            );
            for id in [
                "",
                "--last",
                "--yolo",
                "saved\r/exit",
                "saved another",
                "../saved",
            ] {
                assert!(resume_command(agent, repo.path(), id).is_err());
            }
        }
        for id in ["", "--last", "--yolo", "saved\r/exit", "saved another"] {
            assert!(resume_command("codex", repo.path(), id).is_err());
        }
        assert!(resume_command("terminal", repo.path(), "saved").is_err());
    }

    #[tokio::test]
    async fn prompt_input_is_acknowledged_without_opening_an_output_stream() {
        let (input_tx, input_rx) = std::sync::mpsc::channel();
        let (output_tx, _) = broadcast::channel(8);
        let state = Arc::new(HostState {
            protocol: false,
            agent: "codex".into(),
            token: "input-test".into(),
            session_id: "input-test".into(),
            child_pid: 0,
            scrollback: Arc::new(Mutex::new(Scrollback::new(16))),
            output_tx,
            input_tx,
            stopping: Arc::new(AtomicBool::new(false)),
            resize: Arc::new(|_, _| {}),
        });
        state
            .scrollback
            .lock()
            .expect("scrollback")
            .append(b"? for shortcuts");
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let port = listener.local_addr().expect("address").port();
        let app = Router::new()
            .route("/input", post(host_input_handler))
            .with_state(state);
        let server = tokio::spawn(async move { axum::serve(listener, app).await.expect("server") });
        let record = SessionRecord {
            session_id: "input-test".into(),
            agent: "codex".into(),
            repo_path: ".".into(),
            model: None,
            host_pid: 0,
            child_pid: 0,
            port,
            token: "input-test".into(),
            started_at: String::new(),
        };
        let pending_record = record.clone();
        let pending = tokio::spawn(async move { send_input(&pending_record, b"hello\r").await });
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(
            !pending.is_finished(),
            "input acknowledgment must wait for the PTY write, not just queueing"
        );
        let input = input_rx.try_recv().expect("queued input");
        assert_eq!(input.bytes, b"hello\r");
        input
            .written
            .expect("acknowledgment")
            .send(true)
            .expect("written");
        pending
            .await
            .expect("request task")
            .expect("acknowledged input");
        let queued_rx = input_rx;
        let (observed_tx, input_rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            while let Ok(input) = queued_rx.recv() {
                observed_tx.send(input.bytes).expect("observed input");
                if let Some(written) = input.written {
                    let _ = written.send(true);
                }
            }
        });
        send_prompt(&record, "one line")
            .await
            .expect("agent prompt");
        assert_eq!(
            input_rx.try_recv().expect("paste frame"),
            b"\x1b[200~one line\x1b[201~"
        );
        assert_eq!(input_rx.try_recv().expect("end paste burst"), b"\x1b[F");
        assert_eq!(input_rx.try_recv().expect("submit frame"), b"\r");
        switch_model(&record, "gpt-5.6-luna")
            .await
            .expect("native switch");
        assert_eq!(
            input_rx.try_recv().expect("model command"),
            b"/model gpt-5.6-luna"
        );
        assert_eq!(input_rx.try_recv().expect("model submit"), b"\r");
        let mut claude = record.clone();
        claude.agent = "claude-code".into();
        switch_model(&claude, "opus[1m]")
            .await
            .expect("native extended-context alias");
        assert_eq!(
            input_rx.try_recv().expect("alias command"),
            b"/model opus[1m]"
        );
        assert_eq!(input_rx.try_recv().expect("alias submit"), b"\r");
        for invalid_model in ["gpt-test\r/exit", "gpt-test --yolo", "", "--yolo"] {
            assert!(switch_model(&record, invalid_model).await.is_err());
            assert!(input_rx.try_recv().is_err());
        }
        let mut shell = record.clone();
        shell.agent = "terminal".into();
        send_prompt(&shell, "echo test")
            .await
            .expect("shell command");
        assert_eq!(input_rx.try_recv().expect("shell input"), b"echo test\r");
        let mut invalid = record.clone();
        invalid.token = "wrong".into();
        assert!(send_input(&invalid, b"forbidden").await.is_err());
        assert!(input_rx.try_recv().is_err());
        let denied = reqwest::Client::new()
            .post(format!("http://127.0.0.1:{port}/input"))
            .bearer_auth("input-test")
            .header("Origin", "https://untrusted.example")
            .body("forbidden")
            .send()
            .await
            .expect("response");
        assert_eq!(denied.status(), StatusCode::FORBIDDEN);
        assert!(input_rx.try_recv().is_err());
        server.abort();
    }

    #[test]
    fn terminal_resize_rejects_invalid_dimensions() {
        assert_eq!(
            super::terminal_dimensions(&serde_json::json!({"cols":80,"rows":24})),
            Some((80, 24))
        );
        for value in [
            serde_json::json!({"cols":0,"rows":24}),
            serde_json::json!({"cols":1000,"rows":24}),
            serde_json::json!({"cols":80,"rows":-1}),
        ] {
            assert_eq!(super::terminal_dimensions(&value), None);
        }
    }
    #[test]
    fn owner_rejects_non_loopback_origin_and_host() {
        let mut headers = HeaderMap::new();
        headers.insert("host", "evil.example".parse().expect("host"));
        assert!(!valid_loopback_headers(&headers));
        headers.insert("host", "127.0.0.1:1234".parse().expect("host"));
        headers.insert("origin", "https://evil.example".parse().expect("origin"));
        assert!(!valid_loopback_headers(&headers));
        headers.insert("origin", "http://localhost:1234".parse().expect("origin"));
        assert!(valid_loopback_headers(&headers));
    }

    #[test]
    fn session_record_rejects_path_separators() {
        let record = SessionRecord {
            session_id: "../escape".to_owned(),
            agent: "codex".to_owned(),
            repo_path: ".".to_owned(),
            model: None,
            host_pid: 0,
            child_pid: 0,
            port: 0,
            token: String::new(),
            started_at: String::new(),
        };
        assert!(save_session_record(&record).is_err());
        for id in [
            "",
            "..",
            "../escape",
            "..\\escape",
            "C:\\escape",
            "/absolute",
        ] {
            assert!(!valid_session_id(id), "accepted {id:?}");
            assert!(load_session_record(id).is_err());
        }
        for id in ["codex-123", "claude-code-123", "test_session"] {
            assert!(valid_session_id(id), "rejected {id:?}");
        }
    }

    #[tokio::test]
    async fn stop_rejects_unreachable_host() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let port = listener.local_addr().expect("address").port();
        drop(listener);
        let record = SessionRecord {
            session_id: "test".to_owned(),
            agent: "codex".to_owned(),
            repo_path: ".".to_owned(),
            model: None,
            host_pid: 0,
            child_pid: 0,
            port,
            token: "test".to_owned(),
            started_at: String::new(),
        };
        assert!(send_stop(&record).await.is_err());
    }

    #[tokio::test]
    async fn stop_rejects_owner_error_response() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let port = listener.local_addr().expect("address").port();
        let app = Router::new().route("/stop", post(|| async { StatusCode::UNAUTHORIZED }));
        let server = tokio::spawn(async move { axum::serve(listener, app).await.expect("serve") });
        let record = SessionRecord {
            session_id: "test".to_owned(),
            agent: "codex".to_owned(),
            repo_path: ".".to_owned(),
            model: None,
            host_pid: 0,
            child_pid: 0,
            port,
            token: "wrong".to_owned(),
            started_at: String::new(),
        };
        assert!(send_stop(&record).await.is_err());
        server.abort();
    }

    #[test]
    fn constructs_official_commands_without_permission_bypass() {
        let repo = tempfile::tempdir().unwrap();
        for (agent, executable) in [
            ("claude", "claude"),
            ("claude-code", "claude"),
            ("codex", "codex"),
            ("antigravity", "agy"),
            ("gemini", "gemini"),
            ("gemini-cli", "gemini"),
        ] {
            let command = launch_command(agent, repo.path(), Some("example-model")).unwrap();
            assert_eq!(command.executable, executable);
            assert_eq!(command.arguments, ["--model", "example-model"]);
        }
    }

    #[test]
    fn rejects_unknown_agent_and_nonexistent_workspace() {
        let repo = tempfile::tempdir().unwrap();
        assert!(launch_command("unknown", repo.path(), None).is_err());
        assert!(launch_command("claude", &repo.path().join("missing"), None).is_err());
    }

    #[test]
    fn rejects_model_values_that_could_become_options() {
        let repo = tempfile::tempdir().unwrap();
        for model in ["", "--dangerously-skip-permissions", "-yolo", "--verbose"] {
            assert!(launch_command("claude", repo.path(), Some(model)).is_err());
        }
    }

    #[test]
    fn scrollback_retains_most_recent_bytes_within_limit() {
        let mut buffer = Scrollback::new(5);
        buffer.append(b"abc");
        buffer.append(b"defg");
        assert_eq!(buffer.bytes(), b"cdefg");
        buffer.append(b"123456");
        assert_eq!(buffer.bytes(), b"23456");
    }

    #[tokio::test]
    async fn independent_session_host_serves_health_and_stops_cleanly() {
        let dir = tempfile::tempdir().unwrap();
        let session_id = format!("test-session-{}", std::process::id());

        #[cfg(windows)]
        let override_cmd = Some(("cmd.exe", &["/c", "ping -n 30 127.0.0.1 >nul"][..]));
        #[cfg(not(windows))]
        let override_cmd = Some(("sh", &["-c", "sleep 30"][..]));

        let s_id = session_id.clone();
        let repo = dir.path().to_path_buf();
        let host_handle = tokio::spawn(async move {
            let _ = run_pty_host(&s_id, "claude", &repo, None, override_cmd).await;
        });

        // Wait for session record to be written
        let mut record = None;
        for _ in 0..40 {
            tokio::time::sleep(Duration::from_millis(100)).await;
            if let Ok(r) = load_session_record(&session_id) {
                record = Some(r);
                break;
            }
        }
        let record = record.expect("session record should be created");
        assert_eq!(record.session_id, session_id);
        assert!(record.child_pid > 0);

        let health_url = format!("http://127.0.0.1:{}/health", record.port);
        let client = reqwest::Client::new();
        assert_eq!(
            client
                .get(&health_url)
                .send()
                .await
                .expect("health")
                .status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            client
                .get(&health_url)
                .bearer_auth(&record.token)
                .send()
                .await
                .expect("health")
                .status(),
            StatusCode::OK
        );

        // Verify child process is alive (fail-open invariant)
        assert!(is_pid_alive(record.child_pid));

        // Call stop endpoint
        stop_session(&session_id).await.expect("stop session");

        // Wait for host task to finish
        let _ = host_handle.await;

        // Verify session record is cleaned up
        assert!(load_session_record(&session_id).is_err());
    }

    #[test]
    fn vt100_screen_overwrites_stale_footer_in_place_without_clear() {
        let mut frame = b"\x1b[1;1H> \r\n\x1b[2;1H? for shortcuts".to_vec();
        assert!(codex_prompt_ready(&frame));
        // Overwrite row 2 in place without \x1b[2J
        frame.extend_from_slice(b"\x1b[2;1H\x1b[2Kloading");
        assert!(!codex_prompt_ready(&frame));
    }

    #[test]
    fn composer_has_unsubmitted_prompt_requires_prompt_in_composer_without_dialog() {
        let mut ready_with_prompt = CLAUDE_READY.to_vec();
        ready_with_prompt.extend_from_slice(b" fix the build");
        assert!(composer_has_unsubmitted_prompt(
            "claude-code",
            &ready_with_prompt,
            "fix the build"
        ));
        assert!(!composer_has_unsubmitted_prompt(
            "claude-code",
            CLAUDE_READY,
            "fix the build"
        ));
        let mut dialog_with_prompt = CLAUDE_TRUST.to_vec();
        dialog_with_prompt.extend_from_slice(b"\r\n> fix the build");
        assert!(!composer_has_unsubmitted_prompt(
            "claude-code",
            &dialog_with_prompt,
            "fix the build"
        ));
    }

    #[tokio::test]
    async fn retry_enter_handler_rejects_unless_composer_holds_prompt() {
        let (input_tx, input_rx) = std::sync::mpsc::channel();
        let (output_tx, _) = broadcast::channel(8);
        let mut scrollback = Scrollback::new(SCROLLBACK_BYTES);
        scrollback.append(CLAUDE_READY);
        let state = Arc::new(HostState {
            protocol: false,
            agent: "claude-code".into(),
            token: "retry-test".into(),
            session_id: "retry-test".into(),
            child_pid: 0,
            scrollback: Arc::new(Mutex::new(scrollback)),
            output_tx,
            input_tx,
            stopping: Arc::new(AtomicBool::new(false)),
            resize: Arc::new(|_, _| {}),
        });
        let mut headers = HeaderMap::new();
        headers.insert("host", "127.0.0.1:41738".parse().expect("host"));
        headers.insert("authorization", "Bearer retry-test".parse().expect("token"));

        assert_eq!(
            host_retry_enter_handler(
                headers.clone(),
                State(Arc::clone(&state)),
                Json(RetryEnterRequest {
                    prompt: "fix the build".into()
                })
            )
            .await,
            StatusCode::CONFLICT
        );
        assert!(input_rx.try_recv().is_err());

        state
            .scrollback
            .lock()
            .expect("lock")
            .append(b" fix the build");
        let call = tokio::spawn({
            let state = Arc::clone(&state);
            let headers = headers.clone();
            async move {
                host_retry_enter_handler(
                    headers,
                    State(state),
                    Json(RetryEnterRequest {
                        prompt: "fix the build".into(),
                    }),
                )
                .await
            }
        });
        let sent = tokio::task::spawn_blocking(move || input_rx.recv())
            .await
            .expect("join")
            .expect("recv");
        assert_eq!(sent.bytes, b"\r");
        if let Some(ack) = sent.written {
            let _ = ack.send(true);
        }
        assert_eq!(call.await.expect("handler"), StatusCode::OK);
    }
}
