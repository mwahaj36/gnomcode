use gnomcode_store::Database;
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::sync::Mutex;

// All new SQLite work runs off the async executor.
async fn store<T: Send + 'static>(
    db: &Arc<Mutex<Database>>,
    action: impl FnOnce(&Database) -> Result<T, gnomcode_store::Error> + Send + 'static,
) -> Result<T, String> {
    let guard = db.clone().lock_owned().await;
    tokio::task::spawn_blocking(move || action(&guard))
        .await
        .map_err(|_| "Local storage worker stopped".to_owned())?
        .map_err(|_| "Could not access local automation storage".to_owned())
}

pub(crate) async fn command(db: &Arc<Mutex<Database>>, request: Value) -> Value {
    match request_inner(db, request).await {
        Ok(reply) => reply,
        Err(message) => json!({"type":"error","message":message}),
    }
}

async fn request_inner(db: &Arc<Mutex<Database>>, request: Value) -> Result<Value, String> {
    let kind = request["type"].as_str().unwrap_or("");
    if kind == "notices" {
        return Ok(
            json!({"type":"notices","notices":store(db,Database::automation_notices).await?}),
        );
    }
    if kind == "routing" {
        if let Some(value) = request["policy"].as_str() {
            let policy = gnomcode_core::RoutingPolicy::parse_policy(value)
                .ok_or("Invalid routing policy")?;
            let _control = crate::CONVERSATION_CONTROL.lock().await;
            store(db, move |db| db.save_routing_policy(policy)).await?;
        }
        return Ok(
            json!({"type":"routing","policy":store(db,Database::get_routing_policy).await?.as_str()}),
        );
    }
    let conversation = request["session_id"]
        .as_str()
        .filter(|id| !id.is_empty() && id.len() <= 128)
        .ok_or("Choose a conversation")?
        .to_owned();
    if kind == "schedules" {
        let (messages, armed) = store(db, {
            let id = conversation.clone();
            move |db| {
                let root = db.conversation_id(&id)?;
                let mut armed = false;
                for entry in db.list_resume_queue()? {
                    if db.conversation_id(&entry.session_id)? == root {
                        armed = true;
                        break;
                    }
                }
                Ok((db.scheduled_messages(&id)?, armed))
            }
        })
        .await?;
        return Ok(
            json!({"type":"schedules","session_id":conversation,"messages":messages,"reset_armed":armed}),
        );
    }
    if kind == "schedule" || (kind == "resume_on_reset" && request["enabled"] != false) {
        let id = conversation.clone();
        if !store(db, move |db| Ok(db.conversation_launch(&id)?.is_some())).await? {
            return Err("Imported provider sessions are view-only. Start a Gnomcode conversation to use automation.".into());
        }
    }
    let _control = crate::CONVERSATION_CONTROL.lock().await;
    if kind == "resume_on_reset" {
        let id = conversation.clone();
        if request["enabled"].as_bool() == Some(false) {
            store(db, move |db| db.disarm_resume_queue(&id)).await?;
        } else {
            let armed = store(db, move |db| {
                let native = db
                    .conversation_target(&id)?
                    .and_then(|(_, native, _)| native)
                    .unwrap_or(id.clone());
                let target = db.get_session_focus_target(&native)?.ok_or_else(|| {
                    gnomcode_store::Error::Migration("Missing conversation".into())
                })?;
                let agent = match target.agent.as_str() {
                    "codex" => gnomcode_core::AgentId::Codex,
                    "claude-code" => gnomcode_core::AgentId::ClaudeCode,
                    "gemini-cli" => gnomcode_core::AgentId::GeminiCli,
                    _ => return Err(gnomcode_store::Error::Migration("Unsupported agent".into())),
                };
                let quotas = db.get_latest_quota_snapshots()?;
                let now = chrono::Utc::now();
                let limited: Vec<_> = quotas
                    .iter()
                    .filter(|q| {
                        provider(&q.provider) == provider(&target.agent)
                            && q.used_pct >= 100.0
                            && q.used_pct.is_finite()
                            && q.basis != gnomcode_core::TrustLevel::Unknown
                            && q.observed_at <= now
                            && now - q.observed_at <= chrono::Duration::minutes(5)
                    })
                    .collect();
                if limited.is_empty() {
                    return Ok(false);
                }
                let eligible = limited
                    .iter()
                    .filter_map(|q| q.resets_at)
                    .max()
                    .unwrap_or(now);
                db.arm_resume_queue(&gnomcode_core::rules::ResumeQueueEntry {
                    session_id: id,
                    agent,
                    cwd: target.repo_root.unwrap_or_default(),
                    resume_prompt: "continue".into(),
                    max_runs: 1,
                    runs_completed: 0,
                    eligible_after: eligible,
                })?;
                Ok(true)
            })
            .await?;
            if !armed {
                return Err("No fresh known exhausted quota is available. Refresh Limits after the agent reports its limit, then retry.".into());
            }
        }
        return Ok(json!({"type":"resume_on_reset_ack","session_id":conversation}));
    }
    let id = request["id"]
        .as_str()
        .filter(|id| {
            !id.is_empty()
                && id.len() <= 128
                && id
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"-_".contains(&c))
        })
        .ok_or("Invalid schedule ID")?
        .to_owned();
    if kind == "cancel_schedule" {
        let cancelled = store(db, {
            let conversation = conversation.clone();
            let id = id.clone();
            move |db| db.cancel_message(&conversation, &id)
        })
        .await?;
        if !cancelled {
            return Err("Message was already claimed or is no longer pending".into());
        }
        notice(
            db,
            &format!("cancel:{id}"),
            &conversation,
            "Scheduled message cancelled.",
        )
        .await;
        return Ok(json!({"type":"cancel_schedule_ack","session_id":conversation}));
    }
    if kind != "schedule" {
        return Err("Unsupported automation request".into());
    }
    let text = request["text"]
        .as_str()
        .ok_or("Missing message")?
        .trim()
        .to_owned();
    let due = request["due_at"].as_i64().ok_or("Choose a delivery time")?;
    let now = chrono::Utc::now().timestamp();
    if text.is_empty()
        || text.encode_utf16().count() > 4000
        || text.starts_with('/')
        || text
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\t'))
        || due <= now
        || due > now + 366 * 86400
    {
        return Err("Use a message up to 4000 bytes and a future time within one year".into());
    }
    let conversation_copy = conversation.clone();
    let new = store(db, move |db| {
        let known = db.conversation_launch(&conversation_copy)?.is_some()
            || db.get_session_focus_target(&conversation_copy)?.is_some();
        if !known {
            return Err(gnomcode_store::Error::Migration(
                "Missing conversation".into(),
            ));
        }
        db.schedule_message(&id, &conversation_copy, &text, due)
    })
    .await?;
    if new {
        notice(
            db,
            &format!("saved:{}", request["id"].as_str().unwrap_or_default()),
            &conversation,
            "Scheduled message saved on this laptop.",
        )
        .await;
    }
    Ok(json!({"type":"schedule_ack","session_id":conversation,"id":request["id"]}))
}

