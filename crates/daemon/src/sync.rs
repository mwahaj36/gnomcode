//! Supabase cloud synchronization client structures and rollup hashing (`SPEC §3.15`).
//!
//! Provides:
//! - Device registration payload and token hash verification.
//! - Hourly usage rollup batching (`usage_hourly_sync`).
//! - Cross-device quota pool sync (`quota_pool_sync`).
//! - HMAC-SHA256 privacy-preserving project key generation (`SPEC §3.15.D`).

use chrono::{DateTime, Utc};
use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::Sha256;
use spawnt_store::Database;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone, Serialize)]
pub(crate) struct CloudSyncStatus {
    state: &'static str,
    last_attempt_at: Option<String>,
    last_success_at: Option<String>,
    message: &'static str,
}

static SYNC_STATUS: Mutex<CloudSyncStatus> = Mutex::const_new(CloudSyncStatus {
    state: "unknown",
    last_attempt_at: None,
    last_success_at: None,
    message: "Cloud publication has not been checked in this daemon run.",
});

pub(crate) async fn cloud_sync_status() -> CloudSyncStatus {
    SYNC_STATUS.lock().await.clone()
}

/// Synchronized hourly usage record for multi-device mirroring and team rollups.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HourlyUsageSync {
    /// Associated organization ID.
    pub org_id: String,
    /// Associated user ID.
    pub user_id: String,
    /// Originating device ID.
    pub device_id: String,
    /// UTC hourly bucket timestamp (e.g. `2026-10-26T15:00:00Z`).
    pub hour_bucket: DateTime<Utc>,
    /// Privacy-preserving HMAC-SHA256 hash or plaintext project key.
    pub project_key: String,
    /// Optional plaintext project name (omitted unless user opted in).
    pub project_label: Option<String>,
    /// Agent identifier.
    pub agent: String,
    /// Model identifier.
    pub model: String,
    /// Total input tokens during this hour bucket.
    pub input_tokens: u64,
    /// Total output tokens during this hour bucket.
    pub output_tokens: u64,
    /// Total cached input tokens read.
    pub cache_read_tokens: u64,
    /// Total cache write tokens.
    pub cache_write_tokens: u64,
    /// Total API-equivalent cost in USD.
    pub cost_usd: f64,
    /// Cost basis (`exact` or `estimated`).
    pub cost_basis: String,
    /// Number of turns executed.
    pub turn_count: u32,
}

/// Computes the privacy-preserving HMAC-SHA256 project key (`SPEC §3.15.D`).
///
/// Ensures repository names and folder structures never leak to remote servers
/// unless the user or team explicitly enables `share_project_names`.
#[must_use]
pub fn hash_project_key(org_salt: &[u8], canonical_project_name: &str) -> String {
    let Ok(mut mac) = HmacSha256::new_from_slice(org_salt) else {
        return "proj_unhashed".to_owned();
    };
    mac.update(canonical_project_name.as_bytes());
    let res = mac.finalize().into_bytes();
    let mut hex = String::with_capacity(32);
    for b in &res[..16] {
        use std::fmt::Write as _;
        let _ = write!(hex, "{b:02x}");
    }
    format!("proj_{hex}")
}

