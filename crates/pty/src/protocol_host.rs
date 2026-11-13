//! Opt-in protocol-mode session hosts for Codex `app-server` and Antigravity `agy acp` (ADR 0057).
//!
//! The host exposes the same authenticated loopback API as a PTY host (`/stream`, `/input`,
//! `/stop`), but drives the official CLI's JSON-RPC protocol instead of a terminal. Approval
//! requests become ADR 0032 approvals answered from the console or a paired phone. Nothing is
//! ever auto-approved: no decision, a timeout or a missing daemon answers cancel.

use crate::acp_approvals::{self, AcpApprovalOutcome};
use crate::codex_app_server::{self, ApprovalOutcome};
use crate::{Error, HostParts, PtyInput, Scrollback, launch_command, serve_host, valid_session_id};
use serde_json::{Value, json};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::{broadcast, mpsc};

/// Resolves the running daemon's loopback `(port, bearer token)`, or `None` when it is down.
pub type DaemonEndpoint = Arc<dyn Fn() -> Option<(u16, String)> + Send + Sync>;

const SETTINGS_FILE: &str = "protocol-approvals.json";
/// Longest prompt the terminal line editor accepts.
const MAX_PROMPT_BYTES: usize = 64 * 1024;

fn settings_path() -> PathBuf {
    gnomcode_platform::gnomcode_home().join(SETTINGS_FILE)
}

/// The settings key for an agent that supports protocol mode.
fn setting_key(agent: &str) -> Option<&'static str> {
    match agent {
        "codex" => Some("codex"),
        "antigravity" | "agy" => Some("antigravity"),
        _ => None,
    }
}

fn enabled_in(path: &Path, key: &str) -> bool {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .is_some_and(|value| value.get(key) == Some(&Value::Bool(true)))
}

fn set_in(path: &Path, key: &str, on: bool) -> std::io::Result<()> {
    let mut value = std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}));
    value[key] = Value::Bool(on);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, value.to_string())
}

/// Whether the user opted `agent` in to protocol-mode launches (`protocol_approvals`, default off).
#[must_use]
pub fn protocol_approvals_enabled(agent: &str) -> bool {
    setting_key(agent).is_some_and(|key| enabled_in(&settings_path(), key))
}

/// Records the per-agent `protocol_approvals` opt-in.
///
/// # Errors
/// Returns [`Error::UnsupportedAgent`] for an agent without protocol mode, or an I/O error.
pub fn set_protocol_approvals(agent: &str, on: bool) -> Result<(), Error> {
    let key = setting_key(agent).ok_or_else(|| Error::UnsupportedAgent(agent.to_owned()))?;
    set_in(&settings_path(), key, on)?;
    Ok(())
}

/// Which protocol a host speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Codex,
    Acp,
}

/// Terminal output: in-memory scrollback plus live subscribers. Plain text, CRLF line ends.
#[derive(Clone)]
struct Term {
    scrollback: Arc<Mutex<Scrollback>>,
    tx: broadcast::Sender<Vec<u8>>,
}

impl Term {
    fn emit(&self, text: &str) {
        let bytes = text
            .replace("\r\n", "\n")
            .replace('\n', "\r\n")
            .into_bytes();
        if let Ok(mut scrollback) = self.scrollback.lock() {
            scrollback.append(&bytes);
        }
        let _ = self.tx.send(bytes);
    }
}

/// Minimal line editor over raw terminal bytes: typed text echoes and submits on Enter; a
/// bracketed paste (how prompts are delivered) is buffered and submitted by the Enter after it.
#[derive(Default)]
struct Composer {
    buf: Vec<u8>,
    paste: bool,
    escape: Option<Vec<u8>>,
}