pub(crate) async fn notice(
    db: &Arc<Mutex<Database>>,
    key: &str,
    conversation: &str,
    message: &str,
) {
    let (key, conversation, message) =
        (key.to_owned(), conversation.to_owned(), message.to_owned());
    if store(db, move |db| {
        db.automation_notice(&key, &conversation, &message)
    })
    .await
    .is_err()
    {
        tracing::warn!("automation notification could not be recorded");
    }
}

fn provider(agent: &str) -> &str {
    match agent {
        "anthropic" | "claude" | "claude-code" => "claude-code",
        "openai" | "codex" => "codex",
        "google" | "gemini" | "gemini-cli" => "gemini-cli",
        value => value,
    }
}

#[derive(Default)]
pub(crate) struct Observations {
    sessions: std::collections::HashMap<String, gnomcode_core::SessionState>,
    quotas: std::collections::HashMap<String, (f64, chrono::DateTime<chrono::Utc>)>,
}

pub(crate) async fn tick(db: &Arc<Mutex<Database>>, previous: &mut Observations) {
    let _ = store(db, Database::expire_automation_records).await;
    if let Ok(sessions) = store(db, |db| db.list_active_sessions(1000)).await {
        let mut current = std::collections::HashMap::new();
        for session in sessions
            .into_iter()
            .filter(|session| session.parent_session_id.is_none())
        {
            if session.state == gnomcode_core::SessionState::Idle
                && previous.sessions.get(&session.id).is_some_and(|state| {
                    matches!(
                        state,
                        gnomcode_core::SessionState::Thinking | gnomcode_core::SessionState::Tool
                    )
                })
            {
                notice(
                    db,
                    &format!(
                        "completed:{}:{}",
                        session.id,
                        session.last_event_at.to_rfc3339()
                    ),
                    &session.id,
                    "Agent finished its response. The conversation is ready.",
                )
                .await;
            }
            if session.state == gnomcode_core::SessionState::AwaitingApproval
                && previous.sessions.get(&session.id) != Some(&session.state)
                && chrono::Utc::now() - session.last_event_at <= chrono::Duration::minutes(5)
            {
                notice(
                    db,
                    &format!(
                        "approval:{}:{}",
                        session.id,
                        session.last_event_at.to_rfc3339()
                    ),
                    &session.id,
                    "Agent needs approval. Review it on your laptop.",
                )
                .await;
            }
            current.insert(session.id, session.state);
        }
        previous.sessions = current;
    }
    if let Ok(quotas) = store(db, Database::get_latest_quota_snapshots).await {
        let now = chrono::Utc::now();
        let mut current = std::collections::HashMap::new();
        for quota in quotas.into_iter().filter(|q| {
            q.basis != gnomcode_core::TrustLevel::Unknown
                && q.observed_at <= now
                && now - q.observed_at <= chrono::Duration::minutes(5)
                && (0.0..=100.0).contains(&q.used_pct)
        }) {
            let key = format!("{}:{}:{:?}", quota.provider, quota.limit_id, quota.window);
            if previous.quotas.get(&key).is_some_and(|(used, observed)| {
                *used >= 100.0 && quota.used_pct < 100.0 && quota.observed_at > *observed
            }) {
                notice(
                    db,
                    &format!("quota-reset:{key}:{}", quota.observed_at.to_rfc3339()),
                    "",
                    &format!(
                        "{} quota has reset. Check Limits or reconnect the conversation.",
                        provider(&quota.provider)
                    ),
                )
                .await;
            }
            let threshold = if quota.used_pct >= 100.0 {
                100
            } else if quota.used_pct >= 95.0 {
                95
            } else if quota.used_pct >= 80.0 {
                80
            } else {
                0
            };
            if threshold > 0
                && previous
                    .quotas
                    .get(&key)
                    .is_none_or(|(used, _)| *used < f64::from(threshold))
            {
                let cycle = quota
                    .resets_at
                    .map(|time| time.to_rfc3339())
                    .unwrap_or_else(|| now.date_naive().to_string());
                notice(db,&format!("quota-warning:{key}:{cycle}:{threshold}"),"",&format!("{} quota is running low or exhausted. Open Limits for source and reset details.",provider(&quota.provider))).await;
            }
            current.insert(key, (quota.used_pct, quota.observed_at));
        }
        previous.quotas = current;
    }
    deliver_due(db).await;
    route_limited(db).await;
    resume_due(db).await;
    if let Ok(messages) = store(db, Database::claim_automation_notices).await {
        for message in messages {
            tokio::task::spawn_blocking(move || {
                let _ = gnomcode_platform::send_desktop_notification("Gnomcode", &message);
            });
        }
    }
}