fn build_remote_snapshot(db: &Database) -> Result<Value, spawnt_store::Error> {
    let today = db.generate_usage_report_timeline("today", None, None)?;
    let week = db.generate_usage_report_timeline("7d", None, None)?;
    let month = db.generate_usage_report_timeline("30d", None, None)?;
    let calendar_month = db.generate_usage_report_timeline("month", None, None)?;
    let all = db.generate_usage_report_timeline("all", None, None)?;
    let (sessions, count) = db.list_sessions_paginated(None, None, 0, 200)?;
    let sessions: Vec<_> = sessions
        .into_iter()
        .map(|s| {
            json!({
                "id": s.id, "agent": s.agent, "surface": s.surface,
                "project_name": s.project_name, "model": s.model, "state": s.state,
                "is_active": s.is_active, "is_ongoing": s.is_ongoing, "presence": s.presence,
                "started_at": s.started_at, "last_event_at": s.last_event_at,
                "input_tokens": s.input_tokens, "output_tokens": s.output_tokens,
                "cache_read_tokens": s.cache_read_tokens, "context_tokens": s.context_tokens,
                "context_window": s.context_window, "total_tokens": s.total_tokens,
                "context_fill_pct": s.context_fill_pct, "total_cost_usd": s.total_cost_usd,
                "priced_turns": s.priced_turns, "unpriced_turns": s.unpriced_turns,
                "usage_available": s.usage_available, "cost_basis": s.cost_basis,
            })
        })
        .collect();
    let (_, ongoing_count) = db.list_sessions_paginated(None, Some("ongoing"), 0, 1)?;
    let (_, active_count) = db.list_sessions_paginated(None, Some("active"), 0, 1)?;
    Ok(json!({
        "generated_at": Utc::now().to_rfc3339(),
        "timeline": today,
        "timelines": { "today": today, "7d": week, "30d": month, "month": calendar_month, "all": all },
        "sessions": sessions,
        "quota": crate::api::quota_views(db.get_latest_quota_snapshots()?),
        "active_count": active_count,
        "ongoing_count": ongoing_count,
        "total_count": count,
    }))
}

fn discovery_offer(status: &crate::remote::PairingStatus, now: DateTime<Utc>) -> Option<Value> {
    if !status.enabled || status.paired {
        return None;
    }
    let expires_at = status.expires_at.as_deref()?;
    if DateTime::parse_from_rfc3339(expires_at).ok()? <= now {
        return None;
    }
    Some(json!({
        "room": status.room_id.as_deref()?,
        "key": status.public_key.as_deref()?,
        "relay": status.relay_url.as_deref()?,
        "expires_at": expires_at,
        "name": "My desktop",
        "pairing_code": status.pairing_code()?,
        "protocol_version": 2,
    }))
}

/// Pushes a privacy-filtered live snapshot every minute after the user pairs an account.
pub fn spawn_cloud_sync(
    db: Arc<Mutex<Database>>,
    remote: Arc<crate::remote::RemoteManager>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let Ok(client) = reqwest::Client::builder()
            .https_only(true)
            .timeout(Duration::from_secs(10))
            .build()
        else {
            tracing::warn!("cloud sync client could not start");
            return;
        };
        let mut ticker = tokio::time::interval(Duration::from_secs(60));
        loop {
            tokio::select! {
                _ = ticker.tick() => {},
                () = remote.discovery_changed() => {},
            }
            {
                let mut status = SYNC_STATUS.lock().await;
                status.state = "publishing";
                status.last_attempt_at = Some(Utc::now().to_rfc3339());
                status.message = "Publishing the latest metadata snapshot.";
            }
            let result = push_snapshot(&client, &db, &remote).await;
            let mut status = SYNC_STATUS.lock().await;
            match result {
                Ok(true) => {
                    status.state = "connected";
                    status.last_success_at = Some(Utc::now().to_rfc3339());
                    status.message = "Cloud snapshot published successfully.";
                }
                Ok(false) => {
                    status.state = "unpaired";
                    status.message = "Connect an account to publish desktop snapshots.";
                }
                Err(error) => {
                    status.state = "error";
                    status.message = "Cloud publication failed. Check connectivity and reconnect account pairing if needed.";
                    tracing::warn!("cloud sync failed: {error}");
                }
            }
        }
    })
}

async fn push_snapshot(
    client: &reqwest::Client,
    db: &Arc<Mutex<Database>>,
    remote: &crate::remote::RemoteManager,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let session_path = spawnt_platform::spawnt_home().join("session.json");
    let Some(CloudAuth {
        user_id,
        access_token,
        base_url,
        public_key,
    }) = read_cloud_auth(client, &session_path).await?
    else {
        return Ok(false);
    };

    let db = Arc::clone(db);
    let mut snapshot = tokio::task::spawn_blocking(move || {
        let guard = db.blocking_lock();
        build_remote_snapshot(&guard)
    })
    .await??;
    snapshot["remote_offer"] =
        discovery_offer(&remote.status().await, Utc::now()).unwrap_or(Value::Null);
    client
        .post(format!(
            "{base_url}/rest/v1/spawnt_sync_state?on_conflict=user_id"
        ))
        .header("apikey", &public_key)
        .bearer_auth(&access_token)
        .header("Prefer", "resolution=merge-duplicates,return=minimal")
        .json(&json!({
            "user_id": user_id,
            "live_snapshot": snapshot,
            "updated_at": chrono::Utc::now().to_rfc3339(),
        }))
        .send()
        .await?
        .error_for_status()?;
    Ok(true)
}

