//! Explicit native consent grants Remote to authenticated devices on the same account.

use crate::remote::{RemoteError, RemoteManager};
use crate::sync::CloudAuth;
use gnomcode_store::Database;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};
use tokio::sync::Mutex;

#[derive(Clone, Deserialize, Serialize)]
struct Request {
    id: String,
    public_key: String,
    name: String,
    #[serde(default)]
    revoked: bool,
    #[serde(default, skip_serializing)]
    offer: Option<Value>,
    #[serde(default, skip_serializing)]
    state: String,
}

impl Request {
    fn valid(&self) -> bool {
        is_hex(&self.id, 32)
            && is_hex(&self.public_key, 64)
            && (1..=80).contains(&self.name.len())
            && !self.name.chars().any(char::is_control)
    }
}

pub(crate) fn is_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[derive(Default)]
struct State {
    owner: String,
    desktop: String,
    approved: BTreeMap<String, Request>,
    account_access: bool,
    remote_enabled: bool,
    rooms: BTreeMap<String, Arc<RemoteManager>>,
    directory_error: bool,
}

impl State {
    fn admit(&mut self, request: &Request) -> bool {
        if !self.account_access || !self.remote_enabled || request.revoked || !request.valid() {
            return false;
        }
        self.approved.insert(request.id.clone(), request.clone());
        true
    }
}

#[derive(Default)]
pub(crate) struct AccountDevices {
    state: Mutex<State>,
    changed: tokio::sync::Notify,
}

fn root(owner: &str) -> PathBuf {
    gnomcode_platform::gnomcode_home()
        .join(obfstr::obfstr!("remote-devices"))
        .join(owner)
}

async fn write_json(path: PathBuf, value: Value) -> Result<(), RemoteError> {
    tokio::task::spawn_blocking(move || {
        let bytes = serde_json::to_vec(&value).map_err(|_| RemoteError::Persistence)?;
        gnomcode_platform::write_private_file(&path, &bytes).map_err(|_| RemoteError::Persistence)
    })
    .await
    .map_err(|_| RemoteError::Persistence)?
}

async fn load_state(owner: &str) -> Result<State, RemoteError> {
    let path = root(owner).join("devices.json");
    let value = tokio::task::spawn_blocking(move || match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice::<Value>(&bytes).map_err(|_| RemoteError::Persistence),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Value::Null),
        Err(_) => Err(RemoteError::Persistence),
    })
    .await
    .map_err(|_| RemoteError::Persistence)??;
    let desktop = match value["desktop"].as_str().filter(|id| is_hex(id, 32)) {
        Some(id) => id.to_owned(),
        None if value.is_null() => {
            let mut bytes = [0; 16];
            getrandom::fill(&mut bytes).map_err(|_| RemoteError::Persistence)?;
            crate::remote_trust::hex(&bytes)
        }
        None => return Err(RemoteError::Persistence),
    };
    let approved: BTreeMap<String, Request> = if value.is_null() {
        BTreeMap::new()
    } else {
        serde_json::from_value(value["approved"].clone()).map_err(|_| RemoteError::Persistence)?
    };
    if approved
        .iter()
        .any(|(id, request)| id != &request.id || !request.valid())
    {
        return Err(RemoteError::Persistence);
    }
    let state = State {
        owner: owner.to_owned(),
        desktop,
        approved,
        account_access: value["account_access"].as_bool().unwrap_or(false),
        ..State::default()
    };
    save_state(&state).await?;
    Ok(state)
}

async fn save_state(state: &State) -> Result<(), RemoteError> {
    write_json(
        root(&state.owner).join("devices.json"),
        json!({"desktop":state.desktop,"approved":state.approved,"account_access":state.account_access}),
    )
    .await
}

impl AccountDevices {
    pub(crate) async fn snapshot_desktop_id(&self, owner: &str) -> Result<String, RemoteError> {
        if !uuid_owner(owner) {
            return Err(RemoteError::NotPaired);
        }
        let state = self.state.lock().await;
        if state.owner == owner {
            return Ok(state.desktop.clone());
        }
        drop(state);
        Ok(load_state(owner).await?.desktop)
    }

    pub(crate) async fn enable_account_access(&self, owner: &str) -> Result<(), RemoteError> {
        if !uuid_owner(owner) {
            return Err(RemoteError::NotPaired);
        }
        let mut state = self.state.lock().await;
        if state.owner != owner {
            for room in state.rooms.values() {
                room.suspend().await;
            }
            *state = load_state(owner).await?;
        }
        state.account_access = true;
        save_state(&state).await?;
        self.changed.notify_one();
        Ok(())
    }

    pub(crate) async fn disconnect_owner(&self, owner: &str) -> Result<(), RemoteError> {
        if !uuid_owner(owner) {
            return Err(RemoteError::NotPaired);
        }
        let mut state = self.state.lock().await;
        for room in state.rooms.values() {
            room.suspend().await;
        }
        let mut saved = load_state(owner).await?;
        saved.account_access = false;
        save_state(&saved).await?;
        *state = saved;
        self.changed.notify_one();
        Ok(())
    }