async fn deliver_due(db: &Arc<Mutex<Database>>) {
    let Ok(messages) = store(db, move |db| {
        db.due_messages(chrono::Utc::now().timestamp())
    })
    .await
    else {
        return;
    };
    for message in messages {
        let Ok(_control) = crate::CONVERSATION_CONTROL.try_lock() else {
            break;
        };
        let id = message.conversation_id.clone();
        let ready = store(db, move |db| {
            let sessions = db.list_active_sessions(1000)?;
            let hosted = gnomcode_pty::list_active_sessions();
            let native = crate::remote::active_conversation_id(db, &id, &sessions, &hosted)?;
            if let Some(attempt) = db.last_scheduled_attempt(&id)? {
                let observed = sessions.iter().find(|s| s.id == native);
                let restarted = crate::remote::conversation_host(db, &id, &sessions, &hosted)?
                    .is_some_and(|host| {
                        chrono::DateTime::parse_from_rfc3339(&host.started_at)
                            .is_ok_and(|time| time.timestamp_millis() > attempt)
                    });
                if !restarted
                    && observed.is_none_or(|s| s.last_event_at.timestamp_millis() <= attempt)
                {
                    return Ok(false);
                }
            }
            Ok(!sessions.iter().any(|s| {
                s.id == native
                    && matches!(
                        s.state,
                        gnomcode_core::SessionState::Thinking
                            | gnomcode_core::SessionState::Tool
                            | gnomcode_core::SessionState::AwaitingApproval
                    )
                    || (s.id == native
                        && s.state == gnomcode_core::SessionState::Error
                        && !db
                            .quota_reset_ready(&s.id, s.last_event_at)
                            .unwrap_or(false))
            }))
        })
        .await;
        if !matches!(ready, Ok(true)) {
            continue;
        }
        let id = message.id.clone();
        if !matches!(store(db, move |db| db.claim_message(&id)).await, Ok(true)) {
            continue;
        }
        let result =
            crate::deliver_conversation_prompt_locked(db, &message.conversation_id, &message.text)
                .await;
        let status = match &result {
            Ok(()) => "delivered",
            Err(crate::HandoffError::MissingSession | crate::HandoffError::UnsupportedTarget) => {
                "failed"
            }
            Err(_) => "uncertain",
        };
        let id = message.id.clone();
        if store(db, move |db| db.finish_message(&id, status))
            .await
            .is_err()
        {
            tracing::warn!("scheduled delivery outcome could not be saved");
        }
        notice(db,&format!("outcome:{}",message.id),&message.conversation_id,if result.is_ok(){"Scheduled message accepted by the agent terminal."}else{"Scheduled delivery could not be confirmed. Check the agent terminal before retrying; it will not be replayed."}).await;
    }
}