pub(crate) struct CloudAuth {
    pub user_id: String,
    pub access_token: String,
    pub base_url: String,
    pub public_key: String,
}

static AUTH_REFRESH_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn credential_lock(path: &std::path::Path) -> Result<std::fs::File, std::io::Error> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let lock_path = path.with_extension("json.lock");
    for _ in 0..150 {
        match spawnt_platform::open_exclusive_lock(&lock_path) {
            Ok(file) => return Ok(file),
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock
                    || error.kind() == std::io::ErrorKind::PermissionDenied =>
            {
                tokio::time::sleep(Duration::from_millis(100)).await
            }
            Err(error) => return Err(error),
        }
    }
    Err(std::io::Error::other(
        "Account credentials are busy; try again",
    ))
}

fn stored_auth_session(session: &Value) -> Value {
    // Persist only the credentials needed for desktop summary sync.
    json!({"access_token":session["access_token"],"refresh_token":session["refresh_token"],
        "expires_at":session["expires_at"],"expires_in":session["expires_in"],"token_type":"bearer",
        "user":{"id":session.pointer("/user/id"),"email":session.pointer("/user/email")}})
}

pub(crate) async fn save_cloud_session(
    path: &std::path::Path,
    session: &Value,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _guard = AUTH_REFRESH_LOCK.lock().await;
    let _file_guard = credential_lock(path).await?;
    spawnt_platform::write_private_file(path, &serde_json::to_vec(&stored_auth_session(session))?)?;
    Ok(())
}

pub(crate) async fn remove_cloud_session(path: &std::path::Path) -> Result<(), std::io::Error> {
    let _guard = AUTH_REFRESH_LOCK.lock().await;
    let _file_guard = credential_lock(path).await?;
    match tokio::fs::remove_file(path).await {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        result => result,
    }
}

