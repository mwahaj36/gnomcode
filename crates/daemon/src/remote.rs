//! Opt-in outbound encrypted Remote pairing and bounded chat commands.

use crate::remote_trust::DeviceTrust;
use futures_util::{SinkExt, StreamExt};
use gnomcode_core::remote_crypto::{PeerRole, RemoteChannel, RemoteKeyPair};
use gnomcode_core::{EncryptedRemoteFrame, Project, Session};
use gnomcode_store::Database;
use serde::Serialize;
use serde_json::{Value, json};
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
    #[error("configure GNOMCODE_REMOTE_RELAY_URL before pairing")]
    NotConfigured,
    /// The relay could not be reached.
    #[error("remote relay connection failed")]
    Relay,
    /// No active pairing exists.
    #[error("no active remote pairing")]
    NotPaired,
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
    /// Relay endpoint used by both devices.
    pub relay_url: Option<String>,
    /// Pairing expiration before confirmation.
    pub expires_at: Option<String>,
}

/// One room at a time; replacing or revoking a room closes its connection.
#[derive(Default)]
pub struct RemoteManager {
    current: Mutex<Option<Arc<Room>>>,
    discovery_changed: Arc<tokio::sync::Notify>,
    generation: Arc<AtomicU64>,
    persistence: Arc<Mutex<()>>,
    pub(crate) devices: Arc<crate::account_devices::AccountDevices>,
    trust_path: Option<std::path::PathBuf>,
    enrollment_key: Option<String>,
    controller: std::sync::Weak<crate::account_devices::AccountDevices>,
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
    trust_path: std::path::PathBuf,
    enrollment_key: Option<String>,
    controller: std::sync::Weak<crate::account_devices::AccountDevices>,
}

impl RemoteManager {
    pub(crate) fn for_device(
        path: std::path::PathBuf,
        public_key: String,
        controller: std::sync::Weak<crate::account_devices::AccountDevices>,
    ) -> Self {
        Self {
            trust_path: Some(path),
            enrollment_key: Some(public_key),
            controller,
            ..Self::default()
        }
    }

    fn trust_path(&self) -> std::path::PathBuf {
        self.trust_path
            .clone()
            .unwrap_or_else(crate::remote_trust::path)
    }

    pub(crate) async fn start_device(
        &self,
        db: Arc<Mutex<Database>>,
        room: String,
    ) -> Result<PairingStatus, RemoteError> {
        let generation = self.generation.fetch_add(1, Ordering::AcqRel) + 1;
        let path = self.trust_path();
        let saved = tokio::task::spawn_blocking(move || match std::fs::read(path) {
            Ok(bytes) => serde_json::from_slice::<DeviceTrust>(&bytes)
                .map(Some)
                .map_err(|_| RemoteError::Persistence),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(_) => Err(RemoteError::Persistence),
        })
        .await
        .map_err(|_| RemoteError::Persistence)??;
        let trusted = saved.is_some();
        let identity = if let Some(saved) = saved {
            saved
        } else {
            let mut private = [0; 32];
            let mut secret = [0; 32];
            getrandom::fill(&mut private).map_err(|_| RemoteError::Relay)?;
            getrandom::fill(&mut secret).map_err(|_| RemoteError::Relay)?;
            DeviceTrust {
                room,
                relay: std::env::var("GNOMCODE_REMOTE_RELAY_URL").unwrap_or_else(|_| {
                    "wss://spawnt-remote-relay.mwahaj25.workers.dev/relay".into()
                }),
                private,
                secret,
                last_activity: chrono::Utc::now().timestamp(),
            }
        };
        self.start_record(db, identity, trusted, generation).await
    }

    /// Creates an empty Remote manager.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(test)]
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
            clear_trust_at(self.trust_path()).await?;
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
        let path = self.trust_path();
        let record = tokio::task::spawn_blocking(move || {
            std::fs::read(path)
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
            trust_path: self.trust_path(),
            enrollment_key: self.enrollment_key.clone(),
            controller: self.controller.clone(),
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
                relay_url: None,
                expires_at: None,
            },
        }
    }

    /// Whether a remembered phone could answer right now: some room is trusted and the
    /// desktop is connected to the relay (the phone itself may not have opened the page).
    pub(crate) async fn reachable(&self) -> bool {
        self.reachable_room_only().await || self.devices.any_reachable().await
    }

    pub(crate) async fn reachable_room_only(&self) -> bool {
        let room = self.current.lock().await.clone();
        match room {
            Some(room) => room.is_reachable().await,
            None => false,
        }
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
        clear_trust_at(self.trust_path()).await?;
        Ok(previous.is_some())
    }

    pub(crate) async fn suspend(&self) {
        self.generation.fetch_add(1, Ordering::AcqRel);
        if let Some(room) = self.current.lock().await.take() {
            let _ = room.cancel.send(true);
        }
        // Wait for any earlier write before a replacement manager reads this credential.
        let _gate = self.persistence.lock().await;
    }
}