async fn resume_due(db: &Arc<Mutex<Database>>) {
    let Ok((entries, quotas)) = store(db, |db| {
        Ok((db.list_resume_queue()?, db.get_latest_quota_snapshots()?))
    })
    .await
    else {
        return;
    };
    let now = chrono::Utc::now();
    for entry in entries {
        let matching: Vec<_> = quotas
            .iter()
            .filter(|q| provider(&q.provider) == provider(entry.agent.as_str()))
            .collect();
        if matching.is_empty()
            || !matching.iter().all(|q| {
                gnomcode_core::rules::is_resume_eligible(&entry, now, Some(q))
                    && now - q.observed_at <= chrono::Duration::minutes(5)
            })
        {
            continue;
        }
        let Ok(_control) = crate::CONVERSATION_CONTROL.try_lock() else {
            continue;
        };
        let expected = entry.clone();
        if !matches!(
            store(db, move |db| Ok(db
                .list_resume_queue()?
                .contains(&expected)))
            .await,
            Ok(true)
        ) {
            continue;
        }
        let state_id = entry.session_id.clone();
        let busy = store(db, move |db| {
            let sessions = db.list_active_sessions(1000)?;
            let native = crate::remote::active_conversation_id(
                db,
                &state_id,
                &sessions,
                &gnomcode_pty::list_active_sessions(),
            )?;
            Ok(sessions.iter().any(|s| {
                s.id == native
                    && matches!(
                        s.state,
                        gnomcode_core::SessionState::Thinking
                            | gnomcode_core::SessionState::Tool
                            | gnomcode_core::SessionState::AwaitingApproval
                    )
            }))
        })
        .await;
        if !matches!(busy, Ok(false)) {
            continue;
        }
        // Claim by disarming first: a process crash must never repeat a continuation.
        let id = entry.session_id.clone();
        if store(db, move |db| db.disarm_resume_queue(&id))
            .await
            .is_err()
        {
            continue;
        }
        let result = crate::ensure_conversation_running(db, &entry.session_id).await;
        if result.is_ok() {
            let id = entry.session_id.clone();
            let _ = store(db, move |db| {
                let sessions = db.list_active_sessions(1000)?;
                let native = crate::remote::active_conversation_id(
                    db,
                    &id,
                    &sessions,
                    &gnomcode_pty::list_active_sessions(),
                )?;
                if let Some(s) = sessions
                    .iter()
                    .find(|s| s.id == native && s.state == gnomcode_core::SessionState::Error)
                {
                    db.automation_notice(
                        &format!("reset-ready:{}:{}", s.id, s.last_event_at.to_rfc3339()),
                        &id,
                        "Quota reset confirmed for the observed error.",
                    )?;
                }
                Ok(())
            })
            .await;
        }
        notice(db,&format!("reset:{}:{}",entry.session_id,entry.eligible_after.timestamp()),&entry.session_id,if result.is_ok(){"Quota reset confirmed. Conversation reconnected and ready for your next message."}else{"Quota reset confirmed, but reconnection failed. Check the agent login on your laptop."}).await;
    }
}

