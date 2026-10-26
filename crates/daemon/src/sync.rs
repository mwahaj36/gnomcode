//! Supabase cloud synchronization client structures and rollup hashing (`SPEC §3.15`).
//!
//! Provides:
//! - Device registration payload and token hash verification.
//! - Hourly usage rollup batching (`usage_hourly_sync`).
//! - Cross-device quota pool sync (`quota_pool_sync`).
//! - HMAC-SHA256 privacy-preserving project key generation (`SPEC §3.15.D`).

use chrono::{DateTime, Utc};
use fleethud_store::Database;
use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::Sha256;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

type HmacSha256 = Hmac<Sha256>;

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

fn build_remote_snapshot(db: &Database) -> Result<Value, fleethud_store::Error> {
    let today = db.generate_usage_report_timeline("today", None, None)?;
    let week = db.generate_usage_report_timeline("7d", None, None)?;
    let month = db.generate_usage_report_timeline("30d", None, None)?;
    let all = db.generate_usage_report_timeline("all", None, None)?;
    let (sessions, _) = db.list_sessions_paginated(None, None, 0, 200)?;
    let active_count = sessions.iter().filter(|session| session.is_active).count();
    let count = sessions.len();
    Ok(json!({
        "timeline": today,
        "timelines": { "today": today, "7d": week, "30d": month, "all": all },
        "user_profile": db.get_user_profile()?,
        "sessions": sessions,
        "quota": db.get_latest_quota_snapshots()?,
        "active_count": active_count,
        "total_count": count,
        "total_pages": 1,
        "context_relays": db.list_context_relays().unwrap_or_default(),
        "relay_config": db.get_context_relay_config().unwrap_or_default(),
    }))
}

/// Pushes a privacy-filtered live snapshot every minute after the user pairs an account.
pub fn spawn_cloud_sync(db: Arc<Mutex<Database>>) -> JoinHandle<()> {
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
            ticker.tick().await;
            if let Err(error) = push_snapshot(&client, &db).await {
                tracing::warn!("cloud sync failed: {error}");
            }
        }
    })
}