    pub(crate) async fn changed(&self) {
        self.changed.notified().await;
    }

    pub(crate) async fn disconnect(&self) -> Result<(), RemoteError> {
        let mut state = self.state.lock().await;
        for room in state.rooms.values() {
            room.suspend().await;
        }
        *state = State::default();
        Ok(())
    }

    pub(crate) async fn command(&self, value: Value) -> Value {
        if value["type"] == "devices" {
            return self.view().await;
        }
        json!({"type":"error","message":"Manage account access in Profile > Devices."})
    }

    pub(crate) async fn view(&self) -> Value {
        let state = self.state.lock().await;
        let mut devices = Vec::new();
        for request in state.approved.values() {
            let connected = if let Some(room) = state.rooms.get(&request.id) {
                room.status().await.paired
            } else {
                false
            };
            devices.push(json!({"id":request.id,"name":request.name,"state":"account","connected":connected}));
        }
        json!({"type":"devices","desktop_id":state.desktop,"account_access":state.account_access,"remote_enabled":state.remote_enabled,"devices":devices,"directory_error":state.directory_error})
    }

    pub(crate) async fn set_account_access(
        &self,
        client: &reqwest::Client,
        auth: &CloudAuth,
        enabled: bool,
    ) -> Result<(), RemoteError> {
        if enabled {
            self.enable_account_access(&auth.user_id).await?;
        } else {
            self.disconnect_owner(&auth.user_id).await?;
        }
        let state = self.state.lock().await;
        let desktop = state.desktop.clone();
        drop(state);
        client.post(format!("{}/rest/v1/spawnt_desktops?on_conflict=user_id,id",auth.base_url))
            .header("apikey",&auth.public_key).bearer_auth(&auth.access_token)
            .header("Prefer","resolution=ignore-duplicates,return=minimal")
            .json(&json!({"user_id":auth.user_id,"id":desktop,"name":format!("Desktop {}",&desktop[28..]),"remote_enabled":false}))
            .send().await.map_err(|_|RemoteError::Relay)?.error_for_status().map_err(|_|RemoteError::Relay)?;
        client
            .patch(format!(
                "{}/rest/v1/spawnt_desktops?user_id=eq.{}&id=eq.{desktop}",
                auth.base_url, auth.user_id
            ))
            .header("apikey", &auth.public_key)
            .bearer_auth(&auth.access_token)
            .json(&json!({"remote_enabled":enabled}))
            .send()
            .await
            .map_err(|_| RemoteError::Relay)?
            .error_for_status()
            .map_err(|_| RemoteError::Relay)?;
        let mut state = self.state.lock().await;
        if state.owner == auth.user_id && state.desktop == desktop {
            state.remote_enabled = enabled;
            save_state(&state).await?;
        }
        Ok(())
    }

    pub(crate) async fn remove_device(
        &self,
        client: &reqwest::Client,
        auth: &CloudAuth,
        device_id: &str,
    ) -> Result<(), RemoteError> {
        let mut state = self.state.lock().await;
        if let Some(room) = state.rooms.remove(device_id) {
            let _ = room.revoke().await;
        }
        state.approved.remove(device_id);
        save_state(&state).await?;
        let desktop = state.desktop.clone();
        drop(state);

        let _ = client
            .delete(format!(
                "{}/rest/v1/spawnt_device_requests?user_id=eq.{}&desktop_id=eq.{}&id=eq.{}",
                auth.base_url, auth.user_id, desktop, device_id
            ))
            .header("apikey", &auth.public_key)
            .bearer_auth(&auth.access_token)
            .send()
            .await;

        self.changed.notify_one();
        Ok(())
    }

    pub(crate) async fn poll(
        self: &Arc<Self>,
        client: &reqwest::Client,
        auth: &CloudAuth,
        db: Arc<Mutex<Database>>,
    ) -> Result<(), RemoteError> {
        let result = self.poll_inner(client, auth, db).await;
        self.state.lock().await.directory_error = result.is_err();
        result
    }