impl Room {
    async fn is_reachable(&self) -> bool {
        self.connected.load(Ordering::Acquire)
            && self
                .trusted
                .lock()
                .await
                .as_ref()
                .is_some_and(|trust| trust.valid(chrono::Utc::now().timestamp()))
    }

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
            let path = self.trust_path.clone();
            tokio::task::spawn_blocking(move || saved.save_at(&path))
                .await
                .map_err(|_| RemoteError::Persistence)??;
            *trusted = Some(trust.clone());
            self.outgoing.send(json!({"type":"trusted","secret":crate::remote_trust::hex(&trust.secret),"expires_at":null})).await.map_err(|_|RemoteError::Relay)?;
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
                let path = self.trust_path.clone();
                let _ = tokio::task::spawn_blocking(move || saved.save_at(&path)).await;
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
        PairingStatus {
            remembered,
            trust_expires_at: None,
            enabled: connected || remembered,
            paired: connected
                && remembered
                && self.local_confirmed.load(Ordering::Acquire)
                && self.phone_confirmed.load(Ordering::Acquire),
            room_id: Some(self.id.clone()),
            public_key: Some(self.public_key.clone()),
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

async fn clear_trust_at(path: std::path::PathBuf) -> Result<(), RemoteError> {
    tokio::task::spawn_blocking(move || match std::fs::remove_file(path) {
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
                    if room.trusted.lock().await.is_none() && room.enrollment_key.as_ref().is_some_and(|key|key!=&public_hex) {break;}
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
                if value.get("type").and_then(Value::as_str)==Some("enroll") {
                    if room.enrollment_key.is_none() || room.trusted.lock().await.is_some() {break;}
                    room.local_confirmed.store(true,Ordering::Release);
                    room.phone_confirmed.store(true,Ordering::Release);
                    if room.establish_trust().await.is_err(){break;}
                    continue;
                }
                if value.get("type").and_then(Value::as_str)==Some("resume") {
                    let trusted=room.trusted.lock().await;
                    if !trusted.as_ref().is_some_and(|trust|trust.valid(chrono::Utc::now().timestamp()) && trust.verify(&resume_context,value.get("proof").and_then(Value::as_str).unwrap_or(""))) {drop(trusted);let _=room.outgoing.send(json!({"type":"resume_rejected"})).await;continue;}
                    drop(trusted);
                    room.local_confirmed.store(true,Ordering::Release);room.phone_confirmed.store(true,Ordering::Release);room.discovery_changed.notify_one();
                    let _=room.outgoing.send(json!({"type":"resumed"})).await;continue;
                }
                if !(room.local_confirmed.load(Ordering::Acquire)
                    && room.phone_confirmed.load(Ordering::Acquire)) { break; }
                if !room.trusted.lock().await.as_ref().is_some_and(|trust|trust.valid(chrono::Utc::now().timestamp())) {break;}
                let request_type = value.get("type").cloned().unwrap_or(Value::Null);
                if request_type.as_str()==Some("interrupt") {room.activity().await;}
                if matches!(request_type.as_str(),Some("activity"|"prompt"|"launch"|"switch"|"workspaces"|"schedule"|"cancel_schedule"|"resume_on_reset"|"approval"|"repo_files"|"repo_file"|"changes"|"artifacts"|"upload_attachment"|"quick_actions"|"continue_here"|"worktree_merge"|"worktree_discard"|"worktree_open_pr"|"approvals")) {room.activity().await;}
                if request_type.as_str()==Some("activity") {continue;}
                let mut response = if matches!(request_type.as_str(),Some("devices")) {
                    if let Some(controller)=room.controller.upgrade() { controller.command(value).await } else {json!({"type":"error","message":"Update your desktop to manage devices."})}
                } else {dispatch(db, value).await};
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

pub(crate) async fn dispatch(db: &Arc<Mutex<Database>>, request: Value) -> Value {
    match request.get("type").and_then(Value::as_str) {
        Some("models") => {
            json!({"type":"models","providers":crate::model_catalog::discover().await})
        }
        // "Run in" picker on the phone (ADR 0060): registry plus running list, never boots.
        Some("wsl_distros") => {
            let distros = tokio::task::spawn_blocking(|| {
                let running = gnomcode_platform::wsl::running_distros();
                gnomcode_platform::wsl::registered_distros()
                    .into_iter()
                    .map(|name| {
                        let up = running.iter().any(|r| r.eq_ignore_ascii_case(&name));
                        json!({"name": name, "label": gnomcode_core::wsl::distro_label(&name), "running": up})
                    })
                    .collect::<Vec<Value>>()
            })
            .await
            .unwrap_or_default();
            json!({"type":"wsl_distros","distros":distros})
        }
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
                gnomcode_platform::workspaces::browse(path.as_deref())
            })
            .await
            {
                Ok(Ok(listing)) => {
                    let display = gnomcode_platform::workspaces::display_path;
                    json!({"type":"workspaces","path":display(&listing.path),"parent":listing.parent.as_deref().map(display),"roots":listing.roots.iter().map(|path| display(path)).collect::<Vec<_>>(),"folders":listing.folders.iter().map(|path| json!({"path":display(path),"name":path.file_name().unwrap_or_default().to_string_lossy()})).collect::<Vec<_>>(),"truncated":listing.truncated})
                }
                _ => {
                    json!({"type":"error","message":"Cannot open that desktop folder. Choose another folder or enter its path."})
                }
            }
        }
        Some("sessions") => {
            let result = db.lock().await.list_active_sessions(usize::MAX);
            match result {
                Ok(sessions) => {
                    let hosted = gnomcode_pty::list_active_sessions();
                    let guard = db.lock().await;
                    let mut visible = match conversation_sessions(&guard, &sessions, &hosted) {
                        Ok(rows) => rows,
                        Err(_) => {
                            return json!({"type":"error","message":"Could not load conversations."});
                        }
                    };
                    for row in &mut visible {
                        if row["title"].as_str().is_none_or(str::is_empty) {
                            let id = row["id"].as_str().unwrap_or_default();
                            let source = guard
                                .conversation_source(id)
                                .unwrap_or_else(|_| id.to_owned());
                            let native = source.as_str();
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
                    nest_subagents(&mut visible);
                    let mut page = conversation_page(visible, &request);
                    page["type"] = json!("sessions");
                    page
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
                "claude" | "claude-code" | "codex" | "antigravity" | "agy"
            ) {
                return json!({"type":"error","message":"Remote launch supports agent CLIs only"});
            }
            let Some(repo_path) = request.get("repo_path").and_then(Value::as_str) else {
                return json!({"type":"error","message":"Missing repo_path"});
            };
            let model = request.get("model").and_then(Value::as_str);
            let repo = Path::new(repo_path);
            let starting = match crate::wsl_launch::preflight(agent, repo).await {
                Ok(starting) => starting,
                Err(crate::wsl_launch::Refusal::BadFolder) => {
                    return json!({"type":"error","message":"Invalid workspace directory"});
                }
                Err(crate::wsl_launch::Refusal::Missing(message)) => {
                    return json!({"type":"error","message":message});
                }
            };
            let session_id = format!(
                "{}-{}",
                agent,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or(0)
            );
            let isolated = match crate::worktree_api::prepare_launch(
                repo,
                &session_id,
                request.get("worktree").and_then(Value::as_bool) == Some(true),
                request.get("worktree_branch").and_then(Value::as_str),
            )
            .await
            {
                Ok(record) => record,
                Err(error) => return json!({"type":"error","message":error.to_string()}),
            };
            let cwd = isolated
                .as_ref()
                .map_or_else(|| repo.to_path_buf(), |record| record.cwd.clone().into());
            match gnomcode_pty::launch_session(&session_id, agent, &cwd, model).await {
                Ok(record) => {
                    if db
                        .lock()
                        .await
                        .save_conversation_launch(
                            &record.session_id,
                            &record.agent,
                            &record.repo_path,
                            record.model.as_deref(),
                        )
                        .is_err()
                    {
                        return json!({"type":"error","message":"Agent started but its conversation could not be saved. Check your laptop before retrying."});
                    }
                    let mut ack = json!({"type":"launch_ack","session_id":record.session_id,"agent":record.agent});
                    crate::wsl_launch::annotate(&mut ack, repo, starting);
                    if let (Some(isolated), Some(fields)) = (&isolated, ack.as_object_mut())
                        && let Value::Object(extra) = crate::worktree_api::fields(isolated)
                    {
                        fields.extend(extra);
                    }
                    ack
                }
                Err(err) => {
                    if let Some(isolated) = &isolated {
                        crate::worktree_api::abort_launch(isolated).await;
                    }
                    json!({"type":"error","message":format!("Launch failed: {err}")})
                }
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
            let sessions = db.list_active_sessions(usize::MAX).unwrap_or_default();
            let native_id = conversation_session_id(
                session_id,
                &sessions,
                &gnomcode_pty::list_active_sessions(),
            );
            match conversation_chat(
                &db,
                session_id,
                &native_id,
                &sessions,
                Some(limit as usize),
                before,
            ) {
                Ok(events) => {
                    let events = gnomcode_adapters::codex::clean_stored_conversation(
                        gnomcode_adapters::antigravity::clean_stored_conversation(events),
                    );
                    json!({"type":"chat","session_id":session_id,"before":before,"events":events})
                }
                Err(_) => json!({"type":"error","message":"Could not load chat"}),
            }
        }
        Some(
            "schedule" | "schedules" | "cancel_schedule" | "notices" | "routing"
            | "resume_on_reset",
        ) => crate::automation::command(db, request.clone()).await,
        Some("approvals" | "approval") => crate::approvals::remote_command(&request),
        Some("repo_files" | "repo_file") => {
            crate::workspace_files::remote_command(db, &request).await
        }
        Some("changes") => crate::session_changes::remote_command(db, &request).await,
        Some("artifacts") => crate::session_artifacts::remote_command(db, &request).await,
        Some("upload_attachment") => crate::attachments::remote_command(db, &request).await,
        Some("quick_actions") => crate::quick_actions::remote_command(&request).await,
        Some("features.get" | "features.set") => crate::features::handle_remote(db, &request).await,
        Some("prompt") => {
            let Some(session_id) = request.get("session_id").and_then(Value::as_str) else {
                return json!({"type":"error","message":"Missing session"});
            };
            let Some(text) = request.get("text").and_then(Value::as_str) else {
                return json!({"type":"error","message":"Missing prompt"});
            };
            if text.trim().is_empty()
                || text.encode_utf16().count() > 4000
                || text
                    .chars()
                    .any(|character| character.is_control() && !matches!(character, '\n' | '\t'))
                || text.trim_start().starts_with('/')
            {
                return json!({"type":"error","message":"Invalid prompt"});
            }
            match crate::deliver_conversation_prompt(db, session_id, text).await {
                Ok(()) => json!({"type":"prompt_ack","session_id":session_id}),
                Err(
                    error @ (crate::HandoffError::Busy
                    | crate::HandoffError::NotReady
                    | crate::HandoffError::ImportedSession),
                ) => json!({"type":"error","message":error.to_string()}),
                Err(_) => {
                    json!({"type":"error","message":"Delivery could not be confirmed. Check the conversation before retrying; your draft is saved."})
                }
            }
        }
        Some("continue_here") => crate::continue_here::remote_command(db, &request).await,
        Some("worktree_merge" | "worktree_discard" | "worktree_open_pr" | "session_changes") => {
            crate::worktree_api::remote_command(db, &request).await
        }
        Some("resume_session") => {
            let Some(id) = request
                .get("session_id")
                .and_then(Value::as_str)
                .filter(|id| id.len() <= 128)
            else {
                return json!({"type":"error","message":"Choose a conversation to resume."});
            };
            match crate::resume_conversation(db, id).await {
                Ok(_) => json!({"type":"resume_session_ack","session_id":id}),
                Err(_) => {
                    json!({"type":"error","message":"Could not resume this conversation. Check the agent on your laptop and try again."})
                }
            }
        }
        Some("switch") => {
            let _control = crate::CONVERSATION_CONTROL.lock().await;
            let Some(session_id) = request.get("session_id").and_then(Value::as_str) else {
                return json!({"type":"error","message":"Missing session"});
            };
            let Some(model) = request.get("model").and_then(Value::as_str) else {
                return json!({"type":"error","message":"Missing model"});
            };
            if session_id.len() > 128 || model.len() > 128 {
                return json!({"type":"error","message":"Invalid switch request"});
            }
            let agent = request.get("agent").and_then(Value::as_str);
            let prepared = crate::prepare_model_change(db, session_id, model, agent).await;
            let Ok(prepared) = prepared else {
                return json!({"type":"error","message":match prepared {Err(crate::HandoffError::Busy)=>"Wait for the current response or approval to finish before changing models.",Err(crate::HandoffError::NotReady)=>"The agent is starting or needs native approval. Finish setup on your laptop, then retry.",Err(crate::HandoffError::ImportedSession)=>"This imported session is view-only. Start a Gnomcode conversation to change models.",_=>"Could not prepare model switch. Check the selected agent and workspace on your desktop."}});
            };
            match crate::execute_switch(prepared).await {
                Ok(record) => {
                    let guard = db.lock().await;
                    if record_model_change(&guard, session_id, &record).is_err() {
                        return json!({"type":"error","message":"Could not save the model change. Reload and try again."});
                    }
                    json!({"type":"switch_ack","session_id":session_id,"new_session_id":session_id})
                }
                Err(_) => json!({"type":"error","message":"Model switch failed"}),
            }
        }
        Some("interrupt") => crate::interrupt::command(db, request.clone()).await,
        _ => json!({"type":"error","message":"Unsupported remote command"}),
    }
}

pub(crate) fn record_model_change(
    db: &Database,
    source: &str,
    target: &gnomcode_pty::SessionRecord,
) -> Result<(), gnomcode_store::Error> {
    let sessions = db.list_active_sessions(usize::MAX)?;
    sync_conversation_bindings(db, &sessions, &gnomcode_pty::list_active_sessions())?;
    let previous_native = db
        .conversation_target(source)?
        .and_then(|(_, native, _)| native);
    db.disarm_resume_queue(source)?;
    let same_host = db
        .conversation_target(source)?
        .is_some_and(|(host, _, _)| host == target.session_id);
    if !same_host {
        db.continue_conversation(source, &target.session_id)?;
    }
    db.save_conversation_launch(
        &target.session_id,
        &target.agent,
        &target.repo_path,
        target.model.as_deref(),
    )?;
    let source = previous_native.unwrap_or(db.conversation_source(source)?);
    let last = db.get_session_chat(&source, Some(1))?;
    let event = gnomcode_core::ChatEvent {
        event_id: format!(
            "handoff:{}:{}",
            target.session_id,
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ),
        session_id: source,
        turn_index: last
            .last()
            .map_or(0, |event| event.turn_index.saturating_add(1)),
        role: gnomcode_core::ChatRole::System,
        content: format!(
            "Switching to {}...",
            target.model.as_deref().unwrap_or("agent default")
        ),
        created_at: chrono::Utc::now(),
        tool_name: None,
        status: Some("model_changed".into()),
    };
    db.insert_chat_event(&event)?;
    db.automation_notice(
        &event.event_id,
        &event.session_id,
        "Model switch requested. Activation is pending agent confirmation.",
    )?;
    Ok(())
}

pub(crate) fn sync_conversation_bindings(
    db: &Database,
    sessions: &[Session],
    hosted: &[gnomcode_pty::SessionRecord],
) -> Result<(), gnomcode_store::Error> {
    for record in hosted
        .iter()
        .filter(|record| crate::agent_for_model("@default", Some(&record.agent)).is_some())
    {
        if db.conversation_launch(&record.session_id)?.is_none() {
            db.save_conversation_launch(
                &record.session_id,
                &record.agent,
                &record.repo_path,
                record.model.as_deref(),
            )?;
        }
    }
    for (session, record) in session_bindings(sessions, hosted) {
        db.bind_conversation_host(&record.session_id, &session.id)?;
    }
    Ok(())
}

pub(crate) fn active_conversation_id(
    db: &Database,
    id: &str,
    sessions: &[Session],
    hosted: &[gnomcode_pty::SessionRecord],
) -> Result<String, gnomcode_store::Error> {
    sync_conversation_bindings(db, sessions, hosted)?;
    Ok(db.conversation_target(id)?.map_or_else(
        || conversation_session_id(id, sessions, hosted),
        |(host, native, _)| native.unwrap_or(host),
    ))
}

pub(crate) fn conversation_host<'a>(
    db: &Database,
    id: &str,
    sessions: &[Session],
    hosted: &'a [gnomcode_pty::SessionRecord],
) -> Result<Option<&'a gnomcode_pty::SessionRecord>, gnomcode_store::Error> {
    let record = if let Some((host, _, _)) = db.conversation_target(id)? {
        hosted.iter().find(|record| record.session_id == host)
    } else {
        hosted
            .iter()
            .find(|record| record.session_id == id)
            .or_else(|| {
                sessions
                    .iter()
                    .find(|session| session.id == id)
                    .and_then(|session| session_host(session, sessions, hosted))
            })
    };
    Ok(record.filter(|record| crate::agent_for_model("@default", Some(&record.agent)).is_some()))
}

pub(crate) fn conversation_chat(
    db: &Database,
    requested: &str,
    native: &str,
    sessions: &[Session],
    limit: Option<usize>,
    before: Option<&str>,
) -> Result<Vec<gnomcode_core::ChatEvent>, gnomcode_store::Error> {
    sync_conversation_bindings(db, sessions, &gnomcode_pty::list_active_sessions())?;
    if requested != native && db.conversation_target(requested)?.is_some() {
        // History can arrive after the new host was linked, including after restart.
        let target = db.conversation_target(requested)?;
        if let Some((host, None, _)) = target
            && host == requested
        {
            db.bind_conversation_host(&host, native)?;
        }
    }
    let id = if db.conversation_target(requested)?.is_some() {
        requested
    } else {
        native
    };
    let mut events = db.conversation_chat(id, limit, before)?;
    for event in &mut events {
        if event.role == gnomcode_core::ChatRole::User
            && let Some((_, message)) = event
                .content
                .strip_prefix("<spawnt_context>")
                .and_then(|text| text.split_once("</spawnt_context>"))
        {
            event.content = message.trim_start().to_owned();
        }
    }
    Ok(events)
}

/// Bounded recent history plus the first retained request, fetched independently of pagination.
pub(crate) fn conversation_handoff_chat(
    db: &Database,
    requested: &str,
    native: &str,
    sessions: &[Session],
) -> Result<(Vec<gnomcode_core::ChatEvent>, Option<String>), gnomcode_store::Error> {
    let mut events = conversation_chat(
        db,
        requested,
        native,
        sessions,
        Some(crate::HANDOFF_CONTEXT_EVENTS),
        None,
    )?;
    let id = if db.conversation_target(requested)?.is_some() {
        requested
    } else {
        native
    };
    if let Some(mut first) = db.conversation_first_user_event(id)?
        && !events.iter().any(|event| event.event_id == first.event_id)
    {
        if let Some((_, message)) = first
            .content
            .strip_prefix("<spawnt_context>")
            .and_then(|text| text.split_once("</spawnt_context>"))
        {
            first.content = message.trim_start().to_owned();
        }
        events.insert(0, first);
    }
    Ok((events, db.conversation_title(id)?))
}

fn legacy_tool_session(id: &str) -> bool {
    id.strip_prefix("codex:").is_some_and(|id| {
        ["ctc_", "ctco_", "fc_", "call_"]
            .iter()
            .any(|prefix| id.starts_with(prefix))
    })
}

/// Maximum nested subagents listed per conversation; `subagent_count` still counts them all.
const NESTED_SUBAGENT_LIMIT: usize = 50;

/// Nests each subagent row under its parent as `subagents`, `subagent_count` and
/// `subagents_ongoing`. A subagent is ongoing while its displayed state is a running one.
/// Each child gets a Rust-derived `name`, and a parent that is not itself busy reports
/// `state: "thinking"` with a `state_label` such as `Working · 5 subagents running`.
pub(crate) fn nest_subagents(rows: &mut [Value]) {
    let mut children: std::collections::HashMap<String, Vec<Value>> =
        std::collections::HashMap::new();
    for row in rows.iter() {
        let Some(parent) = row["parent_session_id"].as_str() else {
            continue;
        };
        let ongoing = matches!(
            row["state"].as_str(),
            Some("thinking" | "tool" | "awaiting_approval")
        );
        children.entry(parent.to_owned()).or_default().push(json!({
            "id": row["id"], "title": row["title"], "model": row["model"], "state": row["state"],
            "ongoing": ongoing, "started_at": row["started_at"], "last_event_at": row["last_event_at"],
        }));
    }
    for row in rows.iter_mut() {
        let Some(mut nested) = row["id"].as_str().and_then(|id| children.remove(id)) else {
            continue;
        };
        // Ordinals follow start order so `Subagent N` stays stable as new children appear.
        nested.sort_by(|a, b| {
            a["started_at"]
                .as_str()
                .cmp(&b["started_at"].as_str())
                .then_with(|| a["id"].as_str().cmp(&b["id"].as_str()))
        });
        for (index, child) in nested.iter_mut().enumerate() {
            child["name"] = json!(gnomcode_core::subagent::subagent_name(
                child["title"].as_str(),
                index + 1
            ));
        }
        nested.sort_by(|a, b| {
            b["last_event_at"]
                .as_str()
                .cmp(&a["last_event_at"].as_str())
        });
        let running = nested
            .iter()
            .filter(|child| child["ongoing"] == true)
            .count();
        row["subagent_count"] = json!(nested.len());
        row["subagents_ongoing"] = json!(running);
        if running > 0 && row["state"] != "awaiting_approval" {
            row["state"] = json!("thinking");
            row["state_label"] = json!(gnomcode_core::subagent::working_label(running));
            // Inferred from subagent activity: the parent's own log is quiet.
            row["live"] = json!(gnomcode_core::presence::LiveStatus {
                state: gnomcode_core::presence::LiveState::Working,
                basis: gnomcode_core::TrustLevel::Estimated,
                step: "Running a subagent".to_owned(),
            });
        }
        nested.truncate(NESTED_SUBAGENT_LIMIT);
        row["subagents"] = json!(nested);
    }
}

pub(crate) fn conversation_page(mut rows: Vec<Value>, query: &Value) -> Value {
    let search = query["search"].as_str().unwrap_or("").trim().to_lowercase();
    let include_subagents = query["include_subagents"].as_bool().unwrap_or(false);
    // A child already nested under a listed parent is not repeated as its own top-level row.
    let listed: std::collections::HashSet<String> = rows
        .iter()
        .filter_map(|row| row["id"].as_str().map(str::to_owned))
        .collect();
    rows.retain(|row| {
        (row["parent_session_id"].is_null()
            || (include_subagents
                && row["parent_session_id"]
                    .as_str()
                    .is_none_or(|parent| !listed.contains(parent))))
            && query["source"]
                .as_str()
                .filter(|value| !value.is_empty() && *value != "all")
                .is_none_or(|source| row["source"] == source)
            && query["agent"]
                .as_str()
                .filter(|value| !value.is_empty() && *value != "all")
                .is_none_or(|agent| row["agent"] == agent)
            && query["project"]
                .as_str()
                .filter(|value| !value.is_empty() && *value != "all")
                .is_none_or(|project| row["project_id"] == project)
            && query["status"]
                .as_str()
                .filter(|value| !value.is_empty() && *value != "all")
                .is_none_or(|status| match status {
                    "active" => {
                        row["subagents_ongoing"].as_u64().unwrap_or(0) > 0
                            || matches!(
                                row["state"].as_str(),
                                Some("thinking" | "tool" | "awaiting_approval")
                            )
                    }
                    "ongoing" => {
                        row["hosted"] == true
                            || row["subagents_ongoing"].as_u64().unwrap_or(0) > 0
                            || matches!(
                                row["state"].as_str(),
                                Some("thinking" | "tool" | "awaiting_approval")
                            )
                    }
                    _ => row["state"] == status,
                })
            && (search.is_empty()
                || format!(
                    "{} {} {}",
                    row["title"].as_str().unwrap_or(""),
                    row["agent"].as_str().unwrap_or(""),
                    row["project_name"].as_str().unwrap_or("")
                )
                .to_lowercase()
                .contains(&search))
    });
    if !include_subagents {
        // Unchecked hides subagent rows everywhere; a parent keeps only its count.
        for row in &mut rows {
            if let Some(fields) = row.as_object_mut() {
                fields.remove("subagents");
            }
        }
    }
    rows.sort_by(|a, b| {
        b["last_event_at"]
            .as_str()
            .cmp(&a["last_event_at"].as_str())
            .then_with(|| a["id"].as_str().cmp(&b["id"].as_str()))
    });
    let total = rows.len();
    let limit = query["limit"].as_u64().unwrap_or(50).clamp(1, 100) as usize;
    let pages = total.div_ceil(limit).max(1);
    let page = (query["page"].as_u64().unwrap_or(1) as usize).clamp(1, pages);
    let rows: Vec<_> = rows
        .into_iter()
        .skip((page - 1) * limit)
        .take(limit)
        .collect();
    json!({"sessions":rows,"page":page,"total_pages":pages,"total_count":total})
}

fn remote_sessions(sessions: &[Session], hosted: &[gnomcode_pty::SessionRecord]) -> Vec<Value> {
    let mut visible = sessions.iter().filter(|session| !legacy_tool_session(&session.id)).map(|session| {
        json!({"id":session.id,"title":session.title,"agent":session.agent,"model":session.model,"state":session.state,"started_at":session.started_at,"last_event_at":session.last_event_at,"parent_session_id":session.parent_session_id,"project_id":session.project_id})
    }).collect::<Vec<_>>();
    for record in hosted {
        if !matches!(
            record.agent.as_str(),
            "claude" | "claude-code" | "codex" | "antigravity" | "agy"
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

/// Live hosts report their own state; observed logs go through the shared presence rules.
fn displayed_state(session: &Session, hosted: bool) -> &'static str {
    let state = session.state.as_str();
    let age = (chrono::Utc::now() - session.last_event_at).num_milliseconds();
    let stale = gnomcode_core::presence::classify_presence(state, Some(age), false)
        == gnomcode_core::presence::SessionPresence::Inactive;
    if !hosted && stale && matches!(state, "thinking" | "tool" | "awaiting_approval") {
        "idle"
    } else {
        state
    }
}

pub(crate) fn conversation_sessions(
    db: &Database,
    sessions: &[Session],
    hosted: &[gnomcode_pty::SessionRecord],
) -> Result<Vec<Value>, gnomcode_store::Error> {
    sync_conversation_bindings(db, sessions, hosted)?;
    let quota = db.get_latest_quota_snapshots()?;
    let worktrees = crate::worktree_api::index();
    let mut visible = Vec::new();
    let mut candidates = remote_sessions(sessions, hosted);
    for root in db.conversation_ids()? {
        let target = db.conversation_target(&root)?;
        if let Some((host, None, _)) = target {
            if candidates.iter().any(|row| row["id"] == host) {
                continue;
            }
            if let Some(launch) = db.conversation_launch(&root)? {
                candidates.push(json!({"id":host,"agent":launch.agent,"title":crate::conversation_title(db,&root),"state":"ended","can_prompt":false,"hosted":false}));
            }
        }
    }
    for mut row in candidates {
        let row_id = row["id"].as_str().unwrap_or_default().to_owned();
        let id = row_id.as_str();
        let mapping = db.conversation_target(id)?;
        if mapping
            .as_ref()
            .is_some_and(|(host, native, _)| host != id && native.as_deref() != Some(id))
        {
            continue;
        }
        let root = db.conversation_id(id)?;
        if let Some(source) = sessions.iter().find(|session| {
            session.id == db.conversation_source(id).unwrap_or_else(|_| id.to_owned())
        }) {
            row["parent_session_id"] = json!(source.parent_session_id);
            row["project_id"] = json!(source.project_id);
        }
        let host = conversation_host(db, id, sessions, hosted)?;
        let native = mapping
            .as_ref()
            .and_then(|(_, native, _)| native.as_deref())
            .unwrap_or(id);
        let observed = sessions.iter().find(|session| session.id == native);
        let launch = db.conversation_launch(id)?;
        let agent = host
            .map(|record| record.agent.as_str())
            .or_else(|| observed.map(|session| session.agent.as_str()))
            .or_else(|| launch.as_ref().map(|launch| launch.agent.as_str()))
            .unwrap_or_default();
        let managed = host.is_some() || launch.is_some();
        row["source"] = json!(if managed { "managed" } else { "imported" });
        let can_resume = managed
            && host.is_none()
            && (matches!(agent, "codex" | "claude-code" | "claude" | "antigravity")
                || observed.is_none()
                    && launch.as_ref().is_some_and(|launch| {
                        matches!(
                            launch.agent.as_str(),
                            "codex" | "claude-code" | "antigravity"
                        )
                    }));
        let can_prompt = host.is_some_and(|record| {
            observed.is_none_or(|session| {
                session.state == gnomcode_core::SessionState::Idle
                    || (session.state == gnomcode_core::SessionState::Error
                        && db
                            .quota_reset_ready(&session.id, session.last_event_at)
                            .unwrap_or(false))
                    || chrono::DateTime::parse_from_rfc3339(&record.started_at)
                        .is_ok_and(|started| session.last_event_at < started)
            })
        });
        let limits = crate::api::conversation_quota(&quota, agent, chrono::Utc::now());
        row["agent"] = json!(agent);
        if let Some(label) = observed
            .and_then(|session| gnomcode_core::surface_label(agent, session.surface.as_str()))
        {
            row["surface_label"] = json!(label);
        }
        if let Some(session) = observed {
            row["model"] = json!(session.model);
            row["state"] = json!(displayed_state(session, host.is_some()));
        }
        row["id"] = json!(root);
        if let Some(record) = worktrees.get(&root).or_else(|| worktrees.get(id))
            && let (Value::Object(fields), Value::Object(extra)) =
                (&mut row, crate::worktree_api::fields(record))
        {
            fields.extend(extra);
        }
        row["hosted"] = json!(host.is_some());
        if let Some(session) = observed {
            let (kind, tool, turn) = db.live_inputs(&session.id)?;
            let state = session.state.as_str();
            let open_execution = matches!(
                (state, kind.as_deref()),
                ("thinking", Some("prompt")) | ("tool", Some("tool_start"))
            );
            let age = (chrono::Utc::now() - session.last_event_at).num_milliseconds();
            let presence = gnomcode_core::presence::classify_presence_with_host(
                state,
                Some(age),
                open_execution,
                host.is_some(),
            );
            let live = gnomcode_core::presence::live_status(
                presence,
                state,
                host.is_some(),
                host.is_some(),
                tool.as_deref(),
            );
            if live.state == gnomcode_core::presence::LiveState::Working {
                row["turn_started_at"] = json!(turn);
            }
            row["live"] = json!(live);
        }
        row["can_prompt"] = json!(can_prompt);
        row["can_resume"] = json!(can_resume);
        row["can_reset"] = json!(managed && matches!(agent, "codex" | "claude-code"));
        row["limits"] = json!(limits);
        if host.is_some_and(|record| {
            observed.is_none_or(|session| {
                chrono::DateTime::parse_from_rfc3339(&record.started_at)
                    .is_ok_and(|started| session.last_event_at < started)
            })
        }) {
            row["state"] = json!("waiting_for_history");
        }
        if let Some(title) = crate::conversation_title(db, &root) {
            row["title"] = json!(title);
        }
        if row["title"]
            .as_str()
            .is_none_or(|title| title.trim().is_empty() || title.eq_ignore_ascii_case("unknown"))
        {
            let label = db
                .get_session_focus_target(native)?
                .and_then(|target| target.repo_root)
                .and_then(|path| {
                    Path::new(&path)
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                })
                .filter(|name| name != "unknown")
                .unwrap_or_else(|| "Saved conversation".into());
            row["title"] = json!(format!(
                "{label} · {}",
                row["last_event_at"]
                    .as_str()
                    .unwrap_or("")
                    .get(..10)
                    .unwrap_or("")
            ));
        }
        if let Some(model) = launch.and_then(|launch| launch.model)
            && observed.and_then(|session| session.model.as_deref()) != Some(model.as_str())
        {
            row["requested_model"] = json!(model);
        }
        if !visible
            .iter()
            .any(|existing: &Value| existing["id"] == row["id"])
        {
            visible.push(row);
        }
    }
    Ok(visible)
}

pub(crate) fn conversation_session_id(
    id: &str,
    sessions: &[Session],
    hosted: &[gnomcode_pty::SessionRecord],
) -> String {
    session_bindings(sessions, hosted)
        .into_iter()
        .find(|(_, record)| record.session_id == id)
        .map_or_else(|| id.to_owned(), |(session, _)| session.id.clone())
}

pub(crate) fn session_host<'a>(
    session: &Session,
    sessions: &[Session],
    hosted: &'a [gnomcode_pty::SessionRecord],
) -> Option<&'a gnomcode_pty::SessionRecord> {
    session_bindings(sessions, hosted)
        .into_iter()
        .find(|(candidate, _)| candidate.id == session.id)
        .map(|(_, record)| record)
}

/// Returns native session IDs whose Gnomcode PTY host process is currently alive.
pub(crate) fn live_hosted_native_ids(db: &Database) -> Vec<String> {
    let hosted = gnomcode_pty::try_list_active_sessions().unwrap_or_default();
    if hosted.is_empty() {
        return Vec::new();
    }
    let host_ids: Vec<String> = hosted
        .iter()
        .map(|record| record.session_id.clone())
        .collect();
    let mut native_ids = db.native_ids_for_hosts(&host_ids).unwrap_or_default();
    let sessions = db.list_active_sessions(usize::MAX).unwrap_or_default();
    for (session, _) in session_bindings(&sessions, &hosted) {
        if !native_ids.contains(&session.id) {
            native_ids.push(session.id.clone());
        }
    }
    native_ids
}

/// Resolves unique native/hosted pairs with repository discovery once per host.
pub(crate) fn session_bindings<'s, 'h>(
    sessions: &'s [Session],
    hosted: &'h [gnomcode_pty::SessionRecord],
) -> Vec<(&'s Session, &'h gnomcode_pty::SessionRecord)> {
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
                "agy" => "antigravity",
                other => other,
            };
            let path = Path::new(&record.repo_path);
            let (root, remote) = gnomcode_platform::resolve_git_repo_and_remote(path);
            let root = root.as_deref().unwrap_or(path);
            let project = Project::from_origin(remote.as_deref(), &root.to_string_lossy(), None);
            let workspace_project = Project::from_origin(None, &record.repo_path, None);
            let started = chrono::DateTime::parse_from_rfc3339(&record.started_at).ok()?;
            let mut candidates = sessions.iter().filter(|candidate| {
                !hosted.iter().any(|other| {
                    other.session_id != record.session_id
                        && (Some(other.child_pid) == candidate.pid
                            || other.session_id == candidate.id)
                }) && agent == candidate.agent.as_str()
                    && (project.id == candidate.project_id
                        || workspace_project.id == candidate.project_id)
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

    #[test]
    fn stale_observed_execution_is_not_shown_as_running() {
        let mut session = Session {
            id: "antigravity:old".into(),
            agent: gnomcode_core::AgentId::Antigravity,
            surface: gnomcode_core::Surface::Ide,
            project_id: "p".into(),
            parent_session_id: None,
            model: None,
            title: None,
            state: gnomcode_core::SessionState::Tool,
            pid: None,
            started_at: chrono::Utc::now(),
            last_event_at: chrono::Utc::now() - chrono::Duration::hours(13),
        };
        assert_eq!(displayed_state(&session, false), "idle");
        assert_eq!(displayed_state(&session, true), "tool");
        session.last_event_at = chrono::Utc::now();
        assert_eq!(displayed_state(&session, false), "tool");
    }

    #[test]
    fn every_provider_separates_owned_imported_and_subagent_history() {
        let agents = ["claude-code", "codex", "antigravity"];
        let mut rows = Vec::new();
        for agent in agents {
            for source in ["managed", "imported"] {
                rows.push(json!({"id":format!("{agent}-{source}"),"agent":agent,"source":source,"title":agent,"parent_session_id":null,"state":"idle"}));
            }
            rows.push(json!({"id":format!("{agent}-child"),"agent":agent,"source":"imported","title":"Child","parent_session_id":"parent","state":"idle"}));
        }
        for agent in agents {
            let owned = conversation_page(rows.clone(), &json!({"agent":agent,"source":"managed"}));
            assert_eq!(owned["total_count"], 1, "{agent}");
            let imported =
                conversation_page(rows.clone(), &json!({"agent":agent,"source":"imported"}));
            assert_eq!(imported["total_count"], 1, "{agent}");
            let children = conversation_page(
                rows.clone(),
                &json!({"agent":agent,"source":"imported","include_subagents":true}),
            );
            assert_eq!(children["total_count"], 2, "{agent}");
        }
    }

    #[test]
    fn conversation_history_does_not_truncate_at_one_hundred() {
        let now = chrono::Utc::now();
        let sessions: Vec<_> = (0..205)
            .map(|i| Session {
                id: format!("codex:saved-{i}"),
                agent: gnomcode_core::AgentId::Codex,
                surface: gnomcode_core::Surface::Cli,
                project_id: "test".into(),
                parent_session_id: None,
                model: None,
                title: None,
                state: gnomcode_core::SessionState::Idle,
                pid: None,
                started_at: now,
                last_event_at: now,
            })
            .collect();
        assert_eq!(remote_sessions(&sessions, &[]).len(), 205);
    }

    #[test]
    fn hosted_session_binds_to_exact_subdirectory_without_git_origin_in_log() {
        let repo = tempfile::tempdir().expect("repo");
        for args in [
            vec!["init", "--quiet"],
            vec![
                "remote",
                "add",
                "origin",
                "https://github.com/example/fixture.git",
            ],
        ] {
            assert!(
                std::process::Command::new("git")
                    .args(args)
                    .current_dir(repo.path())
                    .status()
                    .expect("git")
                    .success()
            );
        }
        let workspace = repo.path().join("nested");
        std::fs::create_dir(&workspace).expect("workspace");
        let project = Project::from_origin(None, &workspace.to_string_lossy(), None);
        let now = chrono::Utc::now();
        let session = Session {
            id: "codex:native".into(),
            agent: gnomcode_core::AgentId::Codex,
            surface: gnomcode_core::Surface::Cli,
            project_id: project.id,
            parent_session_id: None,
            model: None,
            title: None,
            state: gnomcode_core::SessionState::Idle,
            pid: None,
            started_at: now,
            last_event_at: now,
        };
        let record = gnomcode_pty::SessionRecord {
            session_id: "host".into(),
            agent: "codex".into(),
            repo_path: workspace.to_string_lossy().into_owned(),
            model: None,
            host_pid: 1,
            child_pid: 2,
            port: 1,
            token: "test".into(),
            started_at: now.to_rfc3339(),
        };
        assert_eq!(
            conversation_session_id(
                "host",
                std::slice::from_ref(&session),
                std::slice::from_ref(&record)
            ),
            "codex:native"
        );
        let duplicate = Session {
            id: "codex:ambiguous".into(),
            ..session.clone()
        };
        assert_eq!(
            conversation_session_id("host", &[session, duplicate], &[record]),
            "host",
            "ambiguous native sessions must remain unbound"
        );
    }

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
            agent: gnomcode_core::AgentId::Codex,
            surface: gnomcode_core::Surface::Cli,
            project_id: "test".into(),
            parent_session_id: None,
            model: None,
            title: None,
            state: gnomcode_core::SessionState::Thinking,
            pid: Some(12),
            started_at: now,
            last_event_at: now,
        };
        let record = gnomcode_pty::SessionRecord {
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
        let db = Database::open_in_memory().expect("db");
        assert_eq!(
            conversation_sessions(&db, std::slice::from_ref(&session), &[record])
                .expect("busy conversation")[0]["can_prompt"],
            false
        );
        let record = gnomcode_pty::SessionRecord {
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
        assert_eq!(
            conversation_sessions(&db, &[], std::slice::from_ref(&record))
                .expect("new conversation")[0]["can_prompt"],
            true
        );
        assert_eq!(
            conversation_sessions(&db, &[], std::slice::from_ref(&record)).expect("hosted")[0]["can_prompt"],
            true
        );
        let shell = gnomcode_pty::SessionRecord {
            agent: "terminal".into(),
            ..record.clone()
        };
        assert!(
            conversation_sessions(&db, &[], &[shell])
                .expect("no shells")
                .iter()
                .all(|row| row["agent"] != "terminal")
        );
    }

    #[test]
    fn hosted_prompt_binding_requires_unique_recent_session() {
        let repo = tempfile::tempdir().expect("repo");
        let path = repo.path().to_string_lossy().into_owned();
        let now = chrono::Utc::now();
        let session = Session {
            id: "codex:native".to_owned(),
            agent: gnomcode_core::AgentId::Codex,
            surface: gnomcode_core::Surface::Cli,
            project_id: Project::from_origin(None, &path, None).id,
            parent_session_id: None,
            model: None,
            title: None,
            state: gnomcode_core::SessionState::Idle,
            pid: None,
            started_at: now,
            last_event_at: now,
        };
        let record = gnomcode_pty::SessionRecord {
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
            session_host(
                &session,
                std::slice::from_ref(&session),
                std::slice::from_ref(&record)
            )
            .is_some()
        );
        let mut other = session.clone();
        other.id = "codex:other".to_owned();
        let db = Database::open_in_memory().expect("db");
        let visible = conversation_sessions(
            &db,
            std::slice::from_ref(&session),
            std::slice::from_ref(&record),
        )
        .expect("one conversation");
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
        assert!(session_host(&session, &[session.clone(), other], &[record]).is_none());
    }

    #[tokio::test]
    async fn an_unapproved_key_cannot_receive_an_enrollment_credential() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let address = listener.local_addr().expect("address");
        let relay = tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.expect("desktop");
            let desktop = tokio_tungstenite::accept_async(tcp)
                .await
                .expect("desktop socket");
            let (tcp, _) = listener.accept().await.expect("phone");
            let phone = tokio_tungstenite::accept_async(tcp)
                .await
                .expect("phone socket");
            let (mut desktop_out, mut desktop_in) = desktop.split();
            let (mut phone_out, mut phone_in) = phone.split();
            loop {
                tokio::select! {
                    message=desktop_in.next()=>{let Some(Ok(message))=message else{break;};if phone_out.send(message).await.is_err(){break;}}
                    message=phone_in.next()=>{let Some(Ok(message))=message else{break;};if desktop_out.send(message).await.is_err(){break;}}
                }
            }
        });
        let home = tempfile::tempdir().expect("home");
        let path = home.path().join("trust.json");
        let approved = RemoteKeyPair::from_private([5; 32]);
        let attacker = RemoteKeyPair::from_private([6; 32]);
        let manager = RemoteManager::for_device(
            path.clone(),
            approved.public_key_hex(),
            std::sync::Weak::new(),
        );
        let base = format!("ws://{address}/relay");
        let db = Arc::new(Mutex::new(Database::open_in_memory().expect("database")));
        let status = manager
            .start_with_url(db, base.clone())
            .await
            .expect("approved room");
        let (mut phone, _) = tokio_tungstenite::connect_async(format!(
            "{base}/{}?role=phone",
            status.room_id.expect("room")
        ))
        .await
        .expect("phone");
        phone
            .send(Message::Binary(
                serde_json::to_vec(&json!({"type":"hello","public_key":attacker.public_key_hex()}))
                    .expect("hello")
                    .into(),
            ))
            .await
            .expect("hello");
        let response = tokio::time::timeout(std::time::Duration::from_secs(2), phone.next())
            .await
            .expect("rejected promptly");
        assert!(
            !matches!(response, Some(Ok(Message::Binary(_)))),
            "an unapproved key must receive no handshake or credential"
        );
        assert!(!path.exists(), "unapproved peers must not create trust");
        assert!(!manager.status().await.paired);
        relay.abort();
    }

    #[tokio::test]
    async fn approved_device_enrolls_without_codes_and_resumes_after_restart() {
        if std::env::var_os("GNOMCODE_TEST_ISOLATED_PAIRING").is_none() {
            let home = tempfile::tempdir().expect("isolated pairing home");
            let output = std::process::Command::new(
                std::env::current_exe().expect("test executable"),
            )
            .args([
                "--exact",
                "remote::tests::approved_device_enrolls_without_codes_and_resumes_after_restart",
            ])
            .env("GNOMCODE_HOME", home.path())
            .env("GNOMCODE_TEST_ISOLATED_PAIRING", "1")
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
        let phone_keys = RemoteKeyPair::from_private([23; 32]);
        let mut manager = Arc::new(RemoteManager::for_device(
            crate::remote_trust::path(),
            phone_keys.public_key_hex(),
            std::sync::Weak::new(),
        ));
        let db = Arc::new(Mutex::new(Database::open_in_memory().expect("database")));
        // Network verification is explicit; normal test runs stay on loopback.
        let base = std::env::var("GNOMCODE_TEST_REMOTE_RELAY_URL")
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
        assert!(!manager.status().await.paired);
        let phone_confirm = phone_channel
            .seal(br#"{"type":"enroll"}"#)
            .expect("seal confirm");
        phone_socket
            .send(Message::Binary(
                serde_json::to_vec(&phone_confirm).expect("frame").into(),
            ))
            .await
            .expect("send confirm");
        // Pairing takes several seconds on a loaded machine; the deadline only guards against hangs.
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
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
            for kind in [
                "continue_here",
                "worktree_merge",
                "worktree_discard",
                "worktree_open_pr",
                "interrupt",
                "approvals",
                "approval",
            ] {
                let active_room = manager.current.lock().await.clone().expect("room");
                let old = chrono::Utc::now().timestamp() - 61;
                active_room
                    .trusted
                    .lock()
                    .await
                    .as_mut()
                    .expect("trust")
                    .last_activity = old;
                let request = json!({"type":kind,"session_id":"missing-activity-session","id":"missing-request","decision":"deny","request_id":"activity-check-request"});
                let frame = fresh
                    .seal(&serde_json::to_vec(&request).expect("request"))
                    .expect("encrypted request");
                next.send(Message::Binary(
                    serde_json::to_vec(&frame).expect("frame").into(),
                ))
                .await
                .expect("send activity request");
                let refreshed = tokio::time::timeout(std::time::Duration::from_secs(2), async {
                    loop {
                        if active_room
                            .trusted
                            .lock()
                            .await
                            .as_ref()
                            .expect("trust")
                            .last_activity
                            > old
                        {
                            break;
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                    }
                })
                .await;
                assert!(
                    refreshed.is_ok(),
                    "{kind} did not refresh authenticated room activity"
                );
            }
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
        if std::env::var_os("GNOMCODE_TEST_ISOLATED_HANDOFF").is_none() {
            let home = tempfile::tempdir().expect("isolated handoff home");
            let output = std::process::Command::new(std::env::current_exe().expect("test executable"))
                .args([
                    "--exact",
                    "remote::tests::handoff_notice_persists_in_new_conversation_and_deletes_without_resurrection",
                ])
                .env("GNOMCODE_HOME", home.path())
                .env("GNOMCODE_TEST_ISOLATED_HANDOFF", "1")
                .output()
                .expect("isolated handoff test");
            assert!(
                output.status.success(),
                "{} {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        let db = Database::open_in_memory().expect("db");
        let mut db = db;
        let now = chrono::Utc::now();
        let project = Project::from_origin(None, "fixture", None);
        let sessions = ["source", "new-native"].map(|id| Session {
            id: id.into(),
            agent: gnomcode_core::AgentId::Codex,
            surface: gnomcode_core::Surface::Cli,
            project_id: project.id.clone(),
            parent_session_id: None,
            model: None,
            title: None,
            state: gnomcode_core::SessionState::Idle,
            pid: None,
            started_at: now,
            last_event_at: now,
        });
        let cp = gnomcode_store::SourceCheckpoint {
            file_path: "fixture".into(),
            last_read_offset: 0,
            file_size: 0,
            fingerprint: "fixture".into(),
            last_ingested_at: now,
        };
        db.commit_batch(&[project], &sessions, &[], &[], &[], &cp)
            .expect("source metadata");
        let target = gnomcode_pty::SessionRecord {
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
        db.insert_chat_event(&gnomcode_core::ChatEvent {
            event_id: "original-message".into(),
            session_id: "source".into(),
            turn_index: 0,
            role: gnomcode_core::ChatRole::User,
            content: "Keep our conversation".into(),
            created_at: now,
            tool_name: None,
            status: None,
        })
        .expect("original chat");
        let rows = conversation_chat(&db, "new-host", "new-native", &[], Some(500), None)
            .expect("bind notice");
        assert!(
            rows.iter()
                .any(|row| row.content == "Keep our conversation")
        );
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().any(|row| row.content.contains("gpt-5.6-luna")));
        assert_eq!(
            conversation_chat(&db, "new-host", "new-native", &[], Some(500), None)
                .expect("reload")
                .len(),
            2
        );
        let visible = conversation_sessions(&db, &sessions, &[]).expect("disconnected history");
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0]["id"], "source");
        assert_eq!(visible[0]["can_resume"], true);
        assert_eq!(visible[0]["title"], "Keep our conversation");
        db.insert_chat_event(&gnomcode_core::ChatEvent {
            event_id: "continued-prompt".into(),
            session_id: "new-native".into(),
            turn_index: 1,
            role: gnomcode_core::ChatRole::User,
            content:
                "<spawnt_context>\nInternal continuation data\n</spawnt_context>\n\nHello again"
                    .into(),
            created_at: now,
            tool_name: None,
            status: None,
        })
        .expect("continued prompt");
        let continued = conversation_chat(&db, "source", "source", &sessions, Some(500), None)
            .expect("same chat");
        assert_eq!(continued.len(), 3);
        assert_eq!(continued.last().expect("last").content, "Hello again");
        assert!(
            continued
                .iter()
                .all(|event| !event.content.contains("Internal continuation data"))
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
        assert_eq!(
            response["message"],
            "Could not prepare model switch. Check the selected agent and workspace on your desktop."
        );
    }

    #[test]
    fn subagents_get_real_names_and_a_working_parent_and_hide_when_unchecked() {
        let mut rows = vec![
            json!({"id":"p","state":"idle","title":"Parent","parent_session_id":null,"started_at":"2026-11-13T04:00:00Z","last_event_at":"2026-11-13T05:00:00Z"}),
            json!({"id":"a","state":"tool","title":"Wire watcher into daemon","parent_session_id":"p","started_at":"2026-11-13T04:10:00Z","last_event_at":"2026-11-13T05:10:00Z"}),
            json!({"id":"b","state":"thinking","title":"Subagent","parent_session_id":"p","started_at":"2026-11-13T04:05:00Z","last_event_at":"2026-11-13T05:05:00Z"}),
            json!({"id":"c","state":"idle","title":null,"parent_session_id":"p","started_at":"2026-11-13T04:20:00Z","last_event_at":"2026-11-13T05:01:00Z"}),
            json!({"id":"q","state":"idle","title":"Quiet","parent_session_id":null,"started_at":"2026-11-13T03:00:00Z","last_event_at":"2026-11-13T04:00:00Z"}),
            json!({"id":"o","state":"idle","title":"Orphan","parent_session_id":"gone","started_at":"2026-11-13T02:00:00Z","last_event_at":"2026-11-13T03:00:00Z"}),
        ];
        nest_subagents(&mut rows);
        assert_eq!(rows[0]["state"], "thinking");
        assert_eq!(rows[0]["state_label"], "Working · 2 subagents running");
        let names: Vec<_> = rows[0]["subagents"]
            .as_array()
            .expect("nested")
            .iter()
            .map(|child| {
                (
                    child["id"].as_str().expect("id"),
                    child["name"].as_str().expect("name"),
                )
            })
            .collect();
        assert_eq!(
            names,
            [
                ("a", "Wire watcher into daemon"),
                ("b", "Subagent 1"),
                ("c", "Subagent 3")
            ],
            "newest first, ordinals follow start order"
        );
        assert!(rows[4]["state_label"].is_null(), "no children, no rollup");
        let hidden = conversation_page(rows.clone(), &json!({}));
        let sessions = hidden["sessions"].as_array().expect("sessions");
        assert_eq!(sessions.len(), 2, "child rows leave the top level");
        assert!(sessions[0]["subagents"].is_null());
        assert_eq!(sessions[0]["subagent_count"], 3);
        let shown = conversation_page(rows, &json!({"include_subagents":true}));
        assert_eq!(
            shown["sessions"].as_array().expect("sessions").len(),
            3,
            "children stay nested while their parent is listed; orphans surface"
        );
        let parent = shown["sessions"]
            .as_array()
            .expect("sessions")
            .iter()
            .find(|row| row["id"] == "p")
            .expect("parent");
        assert!(parent["subagents"].is_array());
    }

    #[test]
    fn subagents_nest_under_their_parent_and_keep_it_in_the_ongoing_filter() {
        let mut rows = vec![
            json!({"id":"p","state":"idle","parent_session_id":null,"last_event_at":"2026-11-13T05:00:00Z"}),
            json!({"id":"a","state":"tool","parent_session_id":"p","last_event_at":"2026-11-13T05:10:00Z"}),
            json!({"id":"b","state":"idle","parent_session_id":"p","last_event_at":"2026-11-13T05:05:00Z"}),
            json!({"id":"q","state":"idle","parent_session_id":null,"last_event_at":"2026-11-13T04:00:00Z"}),
        ];
        nest_subagents(&mut rows);
        assert_eq!(rows[0]["subagent_count"], 2);
        assert_eq!(rows[0]["subagents_ongoing"], 1);
        assert_eq!(rows[0]["subagents"][0]["id"], "a", "newest first");
        assert!(rows[3]["subagents"].is_null());
        let page = conversation_page(rows, &json!({"status":"ongoing"}));
        let ids: Vec<_> = page["sessions"]
            .as_array()
            .expect("sessions")
            .iter()
            .map(|row| row["id"].as_str().expect("id"))
            .collect();
        assert_eq!(ids, ["p"]);
    }
}
