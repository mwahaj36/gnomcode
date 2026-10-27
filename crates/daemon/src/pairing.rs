//! Browser-approved, encrypted, single-use account pairing.

use aes_gcm::{
    Aes256Gcm, KeyInit,
    aead::{Aead, Payload},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::time::Duration;

type PairError = Box<dyn std::error::Error + Send + Sync>;

/// An expiring browser approval request. The URL is a temporary secret; never log it.
pub struct PairingRequest {
    id: String,
    key: [u8; 32],
    expires_at: i64,
    /// Browser URL, with secrets confined to the fragment.
    pub url: String,
    /// Verification code to compare with the browser before approval.
    pub code: String,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn hash(input: &[u8]) -> String {
    hex(&Sha256::digest(input))
}

impl PairingRequest {
    /// Creates an independent request for this device or workspace.
    ///
    /// # Errors
    /// Fails if secure randomness or the pairing URL is unavailable.
    pub fn new(name: &str) -> Result<Self, PairError> {
        let mut key = [0u8; 32];
        let mut id = [0u8; 16];
        getrandom::fill(&mut key).map_err(std::io::Error::other)?;
        getrandom::fill(&mut id).map_err(std::io::Error::other)?;
        let id = hex(&id);
        let code = hash(&key)[..8].to_uppercase();
        let expires_at = chrono::Utc::now().timestamp() + 600;
        let (project, _) = crate::sync::cloud_config();
        let mut fragment = reqwest::Url::parse("https://localhost/")?;
        fragment.query_pairs_mut().extend_pairs([
            ("id", id.as_str()),
            ("key", hex(&key).as_str()),
            ("expires", expires_at.to_string().as_str()),
            ("name", name.chars().take(80).collect::<String>().as_str()),
            ("project", project.as_str()),
        ]);
        let mut url = reqwest::Url::parse("https://fleethud.vercel.app/pair")?;
        url.set_fragment(fragment.query());
        Ok(Self {
            id,
            key,
            expires_at,
            url: url.to_string(),
            code,
        })
    }

    /// Whether the local ten-minute approval window has elapsed.
    #[must_use]
    pub fn expired(&self) -> bool {
        chrono::Utc::now().timestamp() >= self.expires_at
    }

    fn decrypt(&self, envelope: &Value) -> Result<Value, PairError> {
        let iv = STANDARD.decode(envelope["iv"].as_str().ok_or("Missing pairing IV")?)?;
        let ciphertext = STANDARD.decode(
            envelope["ciphertext"]
                .as_str()
                .ok_or("Missing pairing ciphertext")?,
        )?;
        if iv.len() != 12 || ciphertext.len() > 65536 {
            return Err("Invalid pairing envelope".into());
        }
        let cipher = Aes256Gcm::new_from_slice(&self.key).map_err(|_| "Invalid pairing key")?;
        let plain = cipher
            .decrypt(
                iv.as_slice().into(),
                Payload {
                    msg: &ciphertext,
                    aad: self.id.as_bytes(),
                },
            )
            .map_err(|_| "Pairing verification failed")?;
        Ok(serde_json::from_slice(&plain)?)
    }

    /// Waits for approval, validates the fresh login with Supabase, then saves it securely.
    ///
    /// # Errors
    /// Reports expiry, missing migration, network, validation or file errors without credentials.
    pub async fn complete(&self, path: &Path) -> Result<(), PairError> {
        let client = crate::cloud::client()?;
        let (base, public_key) = crate::sync::cloud_config();
        while !self.expired() {
            let response = client
                .post(format!("{base}/rest/v1/rpc/fleethud_pair_consume"))
                .header("apikey", &public_key)
                .json(&json!({"p_request_id": self.id, "p_poll_token": hash(&self.key)}))
                .send()
                .await
                .map_err(
                    |_| "Cannot reach account pairing service. Check your network and retry.",
                )?;
            if response.status() == reqwest::StatusCode::NOT_FOUND {
                return Err("Pairing is not enabled on this Supabase project. Apply supabase_device_pairing_v4.sql, then retry.".into());
            }
            if !response.status().is_success() {
                return Err("Account pairing service rejected the request. Check project configuration and retry.".into());
            }
            let envelope: Value = response
                .json()
                .await
                .map_err(|_| "Invalid pairing response")?;
            if !envelope.is_null() {
                let session = self.decrypt(&envelope)?;
                validate_session(&client, &base, &public_key, &session).await?;
                crate::sync::save_cloud_session(path, &session).await?;
                return Ok(());
            }
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
        Err("Pairing expired. Start again and approve within ten minutes.".into())
    }
}

pub(crate) async fn validate_session(
    client: &reqwest::Client,
    base: &str,
    key: &str,
    session: &Value,
) -> Result<(), PairError> {
    let token = session["access_token"]
        .as_str()
        .filter(|v| !v.is_empty())
        .ok_or("Missing login token")?;
    let id = session
        .pointer("/user/id")
        .and_then(Value::as_str)
        .ok_or("Missing account ID")?;
    if session["refresh_token"].as_str().is_none_or(str::is_empty) {
        return Err("A renewable login session is required".into());
    }
    let response = client
        .get(format!("{base}/auth/v1/user"))
        .header("apikey", key)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|_| "Cannot validate account login")?;
    if !response.status().is_success() {
        return Err("Login expired or belongs to another Supabase project".into());
    }
    let user: Value = response
        .json()
        .await
        .map_err(|_| "Invalid account response")?;
    if user["id"].as_str() != Some(id) {
        return Err("Account identity mismatch".into());
    }
    Ok(())
}

#[derive(Clone, serde::Serialize)]
pub(crate) struct PairingProgress {
    url: String,
    code: String,
    status: String,
    message: String,
}
struct BrowserFlow {
    expires_at: i64,
    progress: std::sync::Arc<tokio::sync::Mutex<PairingProgress>>,
    task: tokio::task::JoinHandle<()>,
}
static BROWSER_FLOW: tokio::sync::Mutex<Option<BrowserFlow>> = tokio::sync::Mutex::const_new(None);

pub(crate) async fn browser_start() -> Result<PairingProgress, PairError> {
    let mut flow = BROWSER_FLOW.lock().await;
    if let Some(existing) = flow.as_ref() {
        if !existing.task.is_finished() && existing.expires_at > chrono::Utc::now().timestamp() {
            return Ok(existing.progress.lock().await.clone());
        }
        existing.task.abort();
    }
    let request = PairingRequest::new(&fleethud_platform::hostname())?;
    let expires_at = request.expires_at;
    let progress = std::sync::Arc::new(tokio::sync::Mutex::new(PairingProgress {
        url: request.url.clone(),
        code: request.code.clone(),
        status: "pending".to_owned(),
        message: "Open the link, compare the code, sign in and approve this device.".to_owned(),
    }));
    let update = std::sync::Arc::clone(&progress);
    let task = tokio::spawn(async move {
        let result = request
            .complete(&fleethud_platform::fleethud_home().join("session.json"))
            .await;
        let mut status = update.lock().await;
        status.url.clear();
        match result {
            Ok(()) => {
                status.status = "paired".to_owned();
                status.message =
                    "Account paired. Allow up to 45 seconds for session sync.".to_owned();
            }
            Err(error) => {
                status.status = "error".to_owned();
                status.message = error.to_string();
            }
        }
    });
    let result = progress.lock().await.clone();
    *flow = Some(BrowserFlow {
        expires_at,
        progress,
        task,
    });
    Ok(result)
}

pub(crate) async fn browser_status() -> Option<PairingProgress> {
    let flow = BROWSER_FLOW.lock().await;
    match flow.as_ref() {
        Some(flow) => Some(flow.progress.lock().await.clone()),
        None => None,
    }
}

pub(crate) async fn browser_cancel() {
    if let Some(flow) = BROWSER_FLOW.lock().await.take() {
        flow.task.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_keeps_secret_out_of_url_query_and_expires() {
        let request = PairingRequest::new("test workspace").expect("request");
        let url = reqwest::Url::parse(&request.url).expect("url");
        assert!(url.query().is_none());
        assert!(url.fragment().expect("fragment").contains("key="));
        assert_eq!(request.code.len(), 8);
        assert!(!request.expired());
        let mut expired = request;
        expired.expires_at = 0;
        assert!(expired.expired());
    }

    #[test]
    fn rejects_tampered_ciphertext_and_wrong_request() {
        use aes_gcm::{
            Aes256Gcm, KeyInit,
            aead::{Aead, Payload},
        };
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        let mut request = PairingRequest::new("test workspace").expect("request");
        let cipher = Aes256Gcm::new_from_slice(&request.key).expect("key");
        let nonce = [7u8; 12];
        let ciphertext = cipher
            .encrypt(
                (&nonce).into(),
                Payload {
                    msg: b"{\"access_token\":\"test\"}",
                    aad: request.id.as_bytes(),
                },
            )
            .expect("encrypt");
        let envelope = serde_json::json!({"iv": STANDARD.encode(nonce), "ciphertext": STANDARD.encode(&ciphertext)});
        assert_eq!(
            request.decrypt(&envelope).expect("decrypt")["access_token"],
            "test"
        );
        request.id = "different-request".to_owned();
        assert!(request.decrypt(&envelope).is_err());
    }
}
