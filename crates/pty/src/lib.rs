//! Agent launch validation, independent session hosting, and terminal sessions.

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

/// Error type for PTY operations.
#[derive(Debug, Error)]
pub enum Error {
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
                let (executable, arguments) = spawnt_platform::executables::terminal_shell();
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
        _ => return Err(Error::UnsupportedAgent(agent.to_owned())),
    };
    command.arguments = vec![verb.to_owned(), native_id.to_owned()];
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
    spawnt_platform::spawnt_home().join("sessions")
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
    spawnt_platform::write_private_file(&path, &bytes)
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
        if path.extension().is_some_and(|ext| ext == "json") {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(record) = serde_json::from_str::<SessionRecord>(&content) {
                    if is_pid_alive(record.host_pid) {
                        sessions.push(record);
                    } else {
                        let _ = fs::remove_file(&path);
                    }
                }
            }
        }
    }
    sessions
}

/// Checks whether a process ID is currently alive.
#[must_use]
pub fn is_pid_alive(pid: u32) -> bool {
    spawnt_platform::process_is_alive(pid)
}

/// Bounded in-memory terminal output buffer.
#[derive(Debug, Clone)]
pub struct Scrollback {
    capacity: usize,
    bytes: VecDeque<u8>,
}

impl Scrollback {
    /// Creates a scrollback buffer with a given byte limit.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            bytes: VecDeque::new(),
        }
    }

    /// Appends output to the buffer, dropping older bytes when full.
    pub fn append(&mut self, data: &[u8]) {
        let keep = data.len().min(self.capacity);
        let remove = self
            .bytes
            .len()
            .saturating_add(keep)
            .saturating_sub(self.capacity);
        self.bytes.drain(..remove);
        self.bytes.extend(&data[data.len() - keep..]);
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
    token: String,
    session_id: String,
    child_pid: u32,
    scrollback: Arc<Mutex<Scrollback>>,
    output_tx: broadcast::Sender<Vec<u8>>,
    input_tx: std::sync::mpsc::Sender<Vec<u8>>,
    stopping: Arc<AtomicBool>,
    resize: Arc<dyn Fn(u16, u16) + Send + Sync>,
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

    let scrollback = Arc::new(Mutex::new(Scrollback::new(256 * 1024)));
    let (output_tx, _) = broadcast::channel(128);
    let (input_tx, input_rx) = std::sync::mpsc::channel::<Vec<u8>>();
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
                spawnt_platform::executables::agent_command(&launch.executable);
            arguments.extend(launch.arguments);
            (executable, arguments)
        };

        let mut command = conpty_oxide::blocking::Command::new(cmd);
        command.args(&args);
        command.current_dir(repo);
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
        while let Ok(data) = input_rx.recv() {
            if writer.write_all(&data).is_err() || writer.flush().is_err() {
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
        token,
        session_id: session_id.to_owned(),
        child_pid,
        scrollback,
        output_tx,
        input_tx,
        stopping: Arc::clone(&stopping),
        resize,
    });

    let app = Router::new()
        .route("/health", get(host_health_handler))
        .route("/stream", get(host_stream_handler))
        .route("/input", post(host_input_handler))
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
    if state.input_tx.send(body.to_vec()).is_err() {
        return StatusCode::GONE;
    }
    StatusCode::OK
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

    if spawnt_platform::stop_owned_process(state.child_pid).is_err() {
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
                            let _ = input_tx.send(b.to_vec());
                        }
                        Some(Ok(Message::Text(t))) => {
                            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&t) {
                                if let Some(size) = value.get("resize") {
                                    if let Some((cols,rows)) = terminal_dimensions(size) {resize(cols,rows);}
                                    continue;
                                }
                            }
                            let _ = input_tx.send(t.as_bytes().to_vec());
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
    send_input(record, b"\r").await
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
    reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .map_err(|error| Error::Process(error.to_string()))?
        .post(format!("http://127.0.0.1:{}/input", record.port))
        .bearer_auth(&record.token)
        .body(input.to_vec())
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|error| Error::Process(error.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resume_uses_the_exact_native_session_without_permission_flags() {
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
            token: "input-test".into(),
            session_id: "input-test".into(),
            child_pid: 0,
            scrollback: Arc::new(Mutex::new(Scrollback::new(16))),
            output_tx,
            input_tx,
            stopping: Arc::new(AtomicBool::new(false)),
            resize: Arc::new(|_, _| {}),
        });
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
        send_input(&record, b"hello\r")
            .await
            .expect("acknowledged input");
        assert_eq!(input_rx.try_recv().expect("queued input"), b"hello\r");
        send_prompt(&record, "one line")
            .await
            .expect("agent prompt");
        assert_eq!(
            input_rx.try_recv().expect("paste frame"),
            b"\x1b[200~one line\x1b[201~"
        );
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
}