pub(crate) async fn read_cloud_auth(
    client: &reqwest::Client,
    session_path: &std::path::Path,
) -> Result<Option<CloudAuth>, Box<dyn std::error::Error + Send + Sync>> {
    // Desktop sync and manual account linking share one rotating login session.
    let _refresh_guard = AUTH_REFRESH_LOCK.lock().await;
    if !session_path.exists() {
        return Ok(None);
    }
    let _file_guard = credential_lock(session_path).await?;
    let raw = match tokio::fs::read_to_string(session_path).await {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let mut session: Value = serde_json::from_str(&raw)?;
    let Some(user_id) = session
        .pointer("/user/id")
        .and_then(Value::as_str)
        .map(str::to_owned)
    else {
        return Ok(None);
    };
    let (base_url, public_key) = cloud_config();
    let expires_at = session
        .get("expires_at")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    if expires_at <= chrono::Utc::now().timestamp() + 120 {
        let Some(refresh_token) = session.get("refresh_token").and_then(Value::as_str) else {
            return Ok(None);
        };
        let refresh_url = format!("{base_url}/auth/v1/token?grant_type=refresh_token");
        let refreshed: Value = client
            .post(refresh_url)
            .header("apikey", &public_key)
            .json(&json!({ "refresh_token": refresh_token }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        if refreshed
            .get("access_token")
            .and_then(Value::as_str)
            .is_none()
        {
            return Ok(None);
        }
        session = refreshed;
        if session.get("expires_at").is_none() {
            let expires_in = session
                .get("expires_in")
                .and_then(Value::as_i64)
                .unwrap_or(3600);
            session["expires_at"] = json!(chrono::Utc::now().timestamp() + expires_in);
        }
        spawnt_platform::write_private_file(
            session_path,
            &serde_json::to_vec(&stored_auth_session(&session))?,
        )?;
    }
    let Some(access_token) = session
        .get("access_token")
        .and_then(Value::as_str)
        .map(str::to_owned)
    else {
        return Ok(None);
    };
    Ok(Some(CloudAuth {
        user_id,
        access_token,
        base_url,
        public_key,
    }))
}

pub(crate) async fn validate_session(
    client: &reqwest::Client,
    base: &str,
    key: &str,
    session: &Value,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

pub(crate) fn cloud_config() -> (String, String) {
    let url = std::env::var("SPAWNT_SUPABASE_URL")
        .unwrap_or_else(|_| obfstr::obfstr!("https://jjnuxngvhuvxkxrfmnqx.supabase.co").to_owned());
    let key = std::env::var("SPAWNT_SUPABASE_KEY").unwrap_or_else(|_| {
        obfstr::obfstr!("sb_publishable_o2ZiGak7RHq_cmL81BOeZQ_g7JJeyCI").to_owned()
    });
    (url.trim_end_matches('/').to_owned(), key)
}

#[cfg(test)]
mod tests {
    use super::hash_project_key;

    #[test]
    fn saved_credentials_exclude_oauth_provider_tokens() {
        let input = serde_json::json!({"access_token":"account","refresh_token":"renew","provider_token":"must-not-copy","user":{"id":"user-1","email":"test@example.invalid","user_metadata":{"anything":"must-not-copy"}}});
        let saved = super::stored_auth_session(&input);
        assert_eq!(saved["access_token"], "account");
        assert_eq!(saved["user"]["id"], "user-1");
        assert!(!saved.to_string().contains("must-not-copy"));
    }

    #[test]
    fn project_key_hashing_is_deterministic_and_prefixes_proj() {
        let salt = b"spawnt-test-org-salt-123456";
        let h1 = hash_project_key(salt, "github.com/mwahaj36/spawnt");
        let h2 = hash_project_key(salt, "github.com/mwahaj36/spawnt");
        let h3 = hash_project_key(salt, "github.com/other/project");

        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
        assert!(h1.starts_with("proj_"));
        assert_eq!(h1.len(), 5 + 32);
    }

    #[test]
    fn discovery_offer_is_public_expiring_and_never_contains_the_confirmation_code() {
        let now = chrono::Utc::now();
        let mut status = crate::remote::PairingStatus {
            remembered: false,
            trust_expires_at: None,
            enabled: true,
            paired: false,
            room_id: Some("a".repeat(32)),
            public_key: Some("public-key".into()),
            verify_code: Some("123456".into()),
            relay_url: Some("wss://relay.example/relay".into()),
            expires_at: Some((now + chrono::Duration::minutes(5)).to_rfc3339()),
        };
        let offer = super::discovery_offer(&status, now).expect("offer");
        assert_eq!(offer.as_object().expect("object").len(), 7);
        assert_eq!(offer["pairing_code"], "63311530");
        assert!(!offer.to_string().contains("123456"));
        assert!(offer.get("verify_code").is_none());
        status.paired = true;
        assert!(super::discovery_offer(&status, now).is_none());
        status.paired = false;
        status.enabled = false;
        assert!(super::discovery_offer(&status, now).is_none());
        status.enabled = true;
        status.expires_at = Some((now - chrono::Duration::seconds(1)).to_rfc3339());
        assert!(super::discovery_offer(&status, now).is_none());
    }

    #[test]
    fn remote_snapshot_reports_total_beyond_the_session_limit() {
        use chrono::Utc;
        use spawnt_core::{AgentId, Project, Session, SessionState, Surface};
        use spawnt_store::{Database, SourceCheckpoint};

        let mut db = Database::open_in_memory().expect("database");
        let project = Project::from_origin(None, "/test/project", None);
        let sessions: Vec<_> = (0..201)
            .map(|i| Session {
                id: format!("codex:{i}"),
                agent: AgentId::Codex,
                surface: Surface::Cli,
                project_id: project.id.clone(),
                parent_session_id: None,
                model: None,
                title: None,
                state: SessionState::Idle,
                pid: None,
                started_at: Utc::now(),
                last_event_at: Utc::now(),
            })
            .collect();
        let checkpoint = SourceCheckpoint {
            file_path: "/test/source".to_owned(),
            last_read_offset: 1,
            file_size: 1,
            fingerprint: "test".to_owned(),
            last_ingested_at: Utc::now(),
        };
        db.commit_batch(&[project], &sessions, &[], &[], &[], &checkpoint)
            .expect("commit");
        let snapshot = super::build_remote_snapshot(&db).expect("snapshot");
        assert_eq!(
            snapshot["sessions"].as_array().expect("sessions").len(),
            200
        );
        assert_eq!(snapshot["total_count"], 201);
        assert!(snapshot.get("total_pages").is_none());
    }

    #[test]
    fn remote_snapshot_contains_no_local_project_path_or_pairing_token() {
        use chrono::Utc;
        use spawnt_core::{AgentId, Project, Session, SessionState, Surface};
        use spawnt_store::{Database, SourceCheckpoint};

        let mut db = Database::open_in_memory().expect("database");
        let project = Project::from_origin(None, "/private/client/source", None);
        let session = Session {
            id: "codex:test".to_owned(),
            agent: AgentId::Codex,
            surface: Surface::Cli,
            project_id: project.id.clone(),
            parent_session_id: None,
            model: Some("gpt-6.1-sol".to_owned()),
            title: Some("private prompt title".to_owned()),
            state: SessionState::Thinking,
            pid: None,
            started_at: Utc::now(),
            last_event_at: Utc::now(),
        };
        let checkpoint = SourceCheckpoint {
            file_path: "/private/client/source/rollout.jsonl".to_owned(),
            last_read_offset: 1,
            file_size: 1,
            fingerprint: "test".to_owned(),
            last_ingested_at: Utc::now(),
        };
        let quota = spawnt_core::QuotaSnapshot {
            provider: "codex".into(),
            limit_id: "pool".into(),
            window: spawnt_core::QuotaWindow::FiveHour,
            used_pct: 10.0,
            resets_at: None,
            plan: None,
            basis: spawnt_core::TrustLevel::Exact,
            observed_at: Utc::now(),
        };
        db.commit_batch(&[project], &[session], &[], &[], &[quota], &checkpoint)
            .expect("commit");
        let snapshot = super::build_remote_snapshot(&db).expect("snapshot");
        let encoded = snapshot.to_string();
        let mut keys: Vec<_> = snapshot
            .as_object()
            .expect("snapshot object")
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "active_count",
                "generated_at",
                "ongoing_count",
                "quota",
                "sessions",
                "timeline",
                "timelines",
                "total_count"
            ]
        );
        let mut session_keys: Vec<_> = snapshot["sessions"][0]
            .as_object()
            .expect("session object")
            .keys()
            .map(String::as_str)
            .collect();
        session_keys.sort_unstable();
        assert_eq!(
            session_keys,
            [
                "agent",
                "cache_read_tokens",
                "context_fill_pct",
                "context_tokens",
                "context_window",
                "cost_basis",
                "id",
                "input_tokens",
                "is_active",
                "is_ongoing",
                "last_event_at",
                "model",
                "output_tokens",
                "presence",
                "priced_turns",
                "project_name",
                "started_at",
                "state",
                "surface",
                "total_cost_usd",
                "total_tokens",
                "unpriced_turns",
                "usage_available"
            ]
        );
        let mut quota_keys: Vec<_> = snapshot["quota"][0]
            .as_object()
            .expect("quota object")
            .keys()
            .map(String::as_str)
            .collect();
        quota_keys.sort_unstable();
        assert_eq!(
            quota_keys,
            [
                "basis",
                "limit_id",
                "observed_at",
                "plan",
                "provider",
                "remaining_pct",
                "resets_at",
                "used_pct",
                "window"
            ]
        );
        assert!(!encoded.contains("/private/client"));
        assert!(!encoded.contains("rollout.jsonl"));
        assert!(!encoded.contains("private prompt title"));
        assert!(snapshot.get("context_relays").is_none());
        assert!(snapshot.get("user_profile").is_none());
        assert!(!encoded.contains("paired_session"));
        assert!(!encoded.contains("relay_commands"));
        assert_eq!(snapshot["active_count"], 1);
        assert!(
            chrono::DateTime::parse_from_rfc3339(
                snapshot["generated_at"].as_str().expect("source timestamp")
            )
            .is_ok()
        );
    }
}