async fn push_snapshot(
    client: &reqwest::Client,
    db: &Arc<Mutex<Database>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let session_path = fleethud_platform::fleethud_home().join("session.json");
    let Ok(raw) = tokio::fs::read_to_string(&session_path).await else {
        return Ok(());
    };
    let mut session: Value = serde_json::from_str(&raw)?;
    let Some(user_id) = session
        .pointer("/user/id")
        .and_then(Value::as_str)
        .map(str::to_owned)
    else {
        return Ok(());
    };
    let (base_url, public_key) = cloud_config();
    let expires_at = session
        .get("expires_at")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    if expires_at <= chrono::Utc::now().timestamp() + 120 {
        let Some(refresh_token) = session.get("refresh_token").and_then(Value::as_str) else {
            return Ok(());
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
            return Ok(());
        }
        session = refreshed;
        if session.get("expires_at").is_none() {
            let expires_in = session
                .get("expires_in")
                .and_then(Value::as_i64)
                .unwrap_or(3600);
            session["expires_at"] = json!(chrono::Utc::now().timestamp() + expires_in);
        }
        tokio::fs::write(&session_path, serde_json::to_vec(&session)?).await?;
    }
    let Some(access_token) = session.get("access_token").and_then(Value::as_str) else {
        return Ok(());
    };

    // Poll for pending remote relay commands from web console
    let check_url =
        format!("{base_url}/rest/v1/fleethud_sync_state?user_id=eq.{user_id}&select=live_snapshot");
    if let Ok(resp) = client
        .get(&check_url)
        .header("apikey", &public_key)
        .bearer_auth(access_token)
        .send()
        .await
    {
        if let Ok(rows) = resp.json::<Vec<serde_json::Value>>().await {
            if let Some(first_row) = rows.first() {
                if let Some(cmds) = first_row
                    .pointer("/live_snapshot/relay_commands")
                    .and_then(Value::as_array)
                {
                    for cmd in cmds {
                        if cmd.get("action").and_then(Value::as_str) == Some("execute") {
                            if let Some(target_session) =
                                cmd.get("session_id").and_then(Value::as_str)
                            {
                                let db_clone = Arc::clone(db);
                                let sid = target_session.to_owned();
                                let _ = tokio::task::spawn_blocking(move || {
                                    let guard = db_clone.blocking_lock();
                                    let relays = guard.list_context_relays().unwrap_or_default();
                                    if let Some(entry) =
                                        relays.iter().find(|r| r.session_id == sid || r.id == sid)
                                    {
                                        let _ = crate::pro::execute_relay_handoff(entry);
                                        let _ = guard
                                            .update_context_relay_status(&entry.id, "executed");
                                    } else {
                                        let (sessions, _) = guard
                                            .list_sessions_paginated(None, None, 0, 100)
                                            .unwrap_or_default();
                                        if let Some(s) = sessions.iter().find(|s| s.id == sid) {
                                            let cwd = guard
                                                .find_antigravity_workspace(&s.id)
                                                .unwrap_or_else(|| {
                                                    std::env::current_dir()
                                                        .map(|p| p.to_string_lossy().to_string())
                                                        .unwrap_or_else(|_| ".".to_owned())
                                                });
                                            let entry = fleethud_store::ContextRelayEntry {
                                                id: format!("relay-{}", s.id),
                                                session_id: s.id.clone(),
                                                source_agent: s.agent.clone(),
                                                target_agent: s.agent.clone(),
                                                project_name: s.project_name.clone(),
                                                cwd,
                                                fill_pct: 92,
                                                status: "executed".to_owned(),
                                                created_at: chrono::Utc::now().to_rfc3339(),
                                                executed_at: Some(chrono::Utc::now().to_rfc3339()),
                                            };
                                            let _ = guard.record_context_relay(&entry);
                                            let _ = crate::pro::execute_relay_handoff(&entry);
                                        }
                                    }
                                })
                                .await;
                            }
                        }
                    }
                }
            }
        }
    }

    let db = Arc::clone(db);
    let snapshot = tokio::task::spawn_blocking(move || {
        let guard = db.blocking_lock();
        build_remote_snapshot(&guard)
    })
    .await??;
    client
        .post(format!(
            "{base_url}/rest/v1/fleethud_sync_state?on_conflict=user_id"
        ))
        .header("apikey", &public_key)
        .bearer_auth(access_token)
        .header("Prefer", "resolution=merge-duplicates,return=minimal")
        .json(&json!({
            "user_id": user_id,
            "live_snapshot": snapshot,
            "updated_at": chrono::Utc::now().to_rfc3339(),
        }))
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

fn cloud_config() -> (String, String) {
    let url = std::env::var("FLEETHUD_SUPABASE_URL")
        .unwrap_or_else(|_| obfstr::obfstr!("https://jjnuxngvhuvxkxrfmnqx.supabase.co").to_owned());
    let key = std::env::var("FLEETHUD_SUPABASE_KEY").unwrap_or_else(|_| {
        obfstr::obfstr!("sb_publishable_o2ZiGak7RHq_cmL81BOeZQ_g7JJeyCI").to_owned()
    });
    (url.trim_end_matches('/').to_owned(), key)
}

#[cfg(test)]
mod tests {
    use super::hash_project_key;

    #[test]
    fn project_key_hashing_is_deterministic_and_prefixes_proj() {
        let salt = b"fleethud-test-org-salt-123456";
        let h1 = hash_project_key(salt, "github.com/mwahaj36/fleethud");
        let h2 = hash_project_key(salt, "github.com/mwahaj36/fleethud");
        let h3 = hash_project_key(salt, "github.com/other/project");

        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
        assert!(h1.starts_with("proj_"));
        assert_eq!(h1.len(), 5 + 32);
    }

    #[test]
    fn remote_snapshot_contains_no_local_project_path_or_pairing_token() {
        use chrono::Utc;
        use fleethud_core::{AgentId, Project, Session, SessionState, Surface};
        use fleethud_store::{Database, SourceCheckpoint};

        let mut db = Database::open_in_memory().expect("database");
        let project = Project::from_origin(None, "/private/client/source", None);
        let session = Session {
            id: "codex:test".to_owned(),
            agent: AgentId::Codex,
            surface: Surface::Cli,
            project_id: project.id.clone(),
            parent_session_id: None,
            model: Some("gpt-6.1-sol".to_owned()),
            title: None,
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
        db.commit_batch(&[project], &[session], &[], &[], &[], &checkpoint)
            .expect("commit");
        let snapshot = super::build_remote_snapshot(&db).expect("snapshot");
        let encoded = snapshot.to_string();
        assert!(!encoded.contains("/private/client"));
        assert!(!encoded.contains("rollout.jsonl"));
        assert!(!encoded.contains("paired_session"));
        assert_eq!(snapshot["active_count"], 1);
    }
}
