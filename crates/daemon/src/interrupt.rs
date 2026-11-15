//! Bounded, state-gated terminal interruption shared by local and encrypted Remote.

use crate::api::{AppState, validate_auth};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use gnomcode_core::presence::{SessionPresence, classify_presence};
use gnomcode_store::Database;
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

struct Receipt {
    request_id: String,
    session_id: String,
    at: Instant,
    response: Value,
}

static RECEIPTS: Mutex<VecDeque<Receipt>> = Mutex::const_new(VecDeque::new());

fn error(message: &str) -> Value {
    json!({"type":"error", "message":message})
}

/// Handles the bounded Remote interrupt request without resuming or killing a host.
pub(crate) async fn command(db: &Arc<Mutex<Database>>, request: Value) -> Value {
    let Some(id) = request
        .get("session_id")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty() && id.len() <= 128 && !id.chars().any(char::is_control))
    else {
        return error("Missing or invalid session");
    };
    let Some(request_id) = request
        .get("request_id")
        .and_then(Value::as_str)
        .filter(|id| gnomcode_core::approval::valid_id(id))
    else {
        return error("Missing or invalid request_id");
    };
    let _control = crate::CONVERSATION_CONTROL.lock().await;
    let mut receipts = RECEIPTS.lock().await;
    receipts.retain(|entry| entry.at.elapsed() < Duration::from_secs(600));
    if let Some(entry) = receipts.iter().find(|entry| entry.request_id == request_id) {
        return if entry.session_id == id {
            entry.response.clone()
        } else {
            error("request_id already belongs to another conversation")
        };
    }
    if receipts.len() >= 256 {
        return error("Too many recent interrupt requests; try later");
    }
    let requested_id = id.to_owned();
    let database = Arc::clone(db);
    let target = tokio::task::spawn_blocking(move || {
        let guard = database.blocking_lock();
        let sessions = guard.list_active_sessions(usize::MAX)?;
        let hosted = gnomcode_pty::list_active_sessions();
        let native =
            crate::remote::active_conversation_id(&guard, &requested_id, &sessions, &hosted)?;
        let host =
            crate::remote::conversation_host(&guard, &requested_id, &sessions, &hosted)?.cloned();
        let running = sessions
            .iter()
            .find(|session| session.id == native)
            .zip(host.as_ref())
            .is_some_and(|(session, host)| {
                let age = chrono::Utc::now()
                    .signed_duration_since(session.last_event_at)
                    .num_milliseconds();
                chrono::DateTime::parse_from_rfc3339(&host.started_at)
                    .is_ok_and(|started| session.last_event_at >= started)
                    && classify_presence(session.state.as_str(), Some(age), false)
                        == SessionPresence::Running
            });
        let waiting = crate::approvals::global().pending().iter().any(|approval| {
            approval.session_id == native
                || native
                    .split_once(':')
                    .is_some_and(|(_, id)| approval.session_id == id)
        });
        Ok::<_, gnomcode_store::Error>(host.filter(|_| running && !waiting))
    })
    .await;
    let response = match target {
        Ok(Ok(Some(record))) => match gnomcode_pty::send_input(&record, b"\x1b").await {
            Ok(()) => {
                json!({"type":"interrupt_ack", "session_id":id, "request_id":request_id, "sent":true})
            }
            Err(_) => error(
                "Interrupt delivery could not be confirmed; check the agent terminal before trying again",
            ),
        },
        Ok(Ok(None)) => {
            json!({"type":"interrupt_ack", "session_id":id, "request_id":request_id, "sent":false})
        }
        _ => error("Could not inspect this conversation"),
    };
    // Retain ambiguous writes too: retrying a timed-out Esc could affect a later composer.
    receipts.push_back(Receipt {
        request_id: request_id.to_owned(),
        session_id: id.to_owned(),
        at: Instant::now(),
        response: response.clone(),
    });
    response
}