    async fn poll_inner(
        self: &Arc<Self>,
        client: &reqwest::Client,
        auth: &CloudAuth,
        db: Arc<Mutex<Database>>,
    ) -> Result<(), RemoteError> {
        if !uuid_owner(&auth.user_id) {
            return Err(RemoteError::NotPaired);
        }
        let mut state = self.state.lock().await;
        if state.owner != auth.user_id {
            for room in state.rooms.values() {
                room.revoke().await?;
            }
            *state = load_state(&auth.user_id).await?;
        }
        let desktop = state.desktop.clone();
        // Register only once: heartbeat writes must never undo account-side revocation.
        client.post(format!("{}/rest/v1/spawnt_desktops?on_conflict=user_id,id",auth.base_url))
            .header("apikey",&auth.public_key).bearer_auth(&auth.access_token)
            .header("Prefer","resolution=ignore-duplicates,return=minimal")
            .json(&json!({"user_id":auth.user_id,"id":desktop,"name":format!("Desktop {}",&desktop[28..]),"remote_enabled":false}))
            .send().await.map_err(|_|RemoteError::Relay)?.error_for_status().map_err(|_|RemoteError::Relay)?;
        let mut heartbeat = json!({"last_seen_at":chrono::Utc::now().to_rfc3339()});
        if !state.account_access {
            heartbeat["remote_enabled"] = json!(false);
        }
        client
            .patch(format!(
                "{}/rest/v1/spawnt_desktops?user_id=eq.{}&id=eq.{desktop}",
                auth.base_url, auth.user_id
            ))
            .header("apikey", &auth.public_key)
            .bearer_auth(&auth.access_token)
            .json(&heartbeat)
            .send()
            .await
            .map_err(|_| RemoteError::Relay)?
            .error_for_status()
            .map_err(|_| RemoteError::Relay)?;
        let hosts = client
            .get(format!(
                "{}/rest/v1/spawnt_desktops?user_id=eq.{}&id=eq.{desktop}&select=remote_enabled",
                auth.base_url, auth.user_id
            ))
            .header("apikey", &auth.public_key)
            .bearer_auth(&auth.access_token)
            .send()
            .await
            .map_err(|_| RemoteError::Relay)?
            .error_for_status()
            .map_err(|_| RemoteError::Relay)?
            .json::<Vec<Value>>()
            .await
            .map_err(|_| RemoteError::Relay)?;
        state.remote_enabled = hosts.len() == 1 && hosts[0]["remote_enabled"] == true;
        if !state.account_access || !state.remote_enabled {
            for room in state.rooms.values() {
                room.suspend().await;
            }
            state.rooms.clear();
            return Ok(());
        }
        let response=client.get(format!("{}/rest/v1/spawnt_device_requests?user_id=eq.{}&desktop_id=eq.{desktop}&select=id,public_key,name,revoked,offer,state&limit=65",auth.base_url,auth.user_id))
            .header("apikey",&auth.public_key).bearer_auth(&auth.access_token)
            .send().await.map_err(|_|RemoteError::Relay)?.error_for_status().map_err(|_|RemoteError::Relay)?
            .json::<Vec<Request>>().await.map_err(|_|RemoteError::Relay)?;
        if response.len() > 64 || response.iter().any(|request| !request.valid()) {
            state.directory_error = true;
            return Err(RemoteError::Relay);
        }
        let requests: BTreeMap<_, _> = response
            .into_iter()
            .map(|request| (request.id.clone(), request))
            .collect();
        // The owner-scoped directory is the authorization source after native consent.
        let revoked: Vec<_> = state
            .approved
            .iter()
            .filter(|(id, approved)| {
                requests.get(*id).is_none_or(|request| {
                    request.revoked || request.public_key != approved.public_key
                })
            })
            .map(|(id, _)| id.clone())
            .collect();
        for id in revoked {
            if let Some(room) = state.rooms.remove(&id) {
                room.revoke().await?;
            }
            state.approved.remove(&id);
            save_state(&state).await?;
        }
        let mut connections_changed = false;
        for request in requests.values() {
            if request.revoked {
                continue;
            }
            connections_changed |= !state.approved.contains_key(&request.id);
            if !state.admit(request) {
                continue;
            }
            let manager = state
                .rooms
                .entry(request.id.clone())
                .or_insert_with(|| {
                    Arc::new(RemoteManager::for_device(
                        root(&auth.user_id).join(format!("{}.json", request.id)),
                        request.public_key.clone(),
                        Arc::downgrade(self),
                    ))
                })
                .clone();
            if !manager.status().await.enabled {
                if !crate::pro::has_remote_access().await {
                    continue;
                }
                let _ = manager.start_device(db.clone(), request.id.clone()).await;
            }
            let status = manager.status().await;
            if let (Some(key), Some(relay)) = (status.public_key, status.relay_url) {
                let offer = json!({"room":request.id,"key":key,"relay":relay,"protocol_version":3});
                if request.offer.as_ref() == Some(&offer) && request.state == "approved" {
                    continue;
                }
                let response=client.patch(format!("{}/rest/v1/spawnt_device_requests?user_id=eq.{}&desktop_id=eq.{desktop}&id=eq.{}&public_key=eq.{}",auth.base_url,auth.user_id,request.id,request.public_key))
                    .header("apikey",&auth.public_key).bearer_auth(&auth.access_token)
                    .json(&json!({"offer":{"room":request.id,"key":key,"relay":relay,"protocol_version":3},"state":"approved"}))
                    .send().await.map_err(|_|RemoteError::Relay)?;
                response
                    .error_for_status()
                    .map_err(|_| RemoteError::Relay)?;
            }
        }
        if connections_changed {
            save_state(&state).await?;
        }
        state.directory_error = false;
        Ok(())
    }
}

fn uuid_owner(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| {
            if [8, 13, 18, 23].contains(&index) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
}

#[cfg(test)]
#[path = "account_devices_tests.rs"]
mod tests;
