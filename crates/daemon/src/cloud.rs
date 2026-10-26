//! Opt-in cloud session presence. Heartbeats never imply agent activity or measured usage.

use chrono::Utc;
use fleethud_core::{AgentId, Project, Session, SessionState, Surface};
use fleethud_store::{Database, SessionListItem, SourceCheckpoint};
use std::path::Path;

/// Creates a verified HTTPS-only client with a bounded request timeout.
///
/// # Errors
/// Returns a TLS/client initialization error.
pub fn client() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .https_only(true)
        .timeout(std::time::Duration::from_secs(10))
        .build()
}

/// Starts cloud presence automatically inside a daemon whose runtime exposes an identity.
/// Uses the daemon's existing account pairing; unpaired collection remains local.
#[must_use]
pub fn spawn_auto_collector(db_path: std::path::PathBuf) -> Option<tokio::task::JoinHandle<()>> {
    let (agent, id) = fleethud_platform::cloud_session_identity()?;
    if !valid_session_id(&id) {
        return None;
    }
    Some(spawn_collector_for(
        db_path,
        agent,
        id,
        fleethud_platform::fleethud_home().join("session.json"),
    ))
}

fn spawn_collector_for(
    db_path: std::path::PathBuf,
    agent: AgentId,
    id: String,
    pairing: std::path::PathBuf,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let Ok(client) = client() else {
            tracing::warn!("cloud collector could not initialize HTTPS");
            return;
        };
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            ticker.tick().await;
            let path = db_path.clone();
            let native_id = id.clone();
            let result = tokio::task::spawn_blocking(move || {
                let mut db = Database::open(&path)?;
                heartbeat(&mut db, agent, &native_id)
            })
            .await;
            match result {
                Ok(Ok(session)) => {
                    if publish(&client, &pairing, &session).await.is_err() {
                        tracing::warn!("cloud presence sync unavailable; retrying automatically");
                    }
                }
                _ => tracing::warn!("cloud heartbeat unavailable; retrying automatically"),
            }
        }
    })
}

/// Validates an opaque provider session ID; arbitrary text is never accepted as metadata.
#[must_use]
pub fn valid_session_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 200
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

/// Records collector presence without creating usage or activity events.
///
/// # Errors
/// Returns a storage error if the heartbeat cannot be committed.
pub fn heartbeat(
    db: &mut Database,
    agent: AgentId,
    native_id: &str,
) -> Result<SessionListItem, fleethud_store::Error> {
    record_at(db, agent, native_id, Utc::now())
}

fn record_at(
    db: &mut Database,
    agent: AgentId,
    native_id: &str,
    now: chrono::DateTime<Utc>,
) -> Result<SessionListItem, fleethud_store::Error> {
    let project = Project::from_origin(None, "cloud-workspace", None);
    let id = Session::compose_id(agent, native_id);
    let session = Session {
        id: id.clone(),
        agent,
        surface: Surface::Cloud,
        project_id: project.id.clone(),
        parent_session_id: None,
        model: None,
        title: None,
        state: SessionState::Connected,
        pid: None,
        started_at: now,
        last_event_at: now,
    };
    let checkpoint = SourceCheckpoint {
        file_path: format!("cloud-presence:{id}"),
        last_read_offset: 0,
        file_size: 0,
        fingerprint: "presence-only".to_owned(),
        last_ingested_at: now,
    };
    db.commit_batch(&[project], &[session], &[], &[], &[], &checkpoint)?;
    db.mark_cloud_session(&id)?;
    // Read the saved start timestamp rather than resetting it on every heartbeat.
    let (items, _) = db.list_sessions_paginated(Some(agent.as_str()), None, 0, 10000)?;
    items.into_iter().find(|item| item.id == id).ok_or_else(|| {
        fleethud_store::Error::Migration("cloud heartbeat missing after commit".to_owned())
    })
}

/// Sends one presence record through the user's existing Supabase login.
/// Returns false when no pairing file exists. Transport failures never affect the agent.
///
/// # Errors
/// Returns an authentication, file, or transport error; callers should retry with backoff.
pub async fn publish(
    client: &reqwest::Client,
    pairing_file: &Path,
    session: &SessionListItem,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let Some(auth) = crate::sync::read_cloud_auth(client, pairing_file).await? else {
        return Ok(false);
    };
    publish_authenticated(client, auth, session).await?;
    Ok(true)
}