/// Authenticated local equivalent of the encrypted Remote request.
pub(crate) async fn handler(
    headers: HeaderMap,
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(mut body): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    if !body.is_object() {
        return Err(StatusCode::BAD_REQUEST);
    }
    body["session_id"] = json!(id);
    Ok(Json(command(&state.db, body).await))
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[tokio::test]
    async fn interrupt_writes_one_escape_once_and_refuses_nonrunning_states() {
        if std::env::var_os("GNOMCODE_TEST_INTERRUPT").is_none() {
            let home = tempfile::tempdir().expect("isolated home");
            let output = std::process::Command::new(std::env::current_exe().expect("test binary"))
                .args(["--exact", "interrupt::tests::interrupt_writes_one_escape_once_and_refuses_nonrunning_states", "--nocapture"])
                .env("GNOMCODE_TEST_INTERRUPT", "1")
                .env("GNOMCODE_HOME", home.path())
                .output().expect("isolated process");
            assert!(
                output.status.success(),
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        use axum::{
            Router,
            body::Bytes,
            extract::State,
            http::{HeaderMap, StatusCode},
            routing::post,
        };
        use gnomcode_core::{AgentId, Project, Session, SessionState, Surface};
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let port = listener.local_addr().expect("address").port();
        let app = Router::new()
            .route(
                "/input",
                post(
                    async |State(tx): State<
                        tokio::sync::mpsc::UnboundedSender<(HeaderMap, Bytes)>,
                    >,
                           headers: HeaderMap,
                           body: Bytes| {
                        let status = if headers["authorization"] == "Bearer fail-token" {
                            StatusCode::GATEWAY_TIMEOUT
                        } else {
                            StatusCode::OK
                        };
                        tx.send((headers, body)).expect("observed input");
                        status
                    },
                ),
            )
            .with_state(tx);
        let server = tokio::spawn(async move { axum::serve(listener, app).await.expect("serve") });
        let repo = tempfile::tempdir().expect("workspace");
        let now = chrono::Utc::now();
        let mut record = gnomcode_pty::SessionRecord {
            session_id: "interrupt-host".into(),
            agent: "codex".into(),
            repo_path: repo.path().to_string_lossy().into_owned(),
            model: None,
            host_pid: std::process::id(),
            child_pid: std::process::id(),
            port,
            token: "interrupt-host-token".into(),
            started_at: (now - chrono::Duration::seconds(1)).to_rfc3339(),
            native_id: None,
        };
        gnomcode_pty::save_session_record(&record).expect("host record");
        let project = Project::from_origin(None, &record.repo_path, None);
        let mut session = Session {
            id: "codex:interrupt-native".into(),
            agent: AgentId::Codex,
            surface: Surface::Cli,
            project_id: project.id.clone(),
            parent_session_id: None,
            model: None,
            title: None,
            state: SessionState::Thinking,
            pid: Some(std::process::id()),
            started_at: now,
            last_event_at: now,
        };
        let checkpoint = gnomcode_store::SourceCheckpoint {
            file_path: "fixture".into(),
            last_read_offset: 0,
            file_size: 0,
            fingerprint: "fixture".into(),
            last_ingested_at: now,
        };
        let db = Arc::new(Mutex::new(
            gnomcode_store::Database::open_in_memory().expect("database"),
        ));
        db.lock()
            .await
            .commit_batch(
                std::slice::from_ref(&project),
                std::slice::from_ref(&session),
                &[],
                &[],
                &[],
                &checkpoint,
            )
            .expect("session");
        let request =
            json!({"type":"interrupt", "session_id":record.session_id, "request_id":"tap-0001"});
        let response = crate::remote::dispatch(&db, request.clone()).await;
        assert_eq!(
            response,
            json!({"type":"interrupt_ack", "session_id":record.session_id, "request_id":"tap-0001", "sent":true})
        );
        let (headers, bytes) = rx.try_recv().expect("input received");
        assert_eq!(headers["authorization"], "Bearer interrupt-host-token");
        assert_eq!(bytes.as_ref(), b"\x1b");
        assert_eq!(
            crate::remote::dispatch(&db, request.clone()).await,
            response
        );
        assert!(
            rx.try_recv().is_err(),
            "a repeated request must not send another Esc"
        );
        let collision = crate::remote::dispatch(
            &db,
            json!({"type":"interrupt", "session_id":"another-session", "request_id":"tap-0001"}),
        )
        .await;
        assert_eq!(
            collision["type"], "error",
            "request IDs cannot be reused for another conversation"
        );
        for (index, state) in [
            SessionState::Idle,
            SessionState::AwaitingApproval,
            SessionState::Ended,
            SessionState::Error,
        ]
        .into_iter()
        .enumerate()
        {
            session.state = state;
            db.lock()
                .await
                .commit_batch(
                    &[],
                    std::slice::from_ref(&session),
                    &[],
                    &[],
                    &[],
                    &checkpoint,
                )
                .expect("state");
            let result = crate::remote::dispatch(&db, json!({"type":"interrupt", "session_id":record.session_id, "request_id":format!("idle-{index:04}")})).await;
            assert_eq!(result["sent"], false, "{state:?}: {result}");
            assert!(
                rx.try_recv().is_err(),
                "nonrunning states must not receive Esc"
            );
        }
        session.state = SessionState::Thinking;
        session.last_event_at = now - chrono::Duration::hours(3);
        db.lock()
            .await
            .commit_batch(
                &[],
                std::slice::from_ref(&session),
                &[],
                &[],
                &[],
                &checkpoint,
            )
            .expect("old state");
        let result = crate::remote::dispatch(
            &db,
            json!({"type":"interrupt", "session_id":record.session_id, "request_id":"stale-0001"}),
        )
        .await;
        assert_eq!(result["sent"], false);
        assert!(
            rx.try_recv().is_err(),
            "pre-launch observations cannot trigger Esc"
        );
        session.state = SessionState::Tool;
        session.last_event_at = chrono::Utc::now();
        db.lock()
            .await
            .commit_batch(&[], &[session], &[], &[], &[], &checkpoint)
            .expect("running tool");
        record.token = "fail-token".into();
        gnomcode_pty::save_session_record(&record).expect("failing host");
        let failed_request =
            json!({"type":"interrupt", "session_id":record.session_id, "request_id":"failed-0001"});
        let failed = crate::remote::dispatch(&db, failed_request.clone()).await;
        assert_eq!(failed["type"], "error");
        assert_eq!(rx.try_recv().expect("attempted write").1.as_ref(), b"\x1b");
        assert_eq!(crate::remote::dispatch(&db, failed_request).await, failed);
        assert!(
            rx.try_recv().is_err(),
            "ambiguous writes must not be retried"
        );
        gnomcode_pty::remove_session_record(&record.session_id);
        server.abort();
    }

    #[tokio::test]
    async fn interrupt_rejects_missing_session_before_host_discovery() {
        let db = Arc::new(Mutex::new(
            gnomcode_store::Database::open_in_memory().expect("database"),
        ));
        let response =
            crate::remote::dispatch(&db, json!({"type":"interrupt", "request_id":"tap-1"})).await;
        assert_eq!(response["message"], "Missing or invalid session");
    }

    #[tokio::test]
    async fn interrupt_requires_bounded_request_id() {
        let db = Arc::new(Mutex::new(
            gnomcode_store::Database::open_in_memory().expect("database"),
        ));
        for request_id in [
            json!(null),
            json!(""),
            json!("../request"),
            json!("x".repeat(65)),
        ] {
            let response = crate::remote::dispatch(
                &db,
                json!({"type":"interrupt", "session_id":"unused", "request_id":request_id}),
            )
            .await;
            assert_eq!(response["message"], "Missing or invalid request_id");
        }
    }

    #[tokio::test]
    async fn local_interrupt_preserves_auth_host_and_origin_checks() {
        use axum::{
            body::Body,
            http::{Request, StatusCode},
        };
        use tower::ServiceExt;
        let state = crate::api::AppState {
            db: Arc::new(Mutex::new(
                gnomcode_store::Database::open_in_memory().expect("database"),
            )),
            bearer_token: "local-token".into(),
            orchestrator: None,
            remote: Arc::new(crate::remote::RemoteManager::new()),
        };
        let app = crate::api::build_router(state);
        for (host, origin, token, expected) in [
            (
                "127.0.0.1",
                "http://127.0.0.1",
                "wrong",
                StatusCode::UNAUTHORIZED,
            ),
            (
                "evil.test",
                "http://127.0.0.1",
                "local-token",
                StatusCode::FORBIDDEN,
            ),
            (
                "127.0.0.1",
                "https://evil.test",
                "local-token",
                StatusCode::FORBIDDEN,
            ),
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/v1/sessions/unused/interrupt")
                        .header("host", host)
                        .header("origin", origin)
                        .header("authorization", format!("Bearer {token}"))
                        .header("content-type", "application/json")
                        .body(Body::from(r#"{"request_id":"local-0001"}"#))
                        .expect("request"),
                )
                .await
                .expect("response");
            assert_eq!(response.status(), expected);
        }
    }
}