impl Composer {
    /// Consumes `bytes`, appends echo to `echo` and returns the prompts completed by them.
    fn feed(&mut self, bytes: &[u8], echo: &mut Vec<u8>) -> Vec<String> {
        let mut submitted = Vec::new();
        for &byte in bytes {
            if let Some(sequence) = self.escape.as_mut() {
                sequence.push(byte);
                let finished = if sequence.len() == 2 {
                    sequence[1] != b'['
                } else {
                    (0x40..=0x7e).contains(&byte)
                };
                if finished {
                    let sequence = self.escape.take().unwrap_or_default();
                    match sequence.as_slice() {
                        b"\x1b[200~" => self.paste = true,
                        b"\x1b[201~" => self.paste = false,
                        _ => {}
                    }
                }
                continue;
            }
            match byte {
                0x1b => self.escape = Some(vec![byte]),
                b'\r' | b'\n' if self.paste => {
                    if byte == b'\n' {
                        self.buf.push(b'\n');
                    }
                }
                b'\r' | b'\n' => {
                    let text = String::from_utf8_lossy(&self.buf).trim().to_owned();
                    self.buf.clear();
                    echo.extend_from_slice(b"\r\n");
                    if !text.is_empty() {
                        submitted.push(text);
                    }
                }
                0x7f | 0x08 if !self.paste => {
                    while self.buf.pop().is_some_and(|last| last & 0xC0 == 0x80) {}
                    echo.extend_from_slice(b"\x08 \x08");
                }
                _ if (byte >= 0x20 || byte == b'\t') && self.buf.len() < MAX_PROMPT_BYTES => {
                    self.buf.push(byte);
                    if !self.paste {
                        echo.push(byte);
                    }
                }
                _ => {}
            }
        }
        submitted
    }
}

fn kind_for(agent: &str) -> Result<Kind, Error> {
    match agent {
        "codex" => Ok(Kind::Codex),
        "antigravity" | "agy" => Ok(Kind::Acp),
        other => Err(Error::UnsupportedAgent(other.to_owned())),
    }
}

/// Runs a protocol-mode host for a new Codex or Antigravity session.
///
/// # Errors
/// Returns an error if validation, process creation or host startup fails.
pub async fn run_protocol_host(
    session_id: &str,
    agent: &str,
    repo: &Path,
    model: Option<&str>,
    daemon: DaemonEndpoint,
) -> Result<(), Error> {
    if !valid_session_id(session_id) {
        return Err(Error::SessionNotFound(session_id.to_owned()));
    }
    let kind = kind_for(agent)?;
    let launch = launch_command(agent, repo, model)?;
    let (name, subcommand) = match kind {
        Kind::Codex => ("codex", "app-server"),
        Kind::Acp => ("agy", "acp"),
    };
    debug_assert_eq!(launch.executable, name);
    let mut command = tokio::process::Command::from(
        gnomcode_platform::executables::background_agent_command(name),
    );
    command
        .arg(subcommand)
        .current_dir(repo)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    let mut child = command
        .spawn()
        .map_err(|error| Error::Process(error.to_string()))?;
    let child_pid = child.id().unwrap_or(0);
    let (Some(stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
        return Err(Error::Process(
            "agent protocol pipes unavailable".to_owned(),
        ));
    };

    let token = {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).map_err(|error| Error::Process(error.to_string()))?;
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
    };
    let scrollback = Arc::new(Mutex::new(Scrollback::new(crate::SCROLLBACK_BYTES)));
    let (output_tx, _) = broadcast::channel(128);
    let term = Term {
        scrollback: Arc::clone(&scrollback),
        tx: output_tx.clone(),
    };
    let (input_tx, input_rx) = std::sync::mpsc::channel::<PtyInput>();
    let (prompt_tx, prompt_rx) = mpsc::unbounded_channel::<String>();
    let echo_term = term.clone();
    std::thread::spawn(move || {
        let mut composer = Composer::default();
        while let Ok(input) = input_rx.recv() {
            if let Some(acknowledge) = input.written {
                let _ = acknowledge.send(true);
            }
            let mut echo = Vec::new();
            let prompts = composer.feed(&input.bytes, &mut echo);
            if !echo.is_empty() {
                echo_term.emit(&String::from_utf8_lossy(&echo));
            }
            for prompt in prompts {
                if prompt_tx.send(prompt).is_err() {
                    return;
                }
            }
        }
    });

    let ctx = Context {
        kind,
        session_id: session_id.to_owned(),
        agent: if kind == Kind::Codex {
            "codex"
        } else {
            "antigravity"
        },
        cwd: repo.display().to_string(),
        model: model.map(ToOwned::to_owned),
        daemon,
    };
    let driver_term = term.clone();
    let stopping = Arc::new(AtomicBool::new(false));
    let driver = tokio::spawn(async move {
        if let Err(error) = drive(
            ctx,
            driver_term.clone(),
            BufReader::new(stdout),
            stdin,
            prompt_rx,
        )
        .await
        {
            driver_term.emit(&format!("\r\n[ERROR] protocol session ended: {error}\r\n"));
        }
    });
    let served = serve_host(HostParts {
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
        resize: Arc::new(|_, _| {}),
        protocol: true,
    })
    .await;
    driver.abort();
    let _ = child.kill().await;
    served
}