fn automatic_switch_allowed(
    db: &Database,
    id: &str,
    expected_selection: Option<&str>,
    source: &str,
    target: &str,
) -> Result<bool, gnomcode_store::Error> {
    let sessions = db.list_active_sessions(1000)?;
    let hosted = gnomcode_pty::list_active_sessions();
    if crate::remote::active_conversation_id(db, id, &sessions, &hosted)? != id {
        return Ok(false);
    }
    if db
        .conversation_launch(id)?
        .and_then(|launch| launch.model)
        .as_deref()
        != expected_selection
    {
        return Ok(false);
    }
    Ok(db
        .get_routing_policy()?
        .allows_automatic(Some(provider(source)), Some(provider(target))))
}

async fn route_limited(db: &Arc<Mutex<Database>>) {
    let Ok((events, policy, mut quotas)) = store(db, |db| {
        Ok((
            db.automation_events()?,
            db.get_routing_policy()?,
            db.get_latest_quota_snapshots()?,
        ))
    })
    .await
    else {
        return;
    };
    for q in &mut quotas {
        let old = q.provider.clone();
        q.provider = provider(&old).to_owned();
        if q.limit_id == old {
            q.limit_id = q.provider.clone();
        }
    }
    for (event, id, kind, timestamp) in events {
        if kind == "approval_wait" {
            continue;
        }
        let key = format!("event:{event}");
        notice(
            db,
            &key,
            &id,
            match kind.as_str() {
                "rate_limited" => {
                    "Agent reached a rate limit. Choose another model or reconnect after reset."
                }
                "approval_wait" => "Agent needs approval. Review it on your laptop.",
                _ => "Agent reported an error. Check the conversation on your laptop.",
            },
        )
        .await;
        if kind != "rate_limited" || policy == gnomcode_core::RoutingPolicy::AskFirst {
            continue;
        }
        let key = format!("route:{event}");
        if !matches!(
            store(db, {
                let key = key.clone();
                move |db| db.has_automation_notice(&key)
            })
            .await,
            Ok(false)
        ) {
            continue;
        }
        let source_id = id.clone();
        let Ok(Some(source)) = store(db, move |db| {
            let sessions = db.list_active_sessions(1000)?;
            let hosted = gnomcode_pty::list_active_sessions();
            let Some(record) =
                crate::remote::conversation_host(db, &source_id, &sessions, &hosted)?
            else {
                return Ok(None);
            };
            if crate::remote::active_conversation_id(db, &source_id, &sessions, &hosted)?
                != source_id
            {
                return Ok(None);
            }
            let Some(session) = sessions.iter().find(|s| s.id == source_id) else {
                return Ok(None);
            };
            if !matches!(
                session.state,
                gnomcode_core::SessionState::Error | gnomcode_core::SessionState::Idle
            ) || chrono::DateTime::parse_from_rfc3339(&timestamp).is_ok_and(|time| {
                time < session.started_at
                    || session.last_event_at.signed_duration_since(time)
                        > chrono::Duration::seconds(10)
            }) {
                return Ok(None);
            }
            Ok(Some((
                record.agent.clone(),
                session.model.clone().unwrap_or_default(),
                db.conversation_launch(&source_id)?
                    .and_then(|launch| launch.model),
            )))
        })
        .await
        else {
            continue;
        };
        let catalogs = crate::model_catalog::discover().await;
        let choices: Vec<_> = catalogs
            .iter()
            .filter(|p| matches!(p.status.as_str(), "authenticated" | "configured"))
            .flat_map(|p| p.models.iter())
            .filter(|m| matches!(provider(&m.agent), "codex" | "claude-code" | "gemini-cli"))
            .collect();
        let candidates: Vec<_> = choices
            .iter()
            .map(|c| (provider(&c.agent), c.model.as_str()))
            .collect();
        let selected = gnomcode_core::routing::select_failover(
            (provider(&source.0), &source.1),
            policy,
            &candidates,
            &quotas,
            chrono::Utc::now(),
        );
        let message = if selected.is_some() {
            "Automatic model switch requested. Activation is pending agent confirmation."
        } else {
            "No eligible model with fresh known quota. Reconnect after quota reset or select a model manually."
        };
        if !matches!(
            store(db, {
                let key = key.clone();
                let id = id.clone();
                move |db| db.automation_notice(&key, &id, message)
            })
            .await,
            Ok(true)
        ) {
            continue;
        }
        if let Some(index) = selected {
            let choice = choices[index];
            let _control = crate::CONVERSATION_CONTROL.lock().await;
            let allowed = store(db, {
                let source_selection = source.2.clone();
                let source = source.0.clone();
                let target = choice.agent.clone();
                let id = id.clone();
                move |db| {
                    automatic_switch_allowed(db, &id, source_selection.as_deref(), &source, &target)
                }
            })
            .await;
            if !matches!(allowed, Ok(true)) {
                continue;
            }
            let result = async {
                let prepared =
                    crate::prepare_model_change(db, &id, &choice.model, Some(&choice.agent))
                        .await?;
                let record = crate::execute_switch(prepared).await?;
                crate::remote::record_model_change(&*db.lock().await, &id, &record)?;
                Ok::<_, crate::HandoffError>(record)
            }
            .await;
            notice(db,&format!("route-outcome:{event}"),&id,if result.is_ok(){"Automatic switch accepted by the terminal. Send your next message to continue; the previous task was not replayed."}else{"Automatic switch failed. Check the target agent installation and login on your laptop."}).await;
        } else {
            let result = command(
                db,
                json!({"type":"resume_on_reset","session_id":id,"enabled":true}),
            )
            .await;
            if result["type"] == "error" {
                notice(db,&format!("route-reset:{event}"),&id,"Reset reconnection needs fresh known exhausted quota. Check Limits and arm reconnection when the provider reports it.").await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn automation_api_requires_loopback_authentication_and_rejects_cross_origin() {
        use axum::{body::Body, http::Request};
        use tower::ServiceExt;
        let app = crate::api::build_router(crate::api::AppState {
            db: Arc::new(Mutex::new(Database::open_in_memory().expect("db"))),
            bearer_token: "fixture".into(),
            orchestrator: None,
            remote: Arc::new(crate::remote::RemoteManager::new()),
        });
        for (token, origin, expected) in [
            (None, None, 401),
            (
                Some("Bearer fixture"),
                Some("https://untrusted.example"),
                403,
            ),
            (Some("Bearer fixture"), None, 200),
        ] {
            let mut request = Request::builder()
                .method("POST")
                .uri("/api/v1/automation")
                .header("Host", "127.0.0.1")
                .header("Content-Type", "application/json");
            if let Some(token) = token {
                request = request.header("Authorization", token);
            }
            if let Some(origin) = origin {
                request = request.header("Origin", origin);
            }
            let response = app
                .clone()
                .oneshot(
                    request
                        .body(Body::from(r#"{"type":"notices"}"#))
                        .expect("request"),
                )
                .await
                .expect("response");
            assert_eq!(response.status().as_u16(), expected);
        }
    }
    #[tokio::test]
    async fn scheduler_waits_for_idle_delivers_once_and_reconnects_after_reset() {
        const CHILD: &str = "GNOMCODE_SCHEDULER_TEST";
        if std::env::var_os(CHILD).is_none() {
            let root = tempfile::tempdir().expect("root");
            let status=std::process::Command::new(std::env::current_exe().expect("test binary"))
                .args(["--exact","automation::tests::scheduler_waits_for_idle_delivers_once_and_reconnects_after_reset","--nocapture"])
                .env(CHILD,"1").env("GNOMCODE_HOME",root.path()).status().expect("isolated test");
            assert!(status.success());
            return;
        }
        use gnomcode_core::{
            AgentId, Project, QuotaSnapshot, QuotaWindow, Session, SessionState, Surface,
            TrustLevel,
        };
        let inputs = Arc::new(Mutex::new(Vec::<Vec<u8>>::new()));
        let captured = inputs.clone();
        let app = axum::Router::new().route(
            "/input",
            axum::routing::post(
                move |headers: axum::http::HeaderMap, body: axum::body::Bytes| {
                    let captured = captured.clone();
                    async move {
                        assert_eq!(headers["authorization"], "Bearer fixture");
                        captured.lock().await.push(body.to_vec());
                        axum::http::StatusCode::OK
                    }
                },
            ),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let port = listener.local_addr().expect("address").port();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("server");
        });
        let repo = tempfile::tempdir().expect("repo");
        let now = chrono::Utc::now();
        let project = Project::from_origin(None, &repo.path().to_string_lossy(), None);
        let mut session = Session {
            id: "codex:fixture".into(),
            agent: AgentId::Codex,
            surface: Surface::Cli,
            project_id: project.id.clone(),
            parent_session_id: None,
            model: Some("gpt-fixture".into()),
            title: None,
            state: SessionState::Thinking,
            pid: Some(std::process::id()),
            started_at: now,
            last_event_at: now,
        };
        gnomcode_pty::save_session_record(&gnomcode_pty::SessionRecord {
            session_id: "test-host".into(),
            agent: "codex".into(),
            repo_path: repo.path().to_string_lossy().into_owned(),
            model: None,
            host_pid: std::process::id(),
            child_pid: std::process::id(),
            port,
            token: "fixture".into(),
            started_at: now.to_rfc3339(),
        })
        .expect("host");
        let checkpoint = gnomcode_store::SourceCheckpoint {
            file_path: "fixture".into(),
            last_read_offset: 0,
            file_size: 0,
            fingerprint: "fixture".into(),
            last_ingested_at: now,
        };
        let mut database = Database::open_in_memory().expect("db");
        database
            .commit_batch(&[project], &[session.clone()], &[], &[], &[], &checkpoint)
            .expect("session");
        database
            .bind_conversation_host("test-host", &session.id)
            .expect("bind");
        database
            .save_conversation_launch(
                "test-host",
                "codex",
                &repo.path().to_string_lossy(),
                Some("catalog-alias"),
            )
            .expect("saved selection");
        database
            .save_routing_policy(gnomcode_core::RoutingPolicy::Automatic)
            .expect("automatic policy");
        assert!(
            automatic_switch_allowed(
                &database,
                &session.id,
                Some("catalog-alias"),
                "codex",
                "claude-code"
            )
            .expect("unchanged alias is eligible even when native model IDs differ")
        );
        database
            .save_conversation_launch(
                "test-host",
                "codex",
                &repo.path().to_string_lossy(),
                Some("replacement"),
            )
            .expect("user changed model");
        assert!(
            !automatic_switch_allowed(
                &database,
                &session.id,
                Some("catalog-alias"),
                "codex",
                "claude-code"
            )
            .expect("changed selection invalidates the pending automatic switch")
        );
        database
            .schedule_message("one", &session.id, "first", now.timestamp())
            .expect("save");
        database
            .schedule_message("two", &session.id, "second", now.timestamp())
            .expect("save");
        let db = Arc::new(Mutex::new(database));
        deliver_due(&db).await;
        assert!(
            inputs.lock().await.is_empty(),
            "busy agents receive no input"
        );
        session.state = SessionState::Idle;
        db.lock()
            .await
            .commit_batch(&[], &[session.clone()], &[], &[], &[], &checkpoint)
            .expect("idle");
        deliver_due(&db).await;
        let first = inputs.lock().await.concat();
        assert_eq!(first, b"\x1b[200~first\x1b[201~\x1b[F\r");
        deliver_due(&db).await;
        assert_eq!(
            inputs.lock().await.concat(),
            first,
            "stale idle evidence cannot deliver a second message"
        );
        session.last_event_at = chrono::Utc::now() + chrono::Duration::seconds(1);
        db.lock()
            .await
            .commit_batch(&[], &[session.clone()], &[], &[], &[], &checkpoint)
            .expect("fresh idle");
        deliver_due(&db).await;
        assert_eq!(
            db.lock()
                .await
                .scheduled_messages(&session.id)
                .expect("list")
                .iter()
                .filter(|m| m.status == "delivered" && m.text.is_empty())
                .count(),
            2
        );
        let delivered = inputs.lock().await.concat();
        let mut quota = QuotaSnapshot {
            provider: "codex".into(),
            limit_id: "codex".into(),
            window: QuotaWindow::FiveHour,
            used_pct: 100.0,
            resets_at: Some(now - chrono::Duration::seconds(1)),
            plan: None,
            basis: TrustLevel::Exact,
            observed_at: now,
        };
        quota.used_pct = 10.0;
        db.lock()
            .await
            .commit_batch(&[], &[], &[], &[], &[quota.clone()], &checkpoint)
            .expect("available quota");
        assert_eq!(
            command(
                &db,
                json!({"type":"resume_on_reset","session_id":session.id,"enabled":true})
            )
            .await["type"],
            "error",
            "available quota is not evidence of a reset"
        );
        quota.used_pct = 100.0;
        session.state = SessionState::Error;
        db.lock()
            .await
            .commit_batch(
                &[],
                &[session.clone()],
                &[],
                &[],
                &[quota.clone()],
                &checkpoint,
            )
            .expect("limited");
        assert_eq!(
            command(
                &db,
                json!({"type":"resume_on_reset","session_id":session.id,"enabled":true})
            )
            .await["type"],
            "resume_on_reset_ack"
        );
        resume_due(&db).await;
        assert_eq!(db.lock().await.list_resume_queue().expect("queue").len(), 1);
        assert_eq!(
            command(
                &db,
                json!({"type":"resume_on_reset","session_id":"test-host","enabled":false})
            )
            .await["type"],
            "resume_on_reset_ack"
        );
        assert!(
            db.lock()
                .await
                .list_resume_queue()
                .expect("cancelled by host identity")
                .is_empty()
        );
        assert_eq!(
            command(
                &db,
                json!({"type":"resume_on_reset","session_id":session.id,"enabled":true})
            )
            .await["type"],
            "resume_on_reset_ack"
        );
        quota.used_pct = 10.0;
        quota.observed_at = chrono::Utc::now();
        db.lock()
            .await
            .commit_batch(&[], &[], &[], &[], &[quota], &checkpoint)
            .expect("reset");
        resume_due(&db).await;
        assert!(
            db.lock()
                .await
                .list_resume_queue()
                .expect("queue")
                .is_empty()
        );
        assert!(
            db.lock()
                .await
                .quota_reset_ready(&session.id, session.last_event_at)
                .expect("ready")
        );
        assert_eq!(
            inputs.lock().await.concat(),
            delivered,
            "reconnecting never submits a task"
        );
        server.abort();
    }
    #[tokio::test]
    async fn acknowledged_schedule_is_stored_and_duplicate_key_is_not_replaced() {
        let db = Arc::new(Mutex::new(Database::open_in_memory().expect("db")));
        db.lock()
            .await
            .reattach_conversation("conversation", "host", None)
            .expect("bind");
        db.lock()
            .await
            .save_conversation_launch("host", "codex", "fixture", None)
            .expect("launch");
        let request = json!({"type":"schedule","session_id":"conversation","id":"one","text":"hello","due_at":chrono::Utc::now().timestamp()+60});
        assert_eq!(command(&db, request.clone()).await["type"], "schedule_ack");
        assert_eq!(command(&db, request).await["type"], "schedule_ack");
        let guard = db.lock().await;
        let messages = guard.scheduled_messages("conversation").expect("list");
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].text, "hello");
    }
    #[tokio::test]
    async fn rejects_invalid_or_missing_conversation_before_saving() {
        let db = std::sync::Arc::new(tokio::sync::Mutex::new(
            gnomcode_store::Database::open_in_memory().expect("db"),
        ));
        let result=command(&db,serde_json::json!({"type":"schedule","session_id":"missing","id":"one","text":"hello","due_at":chrono::Utc::now().timestamp()+60})).await;
        assert_eq!(result["type"], "error");
        assert!(
            db.lock()
                .await
                .due_messages(i64::MAX)
                .expect("list")
                .is_empty()
        );
    }
}
