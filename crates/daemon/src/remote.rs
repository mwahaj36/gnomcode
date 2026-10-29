//! Opt-in outbound encrypted Remote pairing and bounded chat commands.

use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use serde_json::{Value, json};
use spawnt_core::remote_crypto::{PeerRole, RemoteChannel, RemoteKeyPair};
use spawnt_core::{EncryptedRemoteFrame, Project, Session};
use spawnt_store::Database;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use thiserror::Error;
use tokio::sync::{Mutex, mpsc, watch};
use tokio_tungstenite::tungstenite::Message;

/// Failure to establish or use an encrypted Remote pairing.
#[derive(Debug, Error)]
pub enum RemoteError {
    /// The relay has not been configured.
    #[error("configure SPAWNT_REMOTE_RELAY_URL before pairing")]
    NotConfigured,
    /// The relay could not be reached.
    #[error("remote relay connection failed")]
    Relay,
    /// No active pairing exists.
    #[error("no active remote pairing")]
    NotPaired,
    /// The confirmation code does not match.
    #[error("confirmation code does not match")]
    WrongCode,
    /// The pairing has expired.
    #[error("remote pairing expired")]
    Expired,
}

/// Current pairing state returned only by the authenticated loopback API.
#[derive(Clone, Serialize)]
pub struct PairingStatus {
    /// Whether a pairing room exists.
    pub enabled: bool,
    /// Whether both devices have confirmed the same code.
    pub paired: bool,
    /// Random room identifier.
    pub room_id: Option<String>,
    /// Desktop ephemeral X25519 public key.
    pub public_key: Option<String>,
    /// Six-digit code once a phone has joined.
    pub verify_code: Option<String>,
    /// Relay endpoint used by both devices.
    pub relay_url: Option<String>,
    /// Pairing expiration before confirmation.
    pub expires_at: Option<String>,
}

/// One room at a time; replacing or revoking a room closes its connection.
#[derive(Default)]
pub struct RemoteManager {
    current: Mutex<Option<Arc<Room>>>,
}

struct Room {
    id: String,
    public_key: String,
    relay_url: String,
    expires_at: chrono::DateTime<chrono::Utc>,
    keys: RemoteKeyPair,
    channel: Mutex<Option<RemoteChannel>>,
    local_confirmed: AtomicBool,
    phone_confirmed: AtomicBool,
    connected: AtomicBool,
    outgoing: mpsc::Sender<Value>,
    cancel: watch::Sender<bool>,
}

impl RemoteManager {
    /// Creates an empty Remote manager.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Connects outbound to the configured relay and starts fresh pairing.
    ///
    /// # Errors
    /// Returns [`RemoteError`] for missing configuration or failed relay connection.
    pub async fn start(&self, db: Arc<Mutex<Database>>) -> Result<PairingStatus, RemoteError> {
        let relay_url = std::env::var("SPAWNT_REMOTE_RELAY_URL")
            .unwrap_or_else(|_| "wss://spawnt-remote-relay.mwahaj25.workers.dev/relay".to_owned());
        self.start_with_url(db, relay_url).await
    }