struct Context {
    kind: Kind,
    session_id: String,
    agent: &'static str,
    cwd: String,
    model: Option<String>,
    daemon: DaemonEndpoint,
}

struct Wire<W> {
    out: W,
    next_id: u64,
}

impl<W: AsyncWrite + Unpin> Wire<W> {
    async fn send(&mut self, value: &Value) -> Result<(), Error> {
        let mut line =
            serde_json::to_string(value).map_err(|error| Error::Process(error.to_string()))?;
        line.push('\n');
        self.out
            .write_all(line.as_bytes())
            .await
            .map_err(|error| Error::Process(error.to_string()))?;
        self.out
            .flush()
            .await
            .map_err(|error| Error::Process(error.to_string()))
    }

    /// Sends a request and returns its id.
    async fn request(&mut self, method: &str, params: Value) -> Result<u64, Error> {
        self.next_id += 1;
        let id = self.next_id;
        self.send(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}))
            .await?;
        Ok(id)
    }
}

/// Reads lines until the response to request `id`, ignoring everything else (start-up only).
async fn response_to<R: AsyncBufRead + Unpin>(
    lines: &mut tokio::io::Lines<R>,
    id: u64,
) -> Result<Value, Error> {
    while let Some(line) = lines
        .next_line()
        .await
        .map_err(|error| Error::Process(error.to_string()))?
    {
        let Ok(message) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if message.get("method").is_none() && message.get("id") == Some(&json!(id)) {
            if let Some(error) = message.get("error") {
                return Err(Error::Process(format!("agent rejected start-up: {error}")));
            }
            return Ok(message.get("result").cloned().unwrap_or(Value::Null));
        }
    }
    Err(Error::Process("agent exited during start-up".to_owned()))
}

fn safe_model(model: &str) -> bool {
    !model.is_empty()
        && !model.starts_with('-')
        && model.len() <= 128
        && model
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.:/[]".contains(&byte))
}

