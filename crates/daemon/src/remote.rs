//! Opt-in outbound encrypted Remote pairing and bounded chat commands.

use crate::remote_trust::{DeviceTrust, IDLE_SECONDS};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use serde_json::{Value, json};
use spawnt_core::remote_crypto::{PeerRole, RemoteChannel, RemoteKeyPair};
use spawnt_core::{EncryptedRemoteFrame, Project, Session};
use spawnt_store::Database;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
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
    /// Local device trust could not be saved securely.
    #[error("could not save device trust securely")]
    Persistence,
}

/// Current pairing state returned only by the authenticated loopback API.
#[derive(Clone, Serialize)]
pub struct PairingStatus {
    /// Whether an unexpired device credential is retained locally.
    pub remembered: bool,
    /// Unix timestamp when saved device trust expires without further activity.
    pub trust_expires_at: Option<i64>,
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

impl PairingStatus {
    /// Temporary owner-scoped locator; encrypted device confirmation still controls access.
    pub(crate) fn pairing_code(&self) -> Option<String> {
        if !self.enabled || self.paired || self.remembered {
            return None;
        }
        let expires = chrono::DateTime::parse_from_rfc3339(self.expires_at.as_deref()?).ok()?;
        if expires <= chrono::Utc::now() {
            return None;
        }
        let room = self.room_id.as_deref()?;
        let value = u32::from_str_radix(room.get(..8)?, 16).ok()? % 100_000_000;
        Some(format!("{value:08}"))
    }
}

/// One room at a time; replacing or revoking a room closes its connection.
#[derive(Default)]
pub struct RemoteManager {
    current: Mutex<Option<Arc<Room>>>,
    discovery_changed: Arc<tokio::sync::Notify>,
    generation: Arc<AtomicU64>,
    persistence: Arc<Mutex<()>>,
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
    discovery_changed: Arc<tokio::sync::Notify>,
    identity: DeviceTrust,
    trusted: Mutex<Option<DeviceTrust>>,
    generation: Arc<AtomicU64>,
    expected_generation: u64,
    persistence: Arc<Mutex<()>>,
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
        let generation = self.generation.fetch_add(1, Ordering::AcqRel) + 1;
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
        let mut secret = [0; 32];
        getrandom::fill(&mut secret).map_err(|_| RemoteError::Relay)?;
        {
            let _gate = self.persistence.lock().await;
            clear_trust().await?;
        }
        self.start_record(
            db,
            DeviceTrust {
                room: id,
                relay: relay_url,
                private,
                secret,
                last_activity: chrono::Utc::now().timestamp(),
            },
            false,
            generation,
        )
        .await
    }