    async fn start_with_url(
        &self,
        db: Arc<Mutex<Database>>,
        relay_url: String,
    ) -> Result<PairingStatus, RemoteError> {
        if !relay_url.starts_with("wss://")
            && !relay_url.starts_with("ws://127.0.0.1:")
            && !relay_url.starts_with("ws://localhost:")
        {
            return Err(RemoteError::NotConfigured);
        }
        if relay_url.contains(['?', '#', '&']) || !relay_url.ends_with("/relay") {
            return Err(RemoteError::NotConfigured);
        }
        let mut room_bytes = [0; 16];
        let mut private = [0; 32];
        getrandom::fill(&mut room_bytes).map_err(|_| RemoteError::Relay)?;
        getrandom::fill(&mut private).map_err(|_| RemoteError::Relay)?;
        let id = room_bytes
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let url = format!("{}/{id}?role=desktop", relay_url.trim_end_matches('/'));
        let (socket, _) = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            tokio_tungstenite::connect_async(&url),
        )
        .await
        .map_err(|_| RemoteError::Relay)?
        .map_err(|_| RemoteError::Relay)?;
        let (outgoing, receiver) = mpsc::channel(32);
        let (cancel, canceled) = watch::channel(false);
        let keys = RemoteKeyPair::from_private(private);
        let room = Arc::new(Room {
            id,
            public_key: keys.public_key_hex(),
            relay_url,
            expires_at: chrono::Utc::now() + chrono::Duration::minutes(5),
            keys,
            channel: Mutex::new(None),
            local_confirmed: AtomicBool::new(false),
            phone_confirmed: AtomicBool::new(false),
            connected: AtomicBool::new(true),
            outgoing,
            cancel,
        });
        let previous = self.current.lock().await.replace(room.clone());
        if let Some(previous) = previous {
            let _ = previous.cancel.send(true);
        }
        let status = room.status().await;
        tokio::spawn(run_room(room, db, socket, receiver, canceled));
        Ok(status)
    }

    /// Returns the current room and verification code without exposing private key material.
    pub async fn status(&self) -> PairingStatus {
        match self.current.lock().await.clone() {
            Some(room) => room.status().await,
            None => PairingStatus {
                enabled: false,
                paired: false,
                room_id: None,
                public_key: None,
                verify_code: None,
                relay_url: None,
                expires_at: None,
            },
        }
    }

    /// Confirms the code displayed on the paired phone.
    ///
    /// # Errors
    /// Returns [`RemoteError`] if no phone is present or the code differs.
    pub async fn confirm(&self, code: &str) -> Result<PairingStatus, RemoteError> {
        let room = self
            .current
            .lock()
            .await
            .clone()
            .ok_or(RemoteError::NotPaired)?;
        if !room.connected.load(Ordering::Acquire) {
            return Err(RemoteError::NotPaired);
        }
        if chrono::Utc::now() > room.expires_at {
            return Err(RemoteError::Expired);
        }
        let actual = room
            .channel
            .lock()
            .await
            .as_ref()
            .map(|channel| channel.verify_code().to_owned())
            .ok_or(RemoteError::NotPaired)?;
        if code != actual {
            return Err(RemoteError::WrongCode);
        }
        room.local_confirmed.store(true, Ordering::Release);
        room.outgoing
            .send(json!({"type":"confirm"}))
            .await
            .map_err(|_| RemoteError::Relay)?;
        Ok(room.status().await)
    }

    /// Closes the room and destroys its ephemeral key material.
    pub async fn revoke(&self) -> bool {
        let previous = self.current.lock().await.take();
        if let Some(room) = previous {
            let _ = room.cancel.send(true);
            true
        } else {
            false
        }
    }
}

impl Room {
    async fn status(&self) -> PairingStatus {
        if !self.connected.load(Ordering::Acquire) {
            return PairingStatus {
                enabled: false,
                paired: false,
                room_id: None,
                public_key: None,
                verify_code: None,
                relay_url: None,
                expires_at: None,
            };
        }
        let verify_code = self
            .channel
            .lock()
            .await
            .as_ref()
            .map(|channel| channel.verify_code().to_owned());
        PairingStatus {
            enabled: self.connected.load(Ordering::Acquire),
            paired: self.connected.load(Ordering::Acquire)
                && self.local_confirmed.load(Ordering::Acquire)
                && self.phone_confirmed.load(Ordering::Acquire),
            room_id: Some(self.id.clone()),
            public_key: Some(self.public_key.clone()),
            verify_code,
            relay_url: Some(self.relay_url.clone()),
            expires_at: Some(self.expires_at.to_rfc3339()),
        }
    }
}

struct ConnectionGuard(Arc<Room>);

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.0.connected.store(false, Ordering::Release);
    }
}