/// Drives one protocol session: start-up, then prompts in and approvals/output out.
async fn drive<R, W>(
    ctx: Context,
    term: Term,
    reader: R,
    writer: W,
    mut prompts: mpsc::UnboundedReceiver<String>,
) -> Result<(), Error>
where
    R: AsyncBufRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut lines = reader.lines();
    let mut wire = Wire {
        out: writer,
        next_id: 1,
    };
    let mut model = ctx.model.clone();
    let remote_id = match ctx.kind {
        Kind::Codex => {
            codex_app_server::send_initialize(&mut wire.out, 1).await?;
            response_to(&mut lines, 1).await?;
            let mut params = json!({"cwd": ctx.cwd});
            if let Some(model) = &model {
                params["model"] = json!(model);
            }
            let id = wire.request("thread/start", params).await?;
            let result = response_to(&mut lines, id).await?;
            result
                .pointer("/thread/id")
                .and_then(Value::as_str)
                .map(str::to_owned)
        }
        Kind::Acp => {
            acp_approvals::send_initialize(&mut wire.out, 1).await?;
            response_to(&mut lines, 1).await?;
            let id = wire
                .request("session/new", json!({"cwd": ctx.cwd, "mcpServers": []}))
                .await?;
            let result = response_to(&mut lines, id).await?;
            let session = result
                .get("sessionId")
                .and_then(Value::as_str)
                .map(str::to_owned);
            if let (Some(session), Some(model)) = (&session, &model) {
                let id = wire
                    .request(
                        "session/set_model",
                        json!({"sessionId": session, "modelId": model}),
                    )
                    .await?;
                let _ = response_to(&mut lines, id).await;
            }
            session
        }
    };
    let Some(remote_id) = remote_id else {
        return Err(Error::Process("agent returned no session id".to_owned()));
    };
    term.emit(&format!(
        "[RUN] {} protocol session ready. Approvals appear in the Gnomcode console and phone.\r\n\r\n> ",
        if ctx.kind == Kind::Codex {
            "Codex"
        } else {
            "Antigravity"
        }
    ));

    let mut busy = false;
    let mut turn_request: Option<u64> = None;
    let mut queued: VecDeque<String> = VecDeque::new();
    loop {
        tokio::select! {
            line = lines.next_line() => {
                let Some(line) = line.map_err(|error| Error::Process(error.to_string()))? else {
                    return Ok(());
                };
                let Ok(message) = serde_json::from_str::<Value>(&line) else { continue };
                handle_message(
                    &ctx, &term, &mut wire, &message, &mut busy, turn_request,
                ).await?;
            }
            prompt = prompts.recv(), if !busy => {
                let Some(prompt) = prompt else { return Ok(()) };
                queued.push_back(prompt);
            }
        }
        while !busy {
            let Some(prompt) = queued.pop_front() else {
                break;
            };
            if let Some(next) = prompt.strip_prefix("/model ") {
                let next = next.trim();
                if safe_model(next) {
                    model = Some(next.to_owned());
                    if ctx.kind == Kind::Acp {
                        let _ = wire
                            .request(
                                "session/set_model",
                                json!({"sessionId": remote_id, "modelId": next}),
                            )
                            .await?;
                    }
                    term.emit(&format!("[RUN] model set to {next}\r\n\r\n> "));
                } else {
                    term.emit("[ERROR] unsupported model name\r\n\r\n> ");
                }
                continue;
            }
            let id = match ctx.kind {
                Kind::Codex => {
                    let mut params = json!({
                        "threadId": remote_id,
                        "input": [{"type": "text", "text": prompt, "text_elements": []}],
                    });
                    if let Some(model) = &model {
                        params["model"] = json!(model);
                    }
                    wire.request("turn/start", params).await?
                }
                Kind::Acp => {
                    wire.request(
                        "session/prompt",
                        json!({"sessionId": remote_id, "prompt": [{"type": "text", "text": prompt}]}),
                    )
                    .await?
                }
            };
            turn_request = Some(id);
            busy = true;
        }
    }
}

