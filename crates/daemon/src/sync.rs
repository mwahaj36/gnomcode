//! Supabase cloud synchronization client structures and rollup hashing (`SPEC §3.15`).
//!
//! Provides:
//! - Device registration payload and token hash verification.
//! - Hourly usage rollup batching (`usage_hourly_sync`).
//! - Cross-device quota pool sync (`quota_pool_sync`).
//! - HMAC-SHA256 privacy-preserving project key generation (`SPEC §3.15.D`).

use chrono::{DateTime, Utc};
use gnomcode_store::Database;
use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::Sha256;
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

#[derive(Clone, Serialize)]
pub(crate) struct UsageSyncStatus {
    state: &'static str,
    last_attempt_at: Option<String>,
    last_success_at: Option<String>,
    pushed_records: u64,
    message: &'static str,
}

static USAGE_SYNC_STATUS: Mutex<UsageSyncStatus> = Mutex::const_new(UsageSyncStatus {
    state: "off",
    last_attempt_at: None,
    last_success_at: None,
    pushed_records: 0,
    message: "Usage history backup is off.",
});

pub(crate) async fn usage_sync_status() -> UsageSyncStatus {
    USAGE_SYNC_STATUS.lock().await.clone()
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

fn build_remote_snapshot(db: &Database) -> Result<Value, gnomcode_store::Error> {
    let today = db.generate_usage_report_timeline("today", None, None)?;
    let week = db.generate_usage_report_timeline("7d", None, None)?;
    let month = db.generate_usage_report_timeline("30d", None, None)?;
    let calendar_month = db.generate_usage_report_timeline("month", None, None)?;
    let all = db.generate_usage_report_timeline("all", None, None)?;
    let profile = db.get_user_profile()?;
    let roi_by_timeline = json!({
        "today":crate::api::report_roi(&today,&profile,"today"),
        "7d":crate::api::report_roi(&week,&profile,"7d"),
        "30d":crate::api::report_roi(&month,&profile,"30d"),
        "month":crate::api::report_roi(&calendar_month,&profile,"month"),
        "all":crate::api::report_roi(&all,&profile,"all"),
    });
    let live_hosted = crate::remote::live_hosted_native_ids(db);
    let (sessions, count) =
        db.list_sessions_paginated_with_hosts(None, None, &live_hosted, 0, 200)?;
    let sessions: Vec<_> = sessions
        .into_iter()
        .map(|s| {
            json!({
                "id": s.id, "agent": s.agent, "surface": s.surface,
                "project_name": s.project_name, "model": s.model, "state": s.state,
                "is_active": s.is_active, "is_ongoing": s.is_ongoing, "presence": s.presence,
                "home_group": s.home_group,
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
    let (_, ongoing_count) =
        db.list_sessions_paginated_with_hosts(None, Some("ongoing"), &live_hosted, 0, 1)?;
    let (_, active_count) =
        db.list_sessions_paginated_with_hosts(None, Some("active"), &live_hosted, 0, 1)?;
    let activity_heatmap = db.activity_heatmap(Utc::now().date_naive())?;
    Ok(json!({
        "generated_at": Utc::now().to_rfc3339(),
        "timeline": today,
        "roi_by_timeline":roi_by_timeline,
        "timelines": { "today": today, "7d": week, "30d": month, "month": calendar_month, "all": all },
        "activity_heatmap": activity_heatmap,
        "sessions": sessions,
        "quota": crate::api::quota_views(db.get_latest_quota_snapshots()?),
        "active_count": active_count,
        "ongoing_count": ongoing_count,
        "total_count": count,
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
        // The device directory carries the desktop heartbeat. It runs on its own task so snapshot
        // and usage publication, which wait on the database, can never make a running desktop
        // look offline to signed-in phones.
        tokio::spawn({
            let (client, db, remote) = (client.clone(), db.clone(), remote.clone());
            async move {
                let mut devices_ticker = tokio::time::interval(Duration::from_secs(5));
                devices_ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
                let mut backoff_secs = 0u64;
                loop {
                    tokio::select! {
                        _ = devices_ticker.tick() => {},
                        () = remote.devices.changed() => {},
                    }
                    match poll_devices(&client, &db, &remote).await {
                        Ok(()) => backoff_secs = 0,
                        Err(error) => {
                            // Back off so a rejected login or an outage is not retried every 5s.
                            backoff_secs = (backoff_secs * 2).clamp(5, 60);
                            tracing::warn!(
                                "Device directory unavailable (retry in {backoff_secs}s): {error}"
                            );
                            tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
                        }
                    }
                }
            }
        });
        let mut ticker = tokio::time::interval(Duration::from_secs(60));
        let mut usage_backoff_secs = 0u64;
        let mut next_usage_attempt = tokio::time::Instant::now();
        loop {
            tokio::select! {
                _ = ticker.tick() => {},
                () = remote.discovery_changed() => {},
            }
            if !db
                .lock()
                .await
                .get_feature_toggles()
                .map(|toggles| toggles.account_sync_enabled)
                .unwrap_or(false)
            {
                let mut status = SYNC_STATUS.lock().await;
                status.state = "frozen";
                status.message =
                    "Account sync is turned off in Settings, so the web console has no snapshot.";
            } else {
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

            if tokio::time::Instant::now() >= next_usage_attempt {
                match push_usage_history_step(&client, &db, &remote).await {
                    Ok(true) => {
                        usage_backoff_secs = 0;
                        next_usage_attempt = tokio::time::Instant::now();
                    }
                    Ok(false) => {
                        usage_backoff_secs = 0;
                        next_usage_attempt = tokio::time::Instant::now();
                    }
                    Err(error) => {
                        usage_backoff_secs = if usage_backoff_secs == 0 {
                            120
                        } else {
                            (usage_backoff_secs * 2).min(900)
                        };
                        next_usage_attempt =
                            tokio::time::Instant::now() + Duration::from_secs(usage_backoff_secs);
                        tracing::warn!(
                            "usage history sync failed (backing off {usage_backoff_secs}s): {error}"
                        );
                    }
                }
            }
        }
    })
}

async fn poll_devices(
    client: &reqwest::Client,
    db: &Arc<Mutex<Database>>,
    remote: &crate::remote::RemoteManager,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = gnomcode_platform::gnomcode_home().join("session.json");
    if let Some(auth) = read_cloud_auth(client, &path).await? {
        remote.devices.poll(client, &auth, db.clone()).await?;
        let devices = remote.devices.view().await;
        if devices["account_access"] == true && devices["remote_enabled"] == true {
            push_notifications(client, db, &auth).await?;
        }
    } else {
        remote.devices.disconnect().await?;
    }
    Ok(())
}

async fn push_notifications(
    client: &reqwest::Client,
    db: &Arc<Mutex<Database>>,
    auth: &CloudAuth,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let Some(brand) = gnomcode_core::branding::branding() else {
        return Ok(());
    };
    let shared = db.clone();
    let pending = tokio::task::spawn_blocking(move || {
        shared.blocking_lock().pending_push_notices_with_meta()
    })
    .await??;
    for notice in pending {
        let body = match notice.payload {
            Some(payload) => json!({
                "event_id": notice.id,
                "kind": "agent_event",
                "event": payload.event,
                "agent": payload.agent,
            }),
            None => json!({"event_id": notice.id, "kind": "agent_event"}),
        };
        let response = client
            .post(format!("{}/api/push/send", brand.base_url))
            .bearer_auth(&auth.access_token)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;
        let accepted: Value = response.json().await?;
        if accepted["ok"] == true {
            let shared = db.clone();
            let id = notice.id;
            tokio::task::spawn_blocking(move || shared.blocking_lock().mark_notice_pushed(&id))
                .await??;
        }
    }
    Ok(())
}

async fn push_snapshot(
    client: &reqwest::Client,
    db: &Arc<Mutex<Database>>,
    remote: &crate::remote::RemoteManager,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let session_path = gnomcode_platform::gnomcode_home().join("session.json");
    let Some(CloudAuth {
        user_id,
        access_token,
        base_url,
        public_key,
    }) = read_cloud_auth(client, &session_path).await?
    else {
        return Ok(false);
    };

    let desktop = remote.devices.snapshot_desktop_id(&user_id).await?;
    let db = Arc::clone(db);
    let mut snapshot = tokio::task::spawn_blocking(move || {
        let guard = db.blocking_lock();
        build_remote_snapshot(&guard)
    })
    .await??;
    snapshot["remote_offer"] = Value::Null;
    client
        .patch(format!(
            "{base_url}/rest/v1/spawnt_desktops?user_id=eq.{user_id}&id=eq.{desktop}"
        ))
        .header("apikey", &public_key)
        .bearer_auth(&access_token)
        .header("Prefer", "resolution=merge-duplicates,return=minimal")
        .json(&json!({
            "live_snapshot": snapshot,
        }))
        .send()
        .await?
        .error_for_status()?;
    Ok(true)
}

const USAGE_SYNC_BATCH: usize = 500;

/// Builds privacy-bounded Supabase row payloads for `gnomcode_session_meta` and `gnomcode_usage_record`.
pub(crate) fn build_usage_sync_payloads(
    user_id: &str,
    desktop_id: &str,
    sessions: &[gnomcode_store::CloudSessionMeta],
    records: &[gnomcode_store::CloudUsageRecord],
) -> (Vec<Value>, Vec<Value>) {
    let session_rows = sessions
        .iter()
        .map(|s| {
            json!({
                "user_id": user_id,
                "session_id": s.session_id,
                "agent": s.agent,
                "surface": s.surface,
                "model": s.model,
                "project_name": s.project_name,
                "started_at": s.started_at,
                "last_event_at": s.last_event_at,
            })
        })
        .collect();
    let usage_rows = records
        .iter()
        .map(|r| {
            json!({
                "user_id": user_id,
                "desktop_id": desktop_id,
                "dedup_key": r.dedup_key,
                "session_id": r.session_id,
                "timestamp": r.timestamp,
                "model": r.model,
                "input_tokens": r.input_tokens,
                "output_tokens": r.output_tokens,
                "reasoning_tokens": r.reasoning_tokens,
                "cache_read_tokens": r.cache_read_tokens,
                "cache_write_5m_tokens": r.cache_write_5m_tokens,
                "cache_write_1h_tokens": r.cache_write_1h_tokens,
                "context_tokens": r.context_tokens,
                "context_window": r.context_window,
                "cost_usd": r.cost_usd,
                "cache_savings_usd": r.cache_savings_usd,
                "cost_basis": r.cost_basis,
                "pricing_version": r.pricing_version,
            })
        })
        .collect();
    (session_rows, usage_rows)
}

async fn push_usage_history_step(
    client: &reqwest::Client,
    db: &Arc<Mutex<Database>>,
    remote: &crate::remote::RemoteManager,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let enabled = {
        let shared = Arc::clone(db);
        tokio::task::spawn_blocking(move || {
            shared
                .blocking_lock()
                .get_feature_toggles()
                .map(|toggles| toggles.usage_history_sync_enabled)
        })
        .await??
    };
    if !enabled {
        let mut status = USAGE_SYNC_STATUS.lock().await;
        status.state = "off";
        status.message = "Usage history backup is off.";
        return Ok(false);
    }

    let session_path = gnomcode_platform::gnomcode_home().join("session.json");
    let Some(auth) = read_cloud_auth(client, &session_path).await? else {
        let mut status = USAGE_SYNC_STATUS.lock().await;
        status.state = "unpaired";
        status.message = "Connect an account to back up usage history.";
        return Ok(false);
    };

    let desktop_id = remote.devices.snapshot_desktop_id(&auth.user_id).await?;
    {
        let mut status = USAGE_SYNC_STATUS.lock().await;
        status.state = "syncing";
        status.last_attempt_at = Some(Utc::now().to_rfc3339());
        status.message = "Backing up usage records.";
    }

    let result = push_usage_batches(client, db, &auth, &desktop_id).await;
    let mut status = USAGE_SYNC_STATUS.lock().await;
    match result {
        Ok(pushed) => {
            status.state = "connected";
            status.last_success_at = Some(Utc::now().to_rfc3339());
            status.pushed_records = status.pushed_records.saturating_add(pushed);
            status.message = "Usage history is backed up.";
            Ok(true)
        }
        Err(error) => {
            status.state = "error";
            status.message = "Usage history backup failed; will retry automatically.";
            Err(error)
        }
    }
}

pub(crate) async fn push_usage_batches(
    client: &reqwest::Client,
    db: &Arc<Mutex<Database>>,
    auth: &CloudAuth,
    desktop_id: &str,
) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
    let mut total_pushed = 0u64;
    loop {
        let shared = Arc::clone(db);
        let (mut watermark, sessions, records) = tokio::task::spawn_blocking(move || {
            let guard = shared.blocking_lock();
            let wm = guard.get_usage_sync_watermark()?;
            let sessions = guard
                .read_session_meta_batch(wm.last_session_event_at.as_deref(), USAGE_SYNC_BATCH)?;
            let records = guard.read_usage_sync_batch(wm.last_usage_rowid, USAGE_SYNC_BATCH)?;
            Ok::<_, gnomcode_store::Error>((wm, sessions, records))
        })
        .await??;

        if sessions.is_empty() && records.is_empty() {
            break;
        }

        let raw_records: Vec<gnomcode_store::CloudUsageRecord> =
            records.iter().map(|(_, r)| r.clone()).collect();
        let (session_rows, usage_rows) =
            build_usage_sync_payloads(&auth.user_id, desktop_id, &sessions, &raw_records);

        if !session_rows.is_empty() {
            client
                .post(format!(
                    "{}/rest/v1/gnomcode_session_meta?on_conflict=user_id,session_id",
                    auth.base_url
                ))
                .header("apikey", &auth.public_key)
                .bearer_auth(&auth.access_token)
                .header("Prefer", "resolution=merge-duplicates,return=minimal")
                .json(&session_rows)
                .send()
                .await?
                .error_for_status()?;
            if let Some(last) = sessions.last() {
                watermark.last_session_event_at = Some(last.last_event_at.clone());
            }
        }

        if !usage_rows.is_empty() {
            client
                .post(format!(
                    "{}/rest/v1/gnomcode_usage_record?on_conflict=user_id,dedup_key",
                    auth.base_url
                ))
                .header("apikey", &auth.public_key)
                .bearer_auth(&auth.access_token)
                .header("Prefer", "resolution=merge-duplicates,return=minimal")
                .json(&usage_rows)
                .send()
                .await?
                .error_for_status()?;
            if let Some((last_rowid, _)) = records.last() {
                watermark.last_usage_rowid = *last_rowid;
            }
            total_pushed = total_pushed.saturating_add(usage_rows.len() as u64);
        }

        let shared = Arc::clone(db);
        tokio::task::spawn_blocking(move || {
            shared.blocking_lock().set_usage_sync_watermark(&watermark)
        })
        .await??;

        if sessions.len() < USAGE_SYNC_BATCH && records.len() < USAGE_SYNC_BATCH {
            break;
        }
    }
    Ok(total_pushed)
}

#[derive(Serialize)]
pub(crate) struct RestoreUsageSummary {
    pub ok: bool,
    pub restored_records: usize,
    pub restored_sessions: usize,
    pub cloud_records: usize,
}

pub(crate) async fn restore_usage_history(
    db: &Arc<Mutex<Database>>,
) -> Result<RestoreUsageSummary, (axum::http::StatusCode, String)> {
    use axum::http::StatusCode;
    let client = reqwest::Client::builder()
        .https_only(true)
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let session_path = gnomcode_platform::gnomcode_home().join("session.json");
    let auth = read_cloud_auth(&client, &session_path)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?
        .ok_or((
            StatusCode::PRECONDITION_REQUIRED,
            "Sign in to your account before restoring usage history.".to_owned(),
        ))?;

    let mut sessions = Vec::new();
    let mut offset = 0usize;
    loop {
        let page: Vec<gnomcode_store::CloudSessionMeta> = client
            .get(format!(
                "{}/rest/v1/gnomcode_session_meta?user_id=eq.{}&select=session_id,agent,surface,model,project_name,started_at,last_event_at&order=started_at.asc&limit=1000&offset={offset}",
                auth.base_url, auth.user_id
            ))
            .header("apikey", &auth.public_key)
            .bearer_auth(&auth.access_token)
            .send()
            .await
            .map_err(|e| (StatusCode::BAD_GATEWAY, format!("Could not fetch cloud sessions: {e}")))?
            .error_for_status()
            .map_err(|e| (StatusCode::BAD_GATEWAY, format!("Cloud session query failed: {e}")))?
            .json()
            .await
            .map_err(|e| (StatusCode::BAD_GATEWAY, format!("Invalid cloud session payload: {e}")))?;
        let len = page.len();
        sessions.extend(page);
        if len < 1000 {
            break;
        }
        offset += 1000;
    }

    let mut records = Vec::new();
    let mut offset = 0usize;
    loop {
        let page: Vec<gnomcode_store::CloudUsageRecord> = client
            .get(format!(
                "{}/rest/v1/gnomcode_usage_record?user_id=eq.{}&select=dedup_key,session_id,timestamp,model,input_tokens,output_tokens,reasoning_tokens,cache_read_tokens,cache_write_5m_tokens,cache_write_1h_tokens,context_tokens,context_window,cost_usd,cache_savings_usd,cost_basis,pricing_version&order=timestamp.asc&limit=1000&offset={offset}",
                auth.base_url, auth.user_id
            ))
            .header("apikey", &auth.public_key)
            .bearer_auth(&auth.access_token)
            .send()
            .await
            .map_err(|e| (StatusCode::BAD_GATEWAY, format!("Could not fetch cloud usage records: {e}")))?
            .error_for_status()
            .map_err(|e| (StatusCode::BAD_GATEWAY, format!("Cloud usage query failed: {e}")))?
            .json()
            .await
            .map_err(|e| (StatusCode::BAD_GATEWAY, format!("Invalid cloud usage payload: {e}")))?;
        let len = page.len();
        records.extend(page);
        if len < 1000 {
            break;
        }
        offset += 1000;
    }

    let cloud_records = records.len();
    let shared = Arc::clone(db);
    let (restored_records, restored_sessions) = tokio::task::spawn_blocking(move || {
        shared
            .blocking_lock()
            .restore_cloud_usage_history(&sessions, &records)
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(RestoreUsageSummary {
        ok: true,
        restored_records,
        restored_sessions,
        cloud_records,
    })
}

pub(crate) async fn delete_cloud_usage_history(
    db: &Arc<Mutex<Database>>,
) -> Result<(), (axum::http::StatusCode, String)> {
    use axum::http::StatusCode;
    let client = reqwest::Client::builder()
        .https_only(true)
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let session_path = gnomcode_platform::gnomcode_home().join("session.json");
    let auth = read_cloud_auth(&client, &session_path)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?
        .ok_or((
            StatusCode::PRECONDITION_REQUIRED,
            "Sign in to your account before deleting cloud usage history.".to_owned(),
        ))?;

    client
        .delete(format!(
            "{}/rest/v1/gnomcode_usage_record?user_id=eq.{}",
            auth.base_url, auth.user_id
        ))
        .header("apikey", &auth.public_key)
        .bearer_auth(&auth.access_token)
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Failed to delete cloud usage records: {e}"),
            )
        })?
        .error_for_status()
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Cloud delete rejected: {e}"),
            )
        })?;

    client
        .delete(format!(
            "{}/rest/v1/gnomcode_session_meta?user_id=eq.{}",
            auth.base_url, auth.user_id
        ))
        .header("apikey", &auth.public_key)
        .bearer_auth(&auth.access_token)
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Failed to delete cloud session metadata: {e}"),
            )
        })?
        .error_for_status()
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Cloud session delete rejected: {e}"),
            )
        })?;

    let shared = Arc::clone(db);
    tokio::task::spawn_blocking(move || {
        let guard = shared.blocking_lock();
        guard.reset_usage_sync_watermark()?;
        let mut toggles = guard.get_feature_toggles()?;
        toggles.usage_history_sync_enabled = false;
        guard.set_feature_toggles(&toggles)
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut status = USAGE_SYNC_STATUS.lock().await;
    status.state = "off";
    status.pushed_records = 0;
    status.message = "Cloud usage history deleted and backup turned off.";
    Ok(())
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
        match gnomcode_platform::open_exclusive_lock(&lock_path) {
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
    gnomcode_platform::write_private_file(
        path,
        &serde_json::to_vec(&stored_auth_session(session))?,
    )?;
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
        gnomcode_platform::write_private_file(
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
    let brand = gnomcode_core::branding::branding();
    let url = std::env::var("GNOMCODE_SUPABASE_URL")
        .unwrap_or_else(|_| brand.map(|b| b.supabase_url.clone()).unwrap_or_default());
    let key = std::env::var("GNOMCODE_SUPABASE_KEY").unwrap_or_else(|_| {
        brand
            .map(|b| b.supabase_publishable_key.clone())
            .unwrap_or_default()
    });
    (url.trim_end_matches('/').to_owned(), key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gnomcode_core::{
        AgentId, Project, Session, SessionState, Surface, TrustLevel, UsageRecord,
    };
    use gnomcode_store::SourceCheckpoint;

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
        let salt = b"gnomcode-test-org-salt-123456";
        let h1 = hash_project_key(salt, "github.com/mwahaj36/gnomcode");
        let h2 = hash_project_key(salt, "github.com/mwahaj36/gnomcode");
        let h3 = hash_project_key(salt, "github.com/other/project");

        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
        assert!(h1.starts_with("proj_"));
        assert_eq!(h1.len(), 5 + 32);
    }

    #[test]
    fn remote_snapshot_reports_total_beyond_the_session_limit() {
        use chrono::Utc;
        use gnomcode_core::{AgentId, Project, Session, SessionState, Surface};
        use gnomcode_store::{Database, SourceCheckpoint};

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
        assert!(
            snapshot["roi_by_timeline"]["all"]["roi_by_provider"].is_array(),
            "cloud reports need Rust-computed ROI for each timeline"
        );
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
        use gnomcode_core::{AgentId, Project, Session, SessionState, Surface};
        use gnomcode_store::{Database, SourceCheckpoint};

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
        let quota = gnomcode_core::QuotaSnapshot {
            provider: "codex".into(),
            limit_id: "pool".into(),
            window: gnomcode_core::QuotaWindow::FiveHour,
            used_pct: 10.0,
            resets_at: None,
            plan: None,
            basis: gnomcode_core::TrustLevel::Exact,
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
                "activity_heatmap",
                "generated_at",
                "ongoing_count",
                "quota",
                "roi_by_timeline",
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
                "home_group",
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
                "label",
                "limit_id",
                "observed_at",
                "pace",
                "plan",
                "pool",
                "provider",
                "remaining_pct",
                "reset_passed",
                "resets_at",
                "stale",
                "state",
                "used_pct",
                "window",
                "window_label"
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

    #[test]
    fn usage_sync_payload_excludes_titles_and_chat_content() {
        let mut db = Database::open_in_memory().expect("db");
        let project = Project::from_origin(None, "C:\\private\\secret-project", None);
        let session = Session {
            id: "codex:payload-check".to_owned(),
            agent: AgentId::Codex,
            surface: Surface::DesktopApp,
            project_id: project.id.clone(),
            parent_session_id: None,
            model: Some("gpt-5".to_owned()),
            title: Some("Top secret user prompt title".to_owned()),
            state: SessionState::Idle,
            pid: None,
            started_at: Utc::now(),
            last_event_at: Utc::now(),
        };
        let cp = SourceCheckpoint {
            file_path: "rollout.jsonl".to_owned(),
            last_read_offset: 0,
            file_size: 0,
            fingerprint: "fp".to_owned(),
            last_ingested_at: Utc::now(),
        };
        let record = UsageRecord {
            dedup_key: "codex:payload-check:1".to_owned(),
            session_id: "codex:payload-check".to_owned(),
            timestamp: Utc::now(),
            model: "gpt-5".to_owned(),
            input_tokens: 42,
            output_tokens: 18,
            reasoning_tokens: 0,
            cache_read_tokens: 10,
            cache_write_5m_tokens: 0,
            cache_write_1h_tokens: 0,
            context_tokens: 60,
            context_window: Some(200_000),
            cost_usd: 0.004,
            cache_savings_usd: 0.001,
            cost_basis: TrustLevel::Exact,
            pricing_version: "v1".to_owned(),
        };
        db.commit_batch(&[project], &[session], &[record], &[], &[], &cp)
            .expect("commit");

        let sessions = db.read_session_meta_batch(None, 10).expect("sessions");
        let records: Vec<_> = db
            .read_usage_sync_batch(0, 10)
            .expect("records")
            .into_iter()
            .map(|(_, r)| r)
            .collect();
        let (session_rows, usage_rows) =
            build_usage_sync_payloads("user-1", "desk-1", &sessions, &records);
        let encoded = serde_json::to_string(&json!({
            "sessions": session_rows,
            "records": usage_rows,
        }))
        .expect("json");
        assert!(!encoded.contains("Top secret user prompt title"));
        assert!(!encoded.contains("\"title\""));
        assert!(!encoded.contains("C:\\private\\secret-project"));
        assert!(encoded.contains("secret-project"));
        assert!(encoded.contains("codex:payload-check:1"));
    }

    #[tokio::test]
    async fn failed_usage_sync_push_fails_open_and_does_not_advance_watermark() {
        let mut db = Database::open_in_memory().expect("db");
        let project = Project::from_origin(None, "C:\\work\\failopen", None);
        let session = Session {
            id: "codex:failopen".to_owned(),
            agent: AgentId::Codex,
            surface: Surface::Cli,
            project_id: project.id.clone(),
            parent_session_id: None,
            model: Some("gpt-5".to_owned()),
            title: None,
            state: SessionState::Idle,
            pid: None,
            started_at: Utc::now(),
            last_event_at: Utc::now(),
        };
        let cp = SourceCheckpoint {
            file_path: "rollout.jsonl".to_owned(),
            last_read_offset: 0,
            file_size: 0,
            fingerprint: "fp".to_owned(),
            last_ingested_at: Utc::now(),
        };
        let record = UsageRecord {
            dedup_key: "codex:failopen:1".to_owned(),
            session_id: "codex:failopen".to_owned(),
            timestamp: Utc::now(),
            model: "gpt-5".to_owned(),
            input_tokens: 10,
            output_tokens: 5,
            reasoning_tokens: 0,
            cache_read_tokens: 0,
            cache_write_5m_tokens: 0,
            cache_write_1h_tokens: 0,
            context_tokens: 15,
            context_window: None,
            cost_usd: 0.001,
            cache_savings_usd: 0.0,
            cost_basis: TrustLevel::Exact,
            pricing_version: "v1".to_owned(),
        };
        db.commit_batch(&[project], &[session], &[record], &[], &[], &cp)
            .expect("commit");
        let shared = Arc::new(Mutex::new(db));
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(200))
            .build()
            .expect("client");
        let auth = CloudAuth {
            user_id: "00000000-0000-0000-0000-000000000000".to_owned(),
            access_token: "token".to_owned(),
            base_url: "https://127.0.0.1:9".to_owned(),
            public_key: "key".to_owned(),
        };
        let res = push_usage_batches(&client, &shared, &auth, "desk-1").await;
        assert!(res.is_err());
        assert_eq!(
            shared
                .lock()
                .await
                .get_usage_sync_watermark()
                .expect("wm")
                .last_usage_rowid,
            0
        );
    }
}