async fn run_room(
    room: Arc<Room>,
    db: Arc<Mutex<Database>>,
    socket: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    mut outgoing: mpsc::Receiver<Value>,
    mut canceled: watch::Receiver<bool>,
) {
    let _connection = ConnectionGuard(room.clone());
    let (mut writer, mut reader) = socket.split();
    let mut timer = tokio::time::interval(std::time::Duration::from_secs(10));
    loop {
        tokio::select! {
            _ = canceled.changed() => break,
            _ = timer.tick() => {
                if chrono::Utc::now() > room.expires_at
                    && !(room.local_confirmed.load(Ordering::Acquire)
                        && room.phone_confirmed.load(Ordering::Acquire)) { break; }
            }
            outbound = outgoing.recv() => {
                let Some(outbound) = outbound else { break; };
                let Ok(parts) = chunk_response(&outbound) else { break; };
                for part in parts {
                    let Ok(data) = serde_json::to_vec(&part) else { return; };
                    let frame = {
                        let mut guard = room.channel.lock().await;
                        match guard.as_mut().and_then(|channel| channel.seal(&data).ok()) {
                            Some(frame) => frame,
                            None => return,
                        }
                    };
                    let Ok(bytes) = serde_json::to_vec(&frame) else { return; };
                    if writer.send(Message::Binary(bytes.into())).await.is_err() { return; }
                }
            }
            inbound = reader.next() => {
                let Some(Ok(Message::Binary(bytes))) = inbound else { break; };
                if bytes.len() > 65536 { break; }
                if room.channel.lock().await.is_none() {
                    let Ok(value) = serde_json::from_slice::<Value>(&bytes) else { break; };
                    if value.get("type").and_then(Value::as_str) != Some("hello") { break; }
                    let Some(public) = value.get("public_key").and_then(Value::as_str) else { break; };
                    let Ok(public) = RemoteKeyPair::parse_public_hex(public) else { break; };
                    let Ok(channel) = RemoteChannel::new(&room.keys, public, &room.id, PeerRole::Desktop) else { break; };
                    *room.channel.lock().await = Some(channel);
                    continue;
                }
                let Ok(frame) = serde_json::from_slice::<EncryptedRemoteFrame>(&bytes) else { break; };
                let plaintext = {
                    let mut guard = room.channel.lock().await;
                    match guard.as_mut().and_then(|channel| channel.open(&frame).ok()) {
                        Some(data) => data,
                        None => break,
                    }
                };
                let Ok(value) = serde_json::from_slice::<Value>(&plaintext) else { break; };
                if value.get("type").and_then(Value::as_str) == Some("confirm") {
                    room.phone_confirmed.store(true, Ordering::Release);
                    continue;
                }
                if !(room.local_confirmed.load(Ordering::Acquire)
                    && room.phone_confirmed.load(Ordering::Acquire)) { break; }
                let response = dispatch(&db, value).await;
                if room.outgoing.send(response).await.is_err() { break; }
            }
        }
    }
    let _ = writer.send(Message::Close(None)).await;
}

fn chunk_response(value: &Value) -> Result<Vec<Value>, serde_json::Error> {
    let encoded = serde_json::to_string(value)?;
    let mut parts = Vec::new();
    let mut start = 0;
    while start < encoded.len() {
        let mut end = (start + 12_000).min(encoded.len());
        while !encoded.is_char_boundary(end) {
            end -= 1;
        }
        parts
            .push(json!({"type":"chunk","data":&encoded[start..end],"final":end == encoded.len()}));
        start = end;
    }
    Ok(parts)
}