/// Renders one agent message and answers server requests (approvals fail closed).
async fn handle_message<W: AsyncWrite + Unpin>(
    ctx: &Context,
    term: &Term,
    wire: &mut Wire<W>,
    message: &Value,
    busy: &mut bool,
    turn_request: Option<u64>,
) -> Result<(), Error> {
    let method = message.get("method").and_then(Value::as_str);
    let has_id = message.get("id").is_some_and(|id| !id.is_null());
    match (method, has_id) {
        (Some(_), true) => answer_server_request(ctx, term, wire, message).await,
        (Some(method), false) => {
            render_notification(ctx.kind, term, method, message.get("params"), busy);
            Ok(())
        }
        (None, true) => {
            if turn_request.is_some()
                && message.get("id") == turn_request.map(|id| json!(id)).as_ref()
            {
                if let Some(error) = message.get("error") {
                    term.emit(&format!("\r\n[ERROR] {}\r\n\r\n> ", error_text(error)));
                    *busy = false;
                } else if ctx.kind == Kind::Acp {
                    let reason = message
                        .pointer("/result/stopReason")
                        .and_then(Value::as_str)
                        .unwrap_or("done");
                    term.emit(&format!("\r\n[DONE] {reason}\r\n\r\n> "));
                    *busy = false;
                }
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn error_text(error: &Value) -> String {
    error
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("request failed")
        .chars()
        .take(200)
        .collect()
}

fn render_notification(
    kind: Kind,
    term: &Term,
    method: &str,
    params: Option<&Value>,
    busy: &mut bool,
) {
    let Some(params) = params else { return };
    match (kind, method) {
        (Kind::Codex, "item/agentMessage/delta") => {
            if let Some(delta) = params.get("delta").and_then(Value::as_str) {
                term.emit(delta);
            }
        }
        (Kind::Codex, "item/started") => {
            if params.pointer("/item/type").and_then(Value::as_str) == Some("commandExecution") {
                let command = params
                    .pointer("/item/command")
                    .and_then(Value::as_str)
                    .unwrap_or("command");
                term.emit(&format!("\r\n[RUN] {command}\r\n"));
            }
        }
        (Kind::Codex, "turn/completed") => {
            let status = params
                .pointer("/turn/status")
                .and_then(Value::as_str)
                .unwrap_or("completed");
            let label = match status {
                "interrupted" => "[CANCEL] turn interrupted",
                "failed" => "[ERROR] turn failed",
                _ => "[DONE] turn complete",
            };
            term.emit(&format!("\r\n{label}\r\n\r\n> "));
            *busy = false;
        }
        (Kind::Codex, "error") => {
            if let Some(error) = params.get("error") {
                term.emit(&format!("\r\n[ERROR] {}\r\n", error_text(error)));
            }
        }
        (Kind::Acp, "session/update") => {
            let update = params.get("update").unwrap_or(&Value::Null);
            match update.get("sessionUpdate").and_then(Value::as_str) {
                Some("agent_message_chunk") => {
                    if let Some(text) = update.pointer("/content/text").and_then(Value::as_str) {
                        term.emit(text);
                    }
                }
                Some("tool_call") => {
                    let title = update
                        .get("title")
                        .or_else(|| update.get("name"))
                        .and_then(Value::as_str)
                        .unwrap_or("tool");
                    term.emit(&format!("\r\n[RUN] {title}\r\n"));
                }
                _ => {}
            }
        }
        _ => {}
    }
}

/// Answers an approval request through the ADR 0032 API; anything else the host cannot serve
/// gets a JSON-RPC error so the agent never waits on it.
async fn answer_server_request<W: AsyncWrite + Unpin>(
    ctx: &Context,
    term: &Term,
    wire: &mut Wire<W>,
    message: &Value,
) -> Result<(), Error> {
    let endpoint = (ctx.daemon)();
    let response = match ctx.kind {
        Kind::Codex => codex_app_server::parse_server_request(message)
            .map(|request| (request.tool.clone(), request.summary.clone(), request)),
        Kind::Acp => None,
    };
    if let Some((tool, summary, request)) = response {
        term.emit(&format!("\r\n[WAIT] approval needed: {tool} {summary}\r\n"));
        let outcome = match endpoint {
            Some((port, token)) => {
                codex_app_server::request_daemon_approval(port, &token, &ctx.session_id, &request)
                    .await
            }
            None => ApprovalOutcome::NoDecision,
        };
        term.emit(match outcome {
            ApprovalOutcome::Allow => "[ALLOW] approved once\r\n",
            ApprovalOutcome::Deny => "[DENY] denied\r\n",
            ApprovalOutcome::NoDecision => "[CANCEL] no decision; request cancelled\r\n",
        });
        return wire
            .send(&codex_app_server::build_approval_response(
                &request, outcome,
            ))
            .await;
    }
    if ctx.kind == Kind::Acp {
        if let Some(request) = acp_approvals::parse_permission_request(message) {
            term.emit(&format!(
                "\r\n[WAIT] approval needed: {} {}\r\n",
                request.tool, request.summary
            ));
            let outcome = match endpoint {
                Some((port, token)) => {
                    acp_approvals::request_daemon_approval(
                        port,
                        &token,
                        ctx.agent,
                        &ctx.session_id,
                        &request,
                    )
                    .await
                }
                None => AcpApprovalOutcome::NoDecision,
            };
            term.emit(match outcome {
                AcpApprovalOutcome::Allow => "[ALLOW] approved once\r\n",
                AcpApprovalOutcome::Deny => "[DENY] denied\r\n",
                AcpApprovalOutcome::NoDecision => "[CANCEL] no decision; request cancelled\r\n",
            });
            return wire
                .send(&acp_approvals::build_permission_response(&request, outcome))
                .await;
        }
    }
    let id = message.get("id").cloned().unwrap_or(Value::Null);
    wire.send(&json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {"code": -32601, "message": "unsupported by Gnomcode protocol host"},
    }))
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncBufReadExt, BufReader, duplex};

    fn feed(composer: &mut Composer, bytes: &[u8]) -> (Vec<String>, String) {
        let mut echo = Vec::new();
        let prompts = composer.feed(bytes, &mut echo);
        (prompts, String::from_utf8_lossy(&echo).into_owned())
    }

    #[test]
    fn the_setting_is_per_agent_and_off_until_enabled() {
        let dir = tempfile::tempdir().expect("dir");
        let path = dir.path().join(SETTINGS_FILE);
        assert!(!enabled_in(&path, "codex"));
        set_in(&path, "codex", true).expect("set");
        assert!(enabled_in(&path, "codex"));
        assert!(!enabled_in(&path, "antigravity"));
        set_in(&path, "codex", false).expect("clear");
        assert!(!enabled_in(&path, "codex"));
        assert_eq!(setting_key("agy"), Some("antigravity"));
        assert_eq!(setting_key("claude"), None);
        assert!(matches!(
            set_protocol_approvals("claude", true),
            Err(Error::UnsupportedAgent(_))
        ));
    }

    #[test]
    fn a_pasted_prompt_is_submitted_by_the_enter_after_it() {
        let mut composer = Composer::default();
        let (prompts, _) = feed(&mut composer, b"\x1b[200~fix the\nbug\x1b[201~");
        assert!(prompts.is_empty());
        let (prompts, _) = feed(&mut composer, b"\x1b[F");
        assert!(prompts.is_empty());
        let (prompts, echo) = feed(&mut composer, b"\r");
        assert_eq!(prompts, ["fix the\nbug"]);
        assert_eq!(echo, "\r\n");
    }

    #[test]
    fn typed_text_echoes_and_backspace_edits() {
        let mut composer = Composer::default();
        let (prompts, echo) = feed(&mut composer, b"helx\x7flo\r");
        assert_eq!(prompts, ["hello"]);
        assert!(echo.contains("hel") && echo.ends_with("\r\n"));
        let (prompts, _) = feed(&mut composer, b"\x1b[A\r");
        assert!(
            prompts.is_empty(),
            "arrow keys and empty lines submit nothing"
        );
    }

    fn term() -> Term {
        let (tx, _) = broadcast::channel(8);
        Term {
            scrollback: Arc::new(Mutex::new(Scrollback::new(4096))),
            tx,
        }
    }

    fn context(kind: Kind, daemon: DaemonEndpoint) -> Context {
        Context {
            kind,
            session_id: "codex-1".to_owned(),
            agent: if kind == Kind::Codex {
                "codex"
            } else {
                "antigravity"
            },
            cwd: "C:/work".to_owned(),
            model: None,
            daemon,
        }
    }

    /// Plays the agent: answers start-up, sends one approval request after the prompt and
    /// returns the client's answer to it.
    async fn fake_agent(
        kind: Kind,
        mut from_host: tokio::io::Lines<BufReader<tokio::io::DuplexStream>>,
        mut to_host: tokio::io::DuplexStream,
    ) -> Value {
        async fn read(lines: &mut tokio::io::Lines<BufReader<tokio::io::DuplexStream>>) -> Value {
            serde_json::from_str(&lines.next_line().await.expect("io").expect("line"))
                .expect("json")
        }
        async fn write(to_host: &mut tokio::io::DuplexStream, value: Value) {
            to_host
                .write_all(format!("{value}\n").as_bytes())
                .await
                .expect("write");
        }
        let init = read(&mut from_host).await;
        write(
            &mut to_host,
            json!({"jsonrpc":"2.0","id":init["id"],"result":{}}),
        )
        .await;
        if kind == Kind::Codex {
            assert_eq!(read(&mut from_host).await["method"], "initialized");
        }
        let start = read(&mut from_host).await;
        let result = if kind == Kind::Codex {
            assert_eq!(start["method"], "thread/start");
            json!({"thread":{"id":"t1"}})
        } else {
            assert_eq!(start["method"], "session/new");
            json!({"sessionId":"s1"})
        };
        write(
            &mut to_host,
            json!({"jsonrpc":"2.0","id":start["id"],"result":result}),
        )
        .await;
        let turn = read(&mut from_host).await;
        assert!(["turn/start", "session/prompt"].contains(&turn["method"].as_str().unwrap_or("")));
        write(
            &mut to_host,
            json!({"jsonrpc":"2.0","id":turn["id"],"result":{}}),
        )
        .await;
        let request = if kind == Kind::Codex {
            json!({"jsonrpc":"2.0","id":77,"method":"item/commandExecution/requestApproval",
                   "params":{"command":"cargo test","threadId":"t1","turnId":"u1","itemId":"i1"}})
        } else {
            json!({"jsonrpc":"2.0","id":77,"method":"session/request_permission",
                   "params":{"sessionId":"s1","toolCall":{"kind":"edit","title":"File a.txt"},
                   "options":[{"optionId":"allow","kind":"allow_once"},{"optionId":"deny","kind":"reject_once"}]}})
        };
        write(&mut to_host, request).await;
        read(&mut from_host).await
    }

    async fn run(kind: Kind, daemon: DaemonEndpoint) -> Value {
        let (host_out, agent_in) = duplex(64 * 1024);
        let (agent_out, host_in) = duplex(64 * 1024);
        let (prompt_tx, prompt_rx) = mpsc::unbounded_channel();
        prompt_tx.send("do it".to_owned()).expect("prompt");
        let driver = tokio::spawn(drive(
            context(kind, daemon),
            term(),
            BufReader::new(host_in),
            host_out,
            prompt_rx,
        ));
        let answer = fake_agent(kind, BufReader::new(agent_in).lines(), agent_out).await;
        driver.abort();
        answer
    }

    #[tokio::test]
    async fn without_a_daemon_every_approval_is_cancelled_never_allowed() {
        let none: DaemonEndpoint = Arc::new(|| None);
        let codex = run(Kind::Codex, Arc::clone(&none)).await;
        assert_eq!(codex["id"], 77);
        assert_eq!(codex["result"]["decision"], "cancel");
        let agy = run(Kind::Acp, none).await;
        assert_eq!(agy["result"]["outcome"]["outcome"], "cancelled");
    }

    /// A loopback stand-in for the daemon's ADR 0032 API that answers `outcome`.
    async fn mock_daemon(outcome: &'static str) -> u16 {
        use axum::routing::{get, post};
        let app = axum::Router::new()
            .route(
                "/api/v1/approvals",
                post(|| async { axum::Json(json!({"hold": true, "id": "abc"})) }),
            )
            .route(
                "/api/v1/approvals/abc/wait",
                get(move || async move { axum::Json(json!({"outcome": outcome})) }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let port = listener.local_addr().expect("addr").port();
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        port
    }

    #[tokio::test]
    async fn a_console_decision_reaches_the_agent_as_allow_once_or_deny() {
        for (outcome, codex_decision, agy_option) in
            [("allow", "accept", "allow"), ("deny", "decline", "deny")]
        {
            let port = mock_daemon(outcome).await;
            let daemon: DaemonEndpoint = Arc::new(move || Some((port, "token".to_owned())));
            let codex = run(Kind::Codex, Arc::clone(&daemon)).await;
            assert_eq!(codex["result"]["decision"], codex_decision);
            let agy = run(Kind::Acp, daemon).await;
            assert_eq!(agy["result"]["outcome"]["optionId"], agy_option);
        }
    }

    #[tokio::test]
    async fn a_daemon_that_holds_nothing_or_answers_none_cancels() {
        let port = mock_daemon("none").await;
        let daemon: DaemonEndpoint = Arc::new(move || Some((port, "token".to_owned())));
        let codex = run(Kind::Codex, Arc::clone(&daemon)).await;
        assert_eq!(codex["result"]["decision"], "cancel");
        let agy = run(Kind::Acp, daemon).await;
        assert_eq!(agy["result"]["outcome"]["outcome"], "cancelled");
    }
}