    /// Restores unexpired locally protected device trust after a daemon restart.
    pub async fn restore(&self, db: Arc<Mutex<Database>>) -> Result<(), RemoteError> {
        let generation = self.generation.load(Ordering::Acquire);
        let record = tokio::task::spawn_blocking(|| {
            std::fs::read(crate::remote_trust::path())
                .ok()
                .and_then(|bytes| serde_json::from_slice::<DeviceTrust>(&bytes).ok())
        })
        .await
        .map_err(|_| RemoteError::Persistence)?;
        if let Some(record) = record.filter(|record| record.valid(chrono::Utc::now().timestamp())) {
            if !valid_relay(&record.relay) || record.room.len() != 32 {
                return Err(RemoteError::NotConfigured);
            }
            loop {
                if generation != self.generation.load(Ordering::Acquire)
                    || self.current.lock().await.is_some()
                    || !record.valid(chrono::Utc::now().timestamp())
                {
                    break;
                }
                if self
                    .start_record(db.clone(), record.clone(), true, generation)
                    .await
                    .is_ok()
                {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        }
        Ok(())
    }

    async fn start_record(
        &self,
        db: Arc<Mutex<Database>>,
        identity: DeviceTrust,
        trusted: bool,
        generation: u64,
    ) -> Result<PairingStatus, RemoteError> {
        let id = identity.room.clone();
        let relay_url = identity.relay.clone();
        let private = identity.private;
        if !valid_relay(&relay_url) || id.len() != 32 || !id.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(RemoteError::NotConfigured);
        }
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
            discovery_changed: self.discovery_changed.clone(),
            channel: Mutex::new(None),
            local_confirmed: AtomicBool::new(false),
            phone_confirmed: AtomicBool::new(false),
            connected: AtomicBool::new(true),
            outgoing,
            cancel,
            trusted: Mutex::new(trusted.then(|| identity.clone())),
            identity,
            generation: self.generation.clone(),
            expected_generation: generation,
            persistence: self.persistence.clone(),
        });
        let mut current = self.current.lock().await;
        if generation != self.generation.load(Ordering::Acquire) {
            return Err(RemoteError::NotPaired);
        }
        let previous = current.replace(room.clone());
        drop(current);
        if let Some(previous) = previous {
            let _ = previous.cancel.send(true);
        }
        self.discovery_changed.notify_one();
        let status = room.status().await;
        tokio::spawn(run_room(room, db, socket, receiver, canceled));
        Ok(status)
    }

    /// Returns the current room and verification code without exposing private key material.
    pub async fn status(&self) -> PairingStatus {
        match self.current.lock().await.clone() {
            Some(room) => room.status().await,
            None => PairingStatus {
                remembered: false,
                trust_expires_at: None,
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
        self.discovery_changed.notify_one();
        room.outgoing
            .send(json!({"type":"confirm"}))
            .await
            .map_err(|_| RemoteError::Relay)?;
        room.establish_trust().await?;
        Ok(room.status().await)
    }

    /// Wakes metadata publication when the public pairing offer changes.
    pub(crate) async fn discovery_changed(&self) {
        self.discovery_changed.notified().await;
    }

    /// Closes the room and destroys its ephemeral key material.
    pub async fn revoke(&self) -> Result<bool, RemoteError> {
        self.generation.fetch_add(1, Ordering::AcqRel);
        let previous = self.current.lock().await.take();
        self.discovery_changed.notify_one();
        if let Some(room) = previous.as_ref() {
            let _ = room.cancel.send(true);
        }
        let _gate = self.persistence.lock().await;
        clear_trust().await?;
        Ok(previous.is_some())
    }
}

impl Room {
    async fn establish_trust(&self) -> Result<(), RemoteError> {
        let mut trusted = self.trusted.lock().await;
        let _gate = self.persistence.lock().await;
        if self.generation.load(Ordering::Acquire) != self.expected_generation {
            return Err(RemoteError::NotPaired);
        }
        if self.local_confirmed.load(Ordering::Acquire)
            && self.phone_confirmed.load(Ordering::Acquire)
            && trusted.is_none()
        {
            let mut trust = self.identity.clone();
            trust.last_activity = chrono::Utc::now().timestamp();
            let saved = trust.clone();
            tokio::task::spawn_blocking(move || saved.save())
                .await
                .map_err(|_| RemoteError::Persistence)??;
            *trusted = Some(trust.clone());
            self.outgoing.send(json!({"type":"trusted","secret":crate::remote_trust::hex(&trust.secret),"expires_at":trust.last_activity+IDLE_SECONDS})).await.map_err(|_|RemoteError::Relay)?;
        }
        Ok(())
    }

    async fn activity(&self) {
        let mut trusted = self.trusted.lock().await;
        let _gate = self.persistence.lock().await;
        if self.generation.load(Ordering::Acquire) != self.expected_generation {
            return;
        }
        if let Some(trust) = trusted.as_mut() {
            let now = chrono::Utc::now().timestamp();
            if trust.valid(now) && now - trust.last_activity >= 60 {
                trust.last_activity = now;
                let saved = trust.clone();
                let _ = tokio::task::spawn_blocking(move || saved.save()).await;
            }
        }
    }
    async fn status(&self) -> PairingStatus {
        let trusted = self.trusted.lock().await;
        let trusted = trusted
            .as_ref()
            .filter(|trust| trust.valid(chrono::Utc::now().timestamp()));
        let remembered = trusted.is_some();
        let connected = self.connected.load(Ordering::Acquire);
        let verify_code = if remembered {
            None
        } else {
            self.channel
                .lock()
                .await
                .as_ref()
                .map(|channel| channel.verify_code().to_owned())
        };
        PairingStatus {
            remembered,
            trust_expires_at: trusted.map(|trust| trust.last_activity + IDLE_SECONDS),
            enabled: connected || remembered,
            paired: connected
                && remembered
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
        self.0.discovery_changed.notify_one();
    }
}

async fn run_room(
    room: Arc<Room>,
    db: Arc<Mutex<Database>>,
    mut socket: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    mut outgoing: mpsc::Receiver<Value>,
    mut canceled: watch::Receiver<bool>,
) {
    let _connection = ConnectionGuard(room.clone());
    loop {
        while outgoing.try_recv().is_ok() {}
        *room.channel.lock().await = None;
        room.local_confirmed.store(false, Ordering::Release);
        room.phone_confirmed.store(false, Ordering::Release);
        room.connected.store(true, Ordering::Release);
        run_connection(&room, &db, socket, &mut outgoing, &mut canceled).await;
        room.connected.store(false, Ordering::Release);
        room.discovery_changed.notify_one();
        if *canceled.borrow()
            || !room
                .trusted
                .lock()
                .await
                .as_ref()
                .is_some_and(|trust| trust.valid(chrono::Utc::now().timestamp()))
        {
            break;
        }
        loop {
            tokio::select! {_=canceled.changed()=>return,_=tokio::time::sleep(std::time::Duration::from_secs(1))=>{}}
            let url = format!("{}/{}?role=desktop", room.relay_url, room.id);
            if let Ok(Ok((next, _))) = tokio::time::timeout(
                std::time::Duration::from_secs(10),
                tokio_tungstenite::connect_async(&url),
            )
            .await
            {
                socket = next;
                break;
            }
            if !room
                .trusted
                .lock()
                .await
                .as_ref()
                .is_some_and(|trust| trust.valid(chrono::Utc::now().timestamp()))
            {
                return;
            }
        }
    }
}

fn valid_relay(value: &str) -> bool {
    (value.starts_with("wss://")
        || value.starts_with("ws://127.0.0.1:")
        || value.starts_with("ws://localhost:"))
        && value.ends_with("/relay")
        && !value.contains(['?', '#', '&'])
}

async fn clear_trust() -> Result<(), RemoteError> {
    tokio::task::spawn_blocking(|| match std::fs::remove_file(crate::remote_trust::path()) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(RemoteError::Persistence),
    })
    .await
    .map_err(|_| RemoteError::Persistence)?
}

async fn run_connection(
    room: &Arc<Room>,
    db: &Arc<Mutex<Database>>,
    socket: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    outgoing: &mut mpsc::Receiver<Value>,
    canceled: &mut watch::Receiver<bool>,
) {
    let (mut writer, mut reader) = socket.split();
    let mut resume_context = String::new();
    let mut timer = tokio::time::interval(std::time::Duration::from_secs(10));
    loop {
        tokio::select! {
            _ = canceled.changed() => break,
            _ = timer.tick() => {
                let trusted=room.trusted.lock().await;
                if trusted.as_ref().is_some_and(|trust|!trust.valid(chrono::Utc::now().timestamp())) {break;}
                if trusted.is_none() && chrono::Utc::now() > room.expires_at
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
                if room.channel.lock().await.is_some() && serde_json::from_slice::<Value>(&bytes).ok().and_then(|v|v.get("type").and_then(Value::as_str).map(str::to_owned)).as_deref()==Some("hello") {continue;}
                if room.channel.lock().await.is_none() {
                    let Ok(value) = serde_json::from_slice::<Value>(&bytes) else { break; };
                    if value.get("type").and_then(Value::as_str) != Some("hello") { break; }
                    let Some(public) = value.get("public_key").and_then(Value::as_str) else { break; };
                    let public_hex=public.to_owned();
                    let Ok(public) = RemoteKeyPair::parse_public_hex(public) else { break; };
                    let mut nonce=[0;16];if getrandom::fill(&mut nonce).is_err(){break;}
                    let nonce=crate::remote_trust::hex(&nonce);
                    let context=format!("{}:{nonce}",room.id);
                    resume_context=format!("{context}:{public_hex}");
                    let Ok(channel) = RemoteChannel::new(&room.keys, public, &context, PeerRole::Desktop) else { break; };
                    *room.channel.lock().await = Some(channel);
                    let challenge=json!({"type":"challenge","nonce":nonce});
                    let Ok(bytes)=serde_json::to_vec(&challenge) else {break;};
                    if writer.send(Message::Binary(bytes.into())).await.is_err(){break;}
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
                if room.generation.load(Ordering::Acquire)!=room.expected_generation {break;}
                if value.get("type").and_then(Value::as_str)==Some("resume") {
                    let trusted=room.trusted.lock().await;
                    if !trusted.as_ref().is_some_and(|trust|trust.valid(chrono::Utc::now().timestamp()) && trust.verify(&resume_context,value.get("proof").and_then(Value::as_str).unwrap_or(""))) {drop(trusted);let _=room.outgoing.send(json!({"type":"resume_rejected"})).await;continue;}
                    drop(trusted);
                    room.local_confirmed.store(true,Ordering::Release);room.phone_confirmed.store(true,Ordering::Release);room.discovery_changed.notify_one();
                    let _=room.outgoing.send(json!({"type":"resumed"})).await;continue;
                }
                if value.get("type").and_then(Value::as_str) == Some("confirm") {
                    if room.trusted.lock().await.is_some(){break;}
                    room.phone_confirmed.store(true, Ordering::Release);
                    room.discovery_changed.notify_one();
                    if room.establish_trust().await.is_err(){break;}
                    continue;
                }
                if !(room.local_confirmed.load(Ordering::Acquire)
                    && room.phone_confirmed.load(Ordering::Acquire)) { break; }
                if !room.trusted.lock().await.as_ref().is_some_and(|trust|trust.valid(chrono::Utc::now().timestamp())) {break;}
                let request_type = value.get("type").cloned().unwrap_or(Value::Null);
                if matches!(request_type.as_str(),Some("activity"|"prompt"|"launch"|"switch"|"workspaces")) {room.activity().await;}
                if request_type.as_str()==Some("activity") {continue;}
                let mut response = dispatch(db, value).await;
                if response.get("type").and_then(Value::as_str) == Some("error") { response["request_type"] = request_type; }
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
        Some("workspaces") => {
            let path = request
                .get("path")
                .and_then(Value::as_str)
                .map(str::to_owned);
            if path
                .as_ref()
                .is_some_and(|path| path.len() > 4096 || path.chars().any(char::is_control))
            {
                return json!({"type":"error","message":"Invalid workspace path"});
            }
            match tokio::task::spawn_blocking(move || {
                spawnt_platform::workspaces::browse(path.as_deref())
            })
            .await
            {
                Ok(Ok(listing)) => {
                    let display = spawnt_platform::workspaces::display_path;
                    json!({"type":"workspaces","path":display(&listing.path),"parent":listing.parent.as_deref().map(display),"roots":listing.roots.iter().map(|path| display(path)).collect::<Vec<_>>(),"folders":listing.folders.iter().map(|path| json!({"path":display(path),"name":path.file_name().unwrap_or_default().to_string_lossy()})).collect::<Vec<_>>(),"truncated":listing.truncated})
                }
                _ => {
                    json!({"type":"error","message":"Cannot open that desktop folder. Choose another folder or enter its path."})
                }
            }
        }
        Some("sessions") => {
            let result = db.lock().await.list_active_sessions(1000);
            match result {
                Ok(sessions) => {
                    let hosted = spawnt_pty::list_active_sessions();
                    let mut visible = remote_sessions(&sessions, &hosted);
                    let bindings = session_bindings(&sessions, &hosted);
                    let guard = db.lock().await;
                    for row in &mut visible {
                        if row["title"].as_str().is_none_or(str::is_empty) {
                            let id = row["id"].as_str().unwrap_or_default();
                            let native = bindings
                                .iter()
                                .find(|(_, record)| record.session_id == id)
                                .map_or(id, |(session, _)| session.id.as_str());
                            let title = crate::conversation_title(&guard, native)
                                .or_else(|| {
                                    guard
                                        .get_session_focus_target(native)
                                        .ok()
                                        .flatten()
                                        .and_then(|target| target.repo_root)
                                        .and_then(|root| {
                                            Path::new(&root)
                                                .file_name()
                                                .map(|name| name.to_string_lossy().into_owned())
                                        })
                                })
                                .unwrap_or_else(|| "New conversation".to_owned());
                            row["title"] = json!(title);
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
                "claude"
                    | "claude-code"
                    | "codex"
                    | "gemini"
                    | "gemini-cli"
                    | "antigravity"
                    | "agy"
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
            match conversation_chat(
                &db,
                session_id,
                &native_id,
                &sessions,
                Some(limit as usize),
                before,
            ) {
                Ok(events) => {
                    let events = spawnt_adapters::codex::clean_stored_conversation(
                        spawnt_adapters::antigravity::clean_stored_conversation(events),
                    );
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
                || text
                    .chars()
                    .any(|character| character.is_control() && !matches!(character, '\n' | '\t'))
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
            let Some(record) = remote_prompt_target(session_id, &sessions, &hosted) else {
                return json!({"type":"error","message":"Prompting requires an idle agent with a unique hosted session"});
            };
            if spawnt_pty::send_prompt(record, text).await.is_ok() {
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
                    let guard = db.lock().await;
                    if record_model_change(&guard, session_id, &record).is_err() {
                        tracing::warn!("model change notice could not be saved");
                    }
                    json!({"type":"switch_ack","session_id":session_id,"new_session_id":record.session_id})
                }
                Err(_) => json!({"type":"error","message":"Model switch failed"}),
            }
        }
        _ => json!({"type":"error","message":"Unsupported remote command"}),
    }
}

pub(crate) fn record_model_change(
    db: &Database,
    source: &str,
    target: &spawnt_pty::SessionRecord,
) -> Result<(), spawnt_store::Error> {
    let last = db.get_session_chat(source, Some(1))?;
    let event = spawnt_core::ChatEvent {
        event_id: format!("handoff:{}:source", target.session_id),
        session_id: source.to_owned(),
        turn_index: last
            .last()
            .map_or(0, |event| event.turn_index.saturating_add(1)),
        role: spawnt_core::ChatRole::System,
        content: format!(
            "Agent/model change requested: {} / {}. Waiting for the agent to confirm the model.",
            target.agent,
            target.model.as_deref().unwrap_or("agent default")
        ),
        created_at: chrono::Utc::now(),
        tool_name: None,
        status: Some("model_changed".into()),
    };
    db.insert_chat_event(&event)
}

pub(crate) fn conversation_chat(
    db: &Database,
    requested: &str,
    native: &str,
    sessions: &[Session],
    limit: Option<usize>,
    before: Option<&str>,
) -> Result<Vec<spawnt_core::ChatEvent>, spawnt_store::Error> {
    let hosted = spawnt_pty::list_active_sessions();
    let host = sessions
        .iter()
        .find(|session| session.id == native)
        .and_then(|session| session_host(session, sessions, &hosted))
        .map_or(requested, |record| record.session_id.as_str());
    let mut events = db.get_session_chat_before(native, limit, before)?;
    if let Some(mut event) = db.handoff_notice(host)? {
        event.event_id = format!("handoff:{host}:target");
        event.session_id = native.to_owned();
        event.turn_index = 0;
        if !events.iter().any(|row| row.event_id == event.event_id)
            && (before.is_none() || events.first().is_some_and(|row| row.turn_index == 0))
        {
            events.insert(0, event);
        }
    }
    Ok(events)
}

fn remote_sessions(sessions: &[Session], hosted: &[spawnt_pty::SessionRecord]) -> Vec<Value> {
    let bindings = session_bindings(sessions, hosted);
    let mut visible = sessions.iter().take(100).map(|session| {
        let host = bindings.iter().find(|(candidate, _)| candidate.id == session.id).map(|(_, record)| *record);
        let can_prompt = session.state == spawnt_core::SessionState::Idle && host.is_some();
        json!({"id":host.map_or(session.id.as_str(), |record| record.session_id.as_str()),"title":session.title,"agent":session.agent,"model":session.model,"state":session.state,"hosted":host.is_some(),"can_prompt":can_prompt,"last_event_at":session.last_event_at})
    }).collect::<Vec<_>>();
    for record in hosted {
        if !matches!(
            record.agent.as_str(),
            "claude" | "claude-code" | "codex" | "gemini" | "gemini-cli" | "antigravity" | "agy"
        ) {
            continue;
        }
        if !visible
            .iter()
            .any(|row| row["id"].as_str() == Some(&record.session_id))
        {
            let title = Path::new(&record.repo_path).file_name().map_or_else(
                || "New conversation".to_owned(),
                |name| name.to_string_lossy().into_owned(),
            );
            visible.push(json!({"id":record.session_id,"title":title,"agent":record.agent,"requested_model":record.model,"state":"waiting_for_history","hosted":true,"can_prompt":true,"last_event_at":record.started_at}));
        }
    }
    visible.sort_by(|a, b| {
        b["last_event_at"]
            .as_str()
            .cmp(&a["last_event_at"].as_str())
    });
    visible
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

fn remote_prompt_target<'a>(
    id: &str,
    sessions: &[Session],
    hosted: &'a [spawnt_pty::SessionRecord],
) -> Option<&'a spawnt_pty::SessionRecord> {
    let native = conversation_session_id(id, sessions, hosted);
    if let Some(session) = sessions.iter().find(|session| session.id == native) {
        return prompt_host(session, sessions, hosted).filter(|record| {
            matches!(
                record.agent.as_str(),
                "claude"
                    | "claude-code"
                    | "codex"
                    | "gemini"
                    | "gemini-cli"
                    | "antigravity"
                    | "agy"
            )
        });
    }
    hosted.iter().find(|record| {
        record.session_id == id
            && matches!(
                record.agent.as_str(),
                "claude"
                    | "claude-code"
                    | "codex"
                    | "gemini"
                    | "gemini-cli"
                    | "antigravity"
                    | "agy"
            )
    })
}

pub(crate) fn conversation_session_id(
    id: &str,
    sessions: &[Session],
    hosted: &[spawnt_pty::SessionRecord],
) -> String {
    session_bindings(sessions, hosted)
        .into_iter()
        .find(|(_, record)| record.session_id == id)
        .map_or_else(|| id.to_owned(), |(session, _)| session.id.clone())
}

pub(crate) fn session_host<'a>(
    session: &Session,
    sessions: &[Session],
    hosted: &'a [spawnt_pty::SessionRecord],
) -> Option<&'a spawnt_pty::SessionRecord> {
    session_bindings(sessions, hosted)
        .into_iter()
        .find(|(candidate, _)| candidate.id == session.id)
        .map(|(_, record)| record)
}

/// Resolves unique native/hosted pairs with repository discovery once per host.
pub(crate) fn session_bindings<'s, 'h>(
    sessions: &'s [Session],
    hosted: &'h [spawnt_pty::SessionRecord],
) -> Vec<(&'s Session, &'h spawnt_pty::SessionRecord)> {
    let bindings = hosted
        .iter()
        .filter_map(|record| {
            let mut exact = sessions.iter().filter(|candidate| {
                Some(record.child_pid) == candidate.pid || record.session_id == candidate.id
            });
            if let Some(candidate) = exact.next() {
                return exact.next().is_none().then_some((candidate, record));
            }
            let agent = match record.agent.as_str() {
                "claude" => "claude-code",
                "gemini" => "gemini-cli",
                "agy" => "antigravity",
                other => other,
            };
            let path = Path::new(&record.repo_path);
            let (root, remote) = spawnt_platform::resolve_git_repo_and_remote(path);
            let root = root.as_deref().unwrap_or(path);
            let project = Project::from_origin(remote.as_deref(), &root.to_string_lossy(), None);
            let started = chrono::DateTime::parse_from_rfc3339(&record.started_at).ok()?;
            let mut candidates = sessions.iter().filter(|candidate| {
                !hosted.iter().any(|other| {
                    other.session_id != record.session_id
                        && (Some(other.child_pid) == candidate.pid
                            || other.session_id == candidate.id)
                }) && agent == candidate.agent.as_str()
                    && project.id == candidate.project_id
                    && {
                        let delta = candidate.started_at.signed_duration_since(started);
                        delta >= chrono::Duration::seconds(-5)
                            && delta <= chrono::Duration::minutes(5)
                    }
            });
            let candidate = candidates.next()?;
            candidates.next().is_none().then_some((candidate, record))
        })
        .collect::<Vec<_>>();
    bindings
        .iter()
        .filter(|(session, _)| {
            bindings
                .iter()
                .filter(|(candidate, _)| candidate.id == session.id)
                .count()
                == 1
        })
        .copied()
        .collect()
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
        let record = spawnt_pty::SessionRecord {
            session_id: "first-turn".into(),
            agent: "codex".into(),
            repo_path: std::env::temp_dir().to_string_lossy().into_owned(),
            model: None,
            host_pid: 1,
            child_pid: 2,
            port: 1,
            token: "test".into(),
            started_at: now.to_rfc3339(),
        };
        assert!(remote_prompt_target("first-turn", &[], std::slice::from_ref(&record)).is_some());
        assert_eq!(
            remote_sessions(&[], std::slice::from_ref(&record))[0]["can_prompt"],
            true
        );
        let shell = spawnt_pty::SessionRecord {
            agent: "terminal".into(),
            ..record.clone()
        };
        assert!(remote_prompt_target("first-turn", &[], &[shell]).is_none());
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
        let visible = remote_sessions(
            std::slice::from_ref(&session),
            std::slice::from_ref(&record),
        );
        assert_eq!(visible.len(), 1, "a hosted session must not appear twice");
        assert_eq!(visible[0]["id"], record.session_id);
        assert_eq!(visible[0]["can_prompt"], true);
        let mut exact = session.clone();
        exact.pid = Some(record.child_pid);
        let mut unbound = record.clone();
        unbound.session_id = "new-unbound-host".into();
        unbound.child_pid += 1;
        assert_eq!(
            session_host(
                &exact,
                std::slice::from_ref(&exact),
                &[record.clone(), unbound]
            )
            .map(|record| record.session_id.as_str()),
            Some(record.session_id.as_str()),
            "an exact process binding wins over another terminal waiting for history"
        );
        assert!(prompt_host(&session, &[session.clone(), other], &[record]).is_none());
    }

    #[tokio::test]
    async fn pairing_confirms_both_peers_and_serves_encrypted_sessions() {
        if std::env::var_os("SPAWNT_TEST_ISOLATED_PAIRING").is_none() {
            let home = tempfile::tempdir().expect("isolated pairing home");
            let output =
                std::process::Command::new(std::env::current_exe().expect("test executable"))
                    .args([
                        "--exact",
                        "remote::tests::pairing_confirms_both_peers_and_serves_encrypted_sessions",
                    ])
                    .env("SPAWNT_HOME", home.path())
                    .env("SPAWNT_TEST_ISOLATED_PAIRING", "1")
                    .output()
                    .expect("isolated pairing test");
            assert!(
                output.status.success(),
                "{} {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let port = listener.local_addr().expect("address").port();
        let relay_task = tokio::spawn(async move {
            loop {
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
            }
        });
        let mut manager = Arc::new(RemoteManager::new());
        let db = Arc::new(Mutex::new(Database::open_in_memory().expect("database")));
        // Network verification is explicit; normal test runs stay on loopback.
        let base = std::env::var("SPAWNT_TEST_REMOTE_RELAY_URL")
            .unwrap_or_else(|_| format!("ws://127.0.0.1:{port}/relay"));
        let status = manager
            .start_with_url(db.clone(), base.clone())
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
        let challenge = phone_socket
            .next()
            .await
            .expect("challenge")
            .expect("frame")
            .into_data();
        let challenge: Value = serde_json::from_slice(&challenge).expect("challenge json");
        let context = format!("{room}:{}", challenge["nonce"].as_str().expect("nonce"));
        let mut phone_channel =
            RemoteChannel::new(&phone_keys, desktop_public, &context, PeerRole::Phone)
                .expect("channel");
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
        let trusted = phone_socket
            .next()
            .await
            .expect("trust")
            .expect("frame")
            .into_data();
        let trusted: EncryptedRemoteFrame = serde_json::from_slice(&trusted).expect("trust frame");
        let trusted: Value =
            serde_json::from_slice(&phone_channel.open(&trusted).expect("trust decrypt"))
                .expect("trust chunk");
        let trusted: Value = serde_json::from_str(trusted["data"].as_str().expect("trust data"))
            .expect("trust packet");
        assert_eq!(trusted["type"], "trusted");
        let saved: DeviceTrust = serde_json::from_slice(
            &std::fs::read(crate::remote_trust::path()).expect("private trust file"),
        )
        .expect("trust");
        assert!(saved.valid(chrono::Utc::now().timestamp()));
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
        let folder = tempfile::tempdir().expect("workspace browser root");
        std::fs::create_dir(folder.path().join("sample-workspace")).expect("workspace");
        std::fs::write(folder.path().join("private.txt"), "must not be returned").expect("file");
        let request = phone_channel
            .seal(
                &serde_json::to_vec(&json!({"type":"workspaces","path":folder.path()}))
                    .expect("request"),
            )
            .expect("seal");
        phone_socket
            .send(Message::Binary(
                serde_json::to_vec(&request).expect("frame").into(),
            ))
            .await
            .expect("send folders");
        let bytes = phone_socket
            .next()
            .await
            .expect("folder reply")
            .expect("frame")
            .into_data();
        let frame: EncryptedRemoteFrame =
            serde_json::from_slice(&bytes).expect("encrypted folder reply");
        let chunk: Value =
            serde_json::from_slice(&phone_channel.open(&frame).expect("decrypt folder reply"))
                .expect("chunk");
        let listing: Value =
            serde_json::from_str(chunk["data"].as_str().expect("data")).expect("folders");
        assert_eq!(listing["type"], "workspaces");
        assert_eq!(listing["folders"].as_array().expect("folders").len(), 1);
        assert_eq!(listing["folders"][0]["name"], "sample-workspace");
        assert!(!listing.to_string().contains("private.txt"));
        for restart in [false, true] {
            phone_socket
                .close(None)
                .await
                .expect("phone background close");
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            if restart {
                let old = manager.current.lock().await.take().expect("old room");
                let _ = old.cancel.send(true);
                manager = Arc::new(RemoteManager::new());
                manager
                    .restore(db.clone())
                    .await
                    .expect("restore protected trust");
            }
            tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
            let (mut next, _) =
                tokio_tungstenite::connect_async(format!("{base}/{room}?role=phone"))
                    .await
                    .expect("reconnect phone");
            let keys = RemoteKeyPair::from_private([if restart { 25 } else { 24 }; 32]);
            next.send(Message::Binary(
                serde_json::to_vec(&json!({"type":"hello","public_key":keys.public_key_hex()}))
                    .expect("hello")
                    .into(),
            ))
            .await
            .expect("hello");
            let challenge = next
                .next()
                .await
                .expect("challenge")
                .expect("frame")
                .into_data();
            let challenge: Value = serde_json::from_slice(&challenge).expect("challenge json");
            let fresh_context = format!("{room}:{}", challenge["nonce"].as_str().expect("nonce"));
            assert_ne!(
                fresh_context, context,
                "reconnect must use fresh directional keys"
            );
            let proof_context = format!("{fresh_context}:{}", keys.public_key_hex());
            use hmac::{KeyInit as _, Mac as _};
            let mut proof =
                hmac::Hmac::<sha2::Sha256>::new_from_slice(&saved.secret).expect("proof key");
            proof.update(proof_context.as_bytes());
            let proof = crate::remote_trust::hex(&proof.finalize().into_bytes());
            let mut fresh =
                RemoteChannel::new(&keys, desktop_public, &fresh_context, PeerRole::Phone)
                    .expect("fresh channel");
            let frame = fresh
                .seal(&serde_json::to_vec(&json!({"type":"resume","proof":proof})).expect("proof"))
                .expect("sealed proof");
            next.send(Message::Binary(
                serde_json::to_vec(&frame).expect("frame").into(),
            ))
            .await
            .expect("resume");
            let frame = next
                .next()
                .await
                .expect("resumed")
                .expect("frame")
                .into_data();
            let frame: EncryptedRemoteFrame = serde_json::from_slice(&frame).expect("frame");
            let packet: Value =
                serde_json::from_slice(&fresh.open(&frame).expect("authenticated resumed"))
                    .expect("chunk");
            let packet: Value =
                serde_json::from_str(packet["data"].as_str().expect("data")).expect("packet");
            assert_eq!(packet["type"], "resumed");
            assert!(manager.status().await.paired);
            phone_socket = next;
        }
        let old_room = manager.current.lock().await.clone().expect("room");
        old_room
            .trusted
            .lock()
            .await
            .as_mut()
            .expect("trust")
            .last_activity -= 61;
        let gate = manager.persistence.lock().await;
        let old = old_room.clone();
        let activity = tokio::spawn(async move {
            old.activity().await;
        });
        tokio::task::yield_now().await;
        let revoke_manager = manager.clone();
        let revoke = tokio::spawn(async move { revoke_manager.revoke().await });
        while manager.generation.load(Ordering::Acquire) == old_room.expected_generation {
            tokio::task::yield_now().await;
        }
        drop(gate);
        activity.await.expect("old activity");
        assert!(revoke.await.expect("revoke task").expect("revoke"));
        assert!(
            !crate::remote_trust::path().exists(),
            "a delayed save must not resurrect revoked trust"
        );
        assert!(
            old_room.establish_trust().await.is_err(),
            "a delayed confirmation must not resurrect revoked trust"
        );

        assert!(!manager.status().await.enabled);
        relay_task.abort();
    }

    #[test]
    fn handoff_notice_persists_in_new_conversation_and_deletes_without_resurrection() {
        let db = Database::open_in_memory().expect("db");
        let mut db = db;
        let now = chrono::Utc::now();
        let project = Project::from_origin(None, "fixture", None);
        let sessions = ["source", "new-native"].map(|id| Session {
            id: id.into(),
            agent: spawnt_core::AgentId::Codex,
            surface: spawnt_core::Surface::Cli,
            project_id: project.id.clone(),
            parent_session_id: None,
            model: None,
            title: None,
            state: spawnt_core::SessionState::Idle,
            pid: None,
            started_at: now,
            last_event_at: now,
        });
        let cp = spawnt_store::SourceCheckpoint {
            file_path: "fixture".into(),
            last_read_offset: 0,
            file_size: 0,
            fingerprint: "fixture".into(),
            last_ingested_at: now,
        };
        db.commit_batch(&[project], &sessions, &[], &[], &[], &cp)
            .expect("source metadata");
        let target = spawnt_pty::SessionRecord {
            session_id: "new-host".into(),
            agent: "codex".into(),
            repo_path: "fixture".into(),
            model: Some("gpt-5.6-luna".into()),
            host_pid: 1,
            child_pid: 2,
            port: 1,
            token: "test".into(),
            started_at: chrono::Utc::now().to_rfc3339(),
        };
        record_model_change(&db, "source", &target).expect("persist request notice");
        let rows = conversation_chat(&db, "new-host", "new-native", &[], Some(500), None)
            .expect("bind notice");
        assert_eq!(rows.len(), 1);
        assert!(rows[0].content.contains("gpt-5.6-luna"));
        assert_eq!(
            conversation_chat(&db, "new-host", "new-native", &[], Some(500), None)
                .expect("reload")
                .len(),
            1
        );
        db.delete_session_chat_with_host("new-native", "new-host")
            .expect("delete both identities");
        assert!(
            conversation_chat(&db, "new-host", "new-native", &[], Some(500), None)
                .expect("reload after deletion")
                .is_empty()
        );
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