async fn dispatch(db: &Arc<Mutex<Database>>, request: Value) -> Value {
    match request.get("type").and_then(Value::as_str) {
        Some("sessions") => {
            let result = db.lock().await.list_active_sessions(1000);
            match result {
                Ok(sessions) => {
                    let hosted = spawnt_pty::list_active_sessions();
                    let mut visible = sessions.iter().take(100).map(|session| {
                        let can_prompt = session.state == spawnt_core::SessionState::Idle
                            && prompt_host(session, &sessions, &hosted).is_some();
                        json!({"id":session.id,"title":session.title,"agent":session.agent,"state":session.state,"can_prompt":can_prompt})
                    }).collect::<Vec<_>>();
                    for h in &hosted {
                        if !visible
                            .iter()
                            .any(|v| v.get("id").and_then(Value::as_str) == Some(&h.session_id))
                        {
                            visible.push(json!({
                                "id": h.session_id,
                                "title": format!("{} ({})", h.agent, h.repo_path),
                                "agent": h.agent,
                                "state": "unknown",
                                "can_prompt": false,
                            }));
                        }
                    }
                    json!({"type":"sessions","sessions":visible})
                }
                Err(_) => json!({"type":"error","message":"Could not load sessions"}),
            }
        }
        Some("launch") => {
            let Some(agent) = request.get("agent").and_then(Value::as_str) else {
                return json!({"type":"error","message":"Missing agent"});
            };
            if !matches!(
                agent,
                "claude" | "claude-code" | "codex" | "gemini" | "gemini-cli"
            ) {
                return json!({"type":"error","message":"Remote launch supports agent CLIs only"});
            }
            let Some(repo_path) = request.get("repo_path").and_then(Value::as_str) else {
                return json!({"type":"error","message":"Missing repo_path"});
            };
            let model = request.get("model").and_then(Value::as_str);
            let repo = Path::new(repo_path);
            if !repo.is_dir() {
                return json!({"type":"error","message":"Invalid workspace directory"});
            }
            let session_id = format!(
                "{}-{}",
                agent,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or(0)
            );
            match spawnt_pty::launch_session(&session_id, agent, repo, model).await {
                Ok(record) => {
                    json!({"type":"launch_ack","session_id":record.session_id,"agent":record.agent})
                }
                Err(err) => json!({"type":"error","message":format!("Launch failed: {err}")}),
            }
        }
        Some("chat") => {
            let Some(session_id) = request.get("session_id").and_then(Value::as_str) else {
                return json!({"type":"error","message":"Missing session"});
            };
            if session_id.len() > 128 {
                return json!({"type":"error","message":"Invalid session"});
            }
            let before = request.get("before").and_then(Value::as_str);
            if before.is_some_and(|value| value.len() > 256) {
                return json!({"type":"error","message":"Invalid cursor"});
            }
            let limit = request
                .get("limit")
                .and_then(Value::as_u64)
                .unwrap_or(200)
                .min(500);
            let db = db.lock().await;
            let sessions = db.list_active_sessions(1000).unwrap_or_default();
            let native_id =
                conversation_session_id(session_id, &sessions, &spawnt_pty::list_active_sessions());
            match db.get_session_chat_before(&native_id, Some(limit as usize), before) {
                Ok(events) => {
                    let events = spawnt_adapters::antigravity::clean_stored_conversation(events);
                    json!({"type":"chat","session_id":session_id,"before":before,"events":events})
                }
                Err(_) => json!({"type":"error","message":"Could not load chat"}),
            }
        }
        Some("prompt") => {
            let Some(session_id) = request.get("session_id").and_then(Value::as_str) else {
                return json!({"type":"error","message":"Missing session"});
            };
            let Some(text) = request.get("text").and_then(Value::as_str) else {
                return json!({"type":"error","message":"Missing prompt"});
            };
            if text.trim().is_empty()
                || text.len() > 4000
                || text.chars().any(char::is_control)
                || text.trim_start().starts_with('/')
            {
                return json!({"type":"error","message":"Invalid prompt"});
            }
            let hosted = spawnt_pty::list_active_sessions();
            let sessions = db
                .lock()
                .await
                .list_active_sessions(1000)
                .unwrap_or_default();
            let session = sessions.iter().find(|session| session.id == session_id);
            let Some(session) = session else {
                return json!({"type":"error","message":"Session unavailable; wait for the agent log to appear"});
            };
            let Some(record) = prompt_host(session, &sessions, &hosted) else {
                return json!({"type":"error","message":"Prompting requires an idle agent with a unique hosted session"});
            };
            let url = format!(
                "ws://127.0.0.1:{}/stream?token={}",
                record.port, record.token
            );
            let sent = async {
                let (mut socket, _) = tokio_tungstenite::connect_async(&url).await.ok()?;
                socket
                    .send(Message::Binary(format!("{text}\r").into_bytes().into()))
                    .await
                    .ok()?;
                Some(())
            }
            .await;
            if sent.is_some() {
                json!({"type":"prompt_ack","session_id":session_id})
            } else {
                json!({"type":"error","message":"Prompt could not be delivered"})
            }
        }
        Some("switch") => {
            let Some(session_id) = request.get("session_id").and_then(Value::as_str) else {
                return json!({"type":"error","message":"Missing session"});
            };
            let Some(model) = request.get("model").and_then(Value::as_str) else {
                return json!({"type":"error","message":"Missing model"});
            };
            if session_id.len() > 128 || model.len() > 128 {
                return json!({"type":"error","message":"Invalid switch request"});
            }
            let prepared = {
                let guard = db.lock().await;
                let sessions = guard.list_active_sessions(1000).unwrap_or_default();
                let native_id = conversation_session_id(
                    session_id,
                    &sessions,
                    &spawnt_pty::list_active_sessions(),
                );
                crate::prepare_switch(&guard, &native_id, model, None)
            };
            let Ok(prepared) = prepared else {
                return json!({"type":"error","message":"Could not prepare model switch"});
            };
            match crate::execute_switch(prepared).await {
                Ok(record) => {
                    json!({"type":"switch_ack","session_id":session_id,"new_session_id":record.session_id})
                }
                Err(_) => json!({"type":"error","message":"Model switch failed"}),
            }
        }
        _ => json!({"type":"error","message":"Unsupported remote command"}),
    }
}