async fn publish_authenticated(
    client: &reqwest::Client,
    auth: crate::sync::CloudAuth,
    session: &SessionListItem,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut session = session.clone();
    session.project_name = "Cloud workspace".to_owned();
    session.title = None;
    session.device_name = None;
    client
        .post(format!(
            "{}/rest/v1/fleethud_cloud_sessions?on_conflict=user_id,session_id",
            auth.base_url
        ))
        .header("apikey", &auth.public_key)
        .bearer_auth(&auth.access_token)
        .header("Prefer", "resolution=merge-duplicates,return=minimal")
        .json(&serde_json::json!({
            "user_id": auth.user_id, "session_id": session.id, "session": session,
        }))
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

/// Per-invocation token deltas exported by an agent integration. Content fields are rejected.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CloudUsageEvent {
    event_id: String,
    timestamp: chrono::DateTime<Utc>,
    model: String,
    input_tokens: u64,
    output_tokens: u64,
    #[serde(default)]
    cache_read_tokens: u64,
    #[serde(default)]
    cache_write_5m_tokens: u64,
    #[serde(default)]
    cache_write_1h_tokens: u64,
    #[serde(default)]
    reasoning_tokens: u64,
}

/// Imports complete metadata-only JSONL lines, with durable offsets and invocation deduplication.
/// Counts must be deltas, with cached input separated from uncached input. Raw transcripts
/// must use the native adapters instead. Partial lines wait for the next heartbeat.
///
/// # Errors
/// Returns an I/O, schema or database error without committing an invalid batch.
pub fn ingest_usage_file(
    db: &mut Database,
    agent: AgentId,
    native_id: &str,
    path: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use fleethud_core::UsageRecord;
    use fleethud_core::pricing::{TokenBreakdown, calculate_cost};
    let id = Session::compose_id(agent, native_id);
    let key = format!("cloud-usage:{id}:{}", path.to_string_lossy());
    let offset = db.get_checkpoint(&key)?.map_or(0, |cp| cp.last_read_offset);
    let size = std::fs::metadata(path)?.len();
    if size.saturating_sub(offset.min(size)) > 8 * 1024 * 1024 {
        return Err("usage batch exceeds 8 MiB; rotate the metadata file".into());
    }
    let read = fleethud_platform::read_from_offset(path, offset)?;
    let length = read
        .bytes
        .iter()
        .rposition(|b| *b == b'\n')
        .map_or(0, |i| i + 1);
    if length == 0 {
        return Ok(());
    }
    let mut records = Vec::new();
    for line in read.bytes[..length].split(|b| *b == b'\n') {
        if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        let event: CloudUsageEvent = serde_json::from_slice(line)?;
        if !valid_session_id(&event.event_id)
            || event.model.is_empty()
            || event.model.len() > 128
            || !event
                .model
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.:/ ".contains(&b))
        {
            return Err("invalid usage metadata identifier".into());
        }
        let usage = TokenBreakdown {
            input_tokens: event.input_tokens,
            output_tokens: event.output_tokens,
            reasoning_tokens: event.reasoning_tokens,
            cache_read_tokens: event.cache_read_tokens,
            cache_write_5m_tokens: event.cache_write_5m_tokens,
            cache_write_1h_tokens: event.cache_write_1h_tokens,
        };
        let context = usage
            .input_tokens
            .checked_add(usage.cache_read_tokens)
            .and_then(|v| v.checked_add(usage.cache_write_5m_tokens))
            .and_then(|v| v.checked_add(usage.cache_write_1h_tokens))
            .ok_or("token counter overflow")?;
        if [context, usage.output_tokens, usage.reasoning_tokens]
            .iter()
            .any(|v| *v > i64::MAX as u64)
            || usage.reasoning_tokens > usage.output_tokens
        {
            return Err("invalid token counter".into());
        }
        let cost = calculate_cost(&event.model, &usage);
        records.push(UsageRecord {
            dedup_key: format!("cloud-usage:{id}:{}", event.event_id),
            session_id: id.clone(),
            timestamp: event.timestamp,
            model: event.model,
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            reasoning_tokens: usage.reasoning_tokens,
            cache_read_tokens: usage.cache_read_tokens,
            cache_write_5m_tokens: usage.cache_write_5m_tokens,
            cache_write_1h_tokens: usage.cache_write_1h_tokens,
            context_tokens: context,
            context_window: cost.context_window,
            cost_usd: cost.cost_usd,
            cache_savings_usd: cost.cache_savings_usd,
            cost_basis: cost.cost_basis,
            pricing_version: cost.pricing_version.to_owned(),
        });
    }
    db.commit_batch(
        &[],
        &[],
        &records,
        &[],
        &[],
        &SourceCheckpoint {
            file_path: key,
            last_read_offset: read.effective_start_offset + length as u64,
            file_size: read.file_size,
            fingerprint: "metadata-jsonl-v1".to_owned(),
            last_ingested_at: Utc::now(),
        },
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_content_and_unbounded_identifiers() {
        for id in [
            "",
            "hello world",
            "../secret",
            "token=value",
            &"x".repeat(201),
        ] {
            assert!(!valid_session_id(id));
        }
        assert!(valid_session_id("thread-123_abc.456"));
    }

    #[test]
    fn heartbeat_is_idempotent_and_does_not_invent_usage_or_activity() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut db = Database::open(&dir.path().join("state.db")).expect("database");
        let first = heartbeat(&mut db, AgentId::Codex, "example-thread").expect("first heartbeat");
        let next = heartbeat(&mut db, AgentId::Codex, "example-thread").expect("repeat heartbeat");
        assert_eq!(first.id, next.id);
        assert_eq!(first.started_at, next.started_at);
        assert_eq!(next.state, "connected");
        assert!(!next.is_active);
        assert!(!next.usage_available);
        assert_eq!(next.input_tokens, 0);
        assert_eq!(next.total_cost_usd, 0.0);
        assert_eq!(
            db.list_sessions_paginated(None, None, 0, 10)
                .expect("sessions")
                .1,
            1
        );
    }
    #[test]
    fn all_agents_support_presence_and_expire_without_becoming_active() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut db = Database::open(&dir.path().join("state.db")).expect("database");
        for agent in [
            AgentId::Codex,
            AgentId::ClaudeCode,
            AgentId::Cursor,
            AgentId::Antigravity,
            AgentId::GeminiCli,
            AgentId::Cline,
            AgentId::Aider,
        ] {
            let item = record_at(
                &mut db,
                agent,
                "cloud-id",
                Utc::now() - chrono::Duration::seconds(120),
            )
            .expect("heartbeat");
            assert_eq!(item.state, "disconnected");
            assert!(!item.is_active);
            assert!(!item.usage_available);
        }
        assert_eq!(
            db.list_sessions_paginated(None, Some("active"), 0, 10)
                .expect("active")
                .1,
            0
        );
        assert_eq!(
            db.list_sessions_paginated(None, None, 0, 10)
                .expect("all")
                .1,
            7
        );
    }

    #[tokio::test]
    async fn publishes_only_metadata_with_authenticated_session_identity() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("bind");
        let address = listener.local_addr().expect("address");
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .expect("timeout");
            let mut request = Vec::new();
            let mut buffer = [0; 4096];
            loop {
                let count = stream.read(&mut buffer).expect("read");
                assert!(count > 0);
                request.extend_from_slice(&buffer[..count]);
                if let Some(end) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&request[..end]).to_ascii_lowercase();
                    let length: usize = headers
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length: "))
                        .expect("length")
                        .parse()
                        .expect("number");
                    if request.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            stream
                .write_all(
                    b"HTTP/1.1 201 Created\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .expect("response");
            request
        });
        let dir = tempfile::tempdir().expect("temp dir");
        let mut db = Database::open(&dir.path().join("state.db")).expect("db");
        let session = heartbeat(&mut db, AgentId::Codex, "example").expect("heartbeat");
        let auth = crate::sync::CloudAuth {
            user_id: "test-user".to_owned(),
            access_token: "test-token".to_owned(),
            public_key: "public-test-key".to_owned(),
            base_url: format!("http://{address}"),
        };
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("client");
        publish_authenticated(&client, auth, &session)
            .await
            .expect("publish");
        let request = server.join().expect("server");
        let end = request
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .expect("header end");
        let headers = String::from_utf8_lossy(&request[..end]).to_ascii_lowercase();
        assert!(
            headers.starts_with(
                "post /rest/v1/fleethud_cloud_sessions?on_conflict=user_id,session_id "
            )
        );
        assert!(headers.contains("authorization: bearer test-token"));
        let payload: serde_json::Value = serde_json::from_slice(&request[end + 4..]).expect("json");
        assert_eq!(payload["session_id"], "codex:example");
        assert_eq!(payload["user_id"], "test-user");
        assert_eq!(payload["session"]["usage_available"], false);
        assert!(payload["session"].get("repo_root").is_none());
        assert!(payload["session"].get("prompt").is_none());
    }
    #[test]
    fn real_usage_deltas_are_priced_once_across_restarts_and_partial_lines() {
        use std::io::Write;
        let dir = tempfile::tempdir().expect("temp dir");
        let db_path = dir.path().join("state.db");
        let path = dir.path().join("usage.jsonl");
        let event = serde_json::json!({"event_id":"turn-1", "timestamp":"2026-10-27T00:00:00Z", "model":"gpt-6.1-sol", "input_tokens":1000000, "output_tokens":1000000, "cache_read_tokens":1000000});
        std::fs::write(&path, format!("{event}\r\n{{")).expect("write");
        {
            let mut db = Database::open(&db_path).expect("db");
            heartbeat(&mut db, AgentId::Codex, "example").expect("presence");
            ingest_usage_file(&mut db, AgentId::Codex, "example", &path).expect("import");
        }
        let mut db = Database::open(&db_path).expect("reopen");
        ingest_usage_file(&mut db, AgentId::Codex, "example", &path).expect("partial waits");
        let mut next = event.clone();
        next["event_id"] = serde_json::json!("turn-2");
        let partial_rest = next.to_string();
        std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("open")
            .write_all(format!("{}\n", &partial_rest[1..]).as_bytes())
            .expect("complete line");
        ingest_usage_file(&mut db, AgentId::Codex, "example", &path).expect("second invocation");
        // Replay after truncation with a previously seen invocation ID.
        std::fs::write(&path, format!("{event}\n")).expect("rotate");
        ingest_usage_file(&mut db, AgentId::Codex, "example", &path).expect("replay");
        let session = heartbeat(&mut db, AgentId::Codex, "example").expect("presence");
        assert!(session.usage_available);
        assert_eq!(session.model, "gpt-6.1-sol");
        assert_eq!(session.input_tokens, 2000000);
        assert_eq!(session.cache_read_tokens, 2000000);
        assert_eq!(session.output_tokens, 2000000);
        assert!((session.total_cost_usd - 26.25).abs() < 0.00001);
        assert_eq!(session.cost_basis, fleethud_core::TrustLevel::Estimated);
    }

    #[test]
    fn usage_rejects_content_and_preserves_unknown_model_cost() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut db = Database::open(&dir.path().join("state.db")).expect("db");
        heartbeat(&mut db, AgentId::ClaudeCode, "example").expect("presence");
        let path = dir.path().join("usage.jsonl");
        let mut event = serde_json::json!({"event_id":"turn-1", "timestamp":"2026-10-27T00:00:00Z", "model":"unknown-model", "input_tokens":100, "output_tokens":20, "prompt":"must not be persisted"});
        std::fs::write(&path, format!("{event}\n")).expect("write");
        assert!(ingest_usage_file(&mut db, AgentId::ClaudeCode, "example", &path).is_err());
        assert!(
            !heartbeat(&mut db, AgentId::ClaudeCode, "example")
                .expect("presence")
                .usage_available
        );
        event.as_object_mut().expect("object").remove("prompt");
        std::fs::write(&path, format!("{event}\n")).expect("write valid");
        ingest_usage_file(&mut db, AgentId::ClaudeCode, "example", &path).expect("import");
        let session = heartbeat(&mut db, AgentId::ClaudeCode, "example").expect("presence");
        assert!(session.usage_available);
        assert_eq!(session.input_tokens, 100);
        assert_eq!(session.cost_basis, fleethud_core::TrustLevel::Unknown);
    }
    #[tokio::test]
    async fn automatic_daemon_collector_registers_without_manual_cli() {
        let dir = tempfile::tempdir().expect("temporary directory");
        let path = dir.path().join("state.db");
        let task = spawn_collector_for(
            path.clone(),
            AgentId::Codex,
            "automatic-thread".to_owned(),
            dir.path().join("unpaired.json"),
        );
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let found = if path.exists() {
                Database::open(&path)
                    .ok()
                    .and_then(|db| db.list_sessions_paginated(None, None, 0, 10).ok())
                    .is_some_and(|(items, _)| {
                        items
                            .iter()
                            .any(|s| s.id == "codex:automatic-thread" && s.state == "connected")
                    })
            } else {
                false
            };
            if found {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "automatic collector did not register"
            );
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        task.abort();
    }
}