pub(crate) fn prompt_host<'a>(
    session: &Session,
    sessions: &[Session],
    hosted: &'a [spawnt_pty::SessionRecord],
) -> Option<&'a spawnt_pty::SessionRecord> {
    if session.state != spawnt_core::SessionState::Idle {
        return None;
    }
    session_host(session, sessions, hosted)
}

pub(crate) fn conversation_session_id(
    id: &str,
    sessions: &[Session],
    hosted: &[spawnt_pty::SessionRecord],
) -> String {
    let matches = sessions
        .iter()
        .filter(|session| {
            session_host(session, sessions, hosted).is_some_and(|host| host.session_id == id)
        })
        .collect::<Vec<_>>();
    if matches.len() == 1 {
        matches[0].id.clone()
    } else {
        id.to_owned()
    }
}

pub(crate) fn session_host<'a>(
    session: &Session,
    sessions: &[Session],
    hosted: &'a [spawnt_pty::SessionRecord],
) -> Option<&'a spawnt_pty::SessionRecord> {
    if let Some(exact) = hosted
        .iter()
        .find(|record| Some(record.child_pid) == session.pid || record.session_id == session.id)
    {
        return Some(exact);
    }
    let matching = |record: &spawnt_pty::SessionRecord, candidate: &Session| {
        let agent = match record.agent.as_str() {
            "claude" => "claude-code",
            "gemini" => "gemini-cli",
            other => other,
        };
        let path = Path::new(&record.repo_path);
        let (root, remote) = spawnt_platform::resolve_git_repo_and_remote(path);
        let root = root.as_deref().unwrap_or(path);
        let project = Project::from_origin(remote.as_deref(), &root.to_string_lossy(), None);
        let started = chrono::DateTime::parse_from_rfc3339(&record.started_at).ok();
        agent == candidate.agent.as_str()
            && project.id == candidate.project_id
            && started.is_some_and(|start| {
                let delta = candidate.started_at.signed_duration_since(start);
                delta >= chrono::Duration::seconds(-5) && delta <= chrono::Duration::minutes(5)
            })
    };
    let candidates = hosted
        .iter()
        .filter(|record| matching(record, session))
        .collect::<Vec<_>>();
    let record = *candidates.first()?;
    if candidates.len() != 1
        || sessions
            .iter()
            .filter(|candidate| matching(record, candidate))
            .count()
            != 1
    {
        return None;
    }
    Some(record)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn remote_launch_refuses_shells_before_workspace_access() {
        let db = Arc::new(Mutex::new(Database::open_in_memory().expect("database")));
        for agent in ["terminal", "shell", "powershell", "unknown"] {
            let reply = dispatch(
                &db,
                json!({"type":"launch", "agent":agent, "repo_path":"missing-workspace"}),
            )
            .await;
            assert_eq!(reply["message"], "Remote launch supports agent CLIs only");
        }
    }

    #[test]
    fn conversation_binding_keeps_native_id_while_agent_is_thinking() {
        let now = chrono::Utc::now();
        let session = Session {
            id: "codex:native".into(),
            agent: spawnt_core::AgentId::Codex,
            surface: spawnt_core::Surface::Cli,
            project_id: "test".into(),
            parent_session_id: None,
            model: None,
            title: None,
            state: spawnt_core::SessionState::Thinking,
            pid: Some(12),
            started_at: now,
            last_event_at: now,
        };
        let record = spawnt_pty::SessionRecord {
            session_id: "codex-host".into(),
            agent: "codex".into(),
            repo_path: "unused".into(),
            model: None,
            host_pid: 11,
            child_pid: 12,
            port: 1,
            token: "test".into(),
            started_at: now.to_rfc3339(),
        };
        assert_eq!(
            conversation_session_id(
                "codex-host",
                std::slice::from_ref(&session),
                std::slice::from_ref(&record)
            ),
            "codex:native"
        );
        assert!(prompt_host(&session, std::slice::from_ref(&session), &[record]).is_none());
    }

    #[test]
    fn hosted_prompt_binding_requires_unique_recent_session() {
        let repo = tempfile::tempdir().expect("repo");
        let path = repo.path().to_string_lossy().into_owned();
        let now = chrono::Utc::now();
        let session = Session {
            id: "codex:native".to_owned(),
            agent: spawnt_core::AgentId::Codex,
            surface: spawnt_core::Surface::Cli,
            project_id: Project::from_origin(None, &path, None).id,
            parent_session_id: None,
            model: None,
            title: None,
            state: spawnt_core::SessionState::Idle,
            pid: None,
            started_at: now,
            last_event_at: now,
        };
        let record = spawnt_pty::SessionRecord {
            session_id: "codex-hosted".to_owned(),
            agent: "codex".to_owned(),
            repo_path: path,
            model: None,
            host_pid: 1,
            child_pid: 2,
            port: 1,
            token: "test".to_owned(),
            started_at: now.to_rfc3339(),
        };
        assert!(
            prompt_host(
                &session,
                std::slice::from_ref(&session),
                std::slice::from_ref(&record)
            )
            .is_some()
        );
        let mut other = session.clone();
        other.id = "codex:other".to_owned();
        assert!(prompt_host(&session, &[session.clone(), other], &[record]).is_none());
    }

    #[tokio::test]
    async fn pairing_confirms_both_peers_and_serves_encrypted_sessions() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let port = listener.local_addr().expect("address").port();
        let relay_task = tokio::spawn(async move {
            let (desktop_tcp, _) = listener.accept().await.expect("desktop");
            let desktop = tokio_tungstenite::accept_async(desktop_tcp)
                .await
                .expect("desktop websocket");
            let (phone_tcp, _) = listener.accept().await.expect("phone");
            let phone = tokio_tungstenite::accept_async(phone_tcp)
                .await
                .expect("phone websocket");
            let (mut desktop_out, mut desktop_in) = desktop.split();
            let (mut phone_out, mut phone_in) = phone.split();
            loop {
                tokio::select! {
                    message = desktop_in.next() => {
                        let Some(Ok(message)) = message else { break; };
                        if phone_out.send(message).await.is_err() { break; }
                    }
                    message = phone_in.next() => {
                        let Some(Ok(message)) = message else { break; };
                        if desktop_out.send(message).await.is_err() { break; }
                    }
                }
            }
        });
        let manager = RemoteManager::new();
        let db = Arc::new(Mutex::new(Database::open_in_memory().expect("database")));
        // Network verification is explicit; normal test runs stay on loopback.
        let base = std::env::var("SPAWNT_TEST_REMOTE_RELAY_URL")
            .unwrap_or_else(|_| format!("ws://127.0.0.1:{port}/relay"));
        let status = manager
            .start_with_url(db, base.clone())
            .await
            .expect("pairing");
        let room = status.room_id.expect("room");
        let desktop_public = RemoteKeyPair::parse_public_hex(&status.public_key.expect("public"))
            .expect("desktop key");
        let (mut phone_socket, _) =
            tokio_tungstenite::connect_async(format!("{base}/{room}?role=phone"))
                .await
                .expect("phone connect");
        let phone_keys = RemoteKeyPair::from_private([23; 32]);
        let mut phone_channel =
            RemoteChannel::new(&phone_keys, desktop_public, &room, PeerRole::Phone)
                .expect("phone channel");
        phone_socket
            .send(Message::Binary(
                serde_json::to_vec(
                    &json!({"type":"hello","public_key":phone_keys.public_key_hex()}),
                )
                .expect("hello")
                .into(),
            ))
            .await
            .expect("send hello");
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                if manager.status().await.verify_code.is_some() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("desktop derived code");
        assert_eq!(
            manager.status().await.verify_code.as_deref(),
            Some(phone_channel.verify_code())
        );
        assert!(!manager.status().await.paired);
        manager
            .confirm(phone_channel.verify_code())
            .await
            .expect("desktop confirm");
        let bytes = phone_socket
            .next()
            .await
            .expect("desktop confirmation frame")
            .expect("websocket frame")
            .into_data();
        let frame: EncryptedRemoteFrame = serde_json::from_slice(&bytes).expect("encrypted frame");
        let chunk: Value =
            serde_json::from_slice(&phone_channel.open(&frame).expect("decrypt")).expect("chunk");
        let confirmed: Value =
            serde_json::from_str(chunk["data"].as_str().expect("data")).expect("confirmation");
        assert_eq!(confirmed["type"], "confirm");
        assert!(!manager.status().await.paired);
        let phone_confirm = phone_channel
            .seal(br#"{"type":"confirm"}"#)
            .expect("seal confirm");
        phone_socket
            .send(Message::Binary(
                serde_json::to_vec(&phone_confirm).expect("frame").into(),
            ))
            .await
            .expect("send confirm");
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                if manager.status().await.paired {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("paired");
        let request = phone_channel
            .seal(br#"{"type":"sessions"}"#)
            .expect("seal request");
        phone_socket
            .send(Message::Binary(
                serde_json::to_vec(&request).expect("frame").into(),
            ))
            .await
            .expect("request sessions");
        let bytes = phone_socket
            .next()
            .await
            .expect("sessions frame")
            .expect("websocket frame")
            .into_data();
        let frame: EncryptedRemoteFrame =
            serde_json::from_slice(&bytes).expect("encrypted sessions");
        let chunk: Value =
            serde_json::from_slice(&phone_channel.open(&frame).expect("decrypt sessions"))
                .expect("sessions chunk");
        let sessions: Value =
            serde_json::from_str(chunk["data"].as_str().expect("data")).expect("sessions");
        assert_eq!(sessions["type"], "sessions");
        assert_eq!(sessions["sessions"], json!([]));
        assert!(manager.revoke().await);
        assert!(!manager.status().await.enabled);
        relay_task.abort();
    }

    #[test]
    fn large_remote_response_is_split_into_bounded_reassemblable_chunks() {
        let value = json!({"type":"chat","content":"long reply ".repeat(10000)});
        let parts = chunk_response(&value).expect("chunks");
        assert!(parts.len() > 1);
        assert!(
            parts
                .iter()
                .all(|part| serde_json::to_vec(part).expect("json").len() < 30000)
        );
        let joined = parts
            .iter()
            .map(|part| part["data"].as_str().expect("part"))
            .collect::<String>();
        assert_eq!(
            serde_json::from_str::<Value>(&joined).expect("response"),
            value
        );
        assert_eq!(parts.last().expect("last")["final"], true);
    }

    #[tokio::test]
    async fn remote_prompt_rejects_terminal_controls_and_commands() {
        let db = Arc::new(Mutex::new(Database::open_in_memory().expect("database")));
        for text in ["", "hello\r/exit", "/exit", "hello\u{1b}[2J"] {
            let response = dispatch(
                &db,
                json!({"type":"prompt","session_id":"missing","text":text}),
            )
            .await;
            assert_eq!(response["message"], "Invalid prompt");
        }
    }

    #[tokio::test]
    async fn remote_switch_rejects_unknown_model_before_launch() {
        let db = Arc::new(Mutex::new(Database::open_in_memory().expect("db")));
        let response = dispatch(
            &db,
            json!({"type":"switch","session_id":"missing","model":"--yolo"}),
        )
        .await;
        assert_eq!(response["message"], "Could not prepare model switch");
    }
}
