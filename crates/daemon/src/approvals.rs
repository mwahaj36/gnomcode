//! Tool approvals answered from the console or a paired phone (`SPEC §3.10`, ADR 0032).
//!
//! A Claude Code `PermissionRequest` hook forwards the prompt here over the loopback API.
//! The daemon holds it only while someone can answer (a paired phone is reachable or the
//! console is open). Any failure, timeout or missing audience leaves the agent's own
//! prompt in charge; nothing is ever allowed by default.

use crate::api::{AppState, validate_auth};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};
use gnomcode_core::approval::{
    Applied, Approval, ApprovalBook, CreateError, DecideError, Decider, Decision, HOLD_SECS,
    NewApproval, State as ApprovalState,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::{
    Mutex, OnceLock, PoisonError,
    atomic::{AtomicU64, Ordering},
};
use std::time::Duration;

/// Seconds after the console last asked for approvals during which it counts as watching.
const CONSOLE_WATCH_SECS: u64 = 15;

/// What a waiting hook gets back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Allowed once.
    Allow,
    /// Denied.
    Deny,
    /// No decision: the agent's own prompt decides.
    NoDecision,
}

/// Process-wide set of pending approvals.
#[derive(Default)]
pub struct Approvals {
    book: Mutex<ApprovalBook>,
    changed: tokio::sync::Notify,
    console_seen: AtomicU64,
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

/// The registry shared by the HTTP handlers and Remote.
pub fn global() -> &'static Approvals {
    static REGISTRY: OnceLock<Approvals> = OnceLock::new();
    REGISTRY.get_or_init(Approvals::default)
}

impl Approvals {
    fn book(&self) -> std::sync::MutexGuard<'_, ApprovalBook> {
        self.book.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Adds a pending request that stops waiting after `hold_secs`.
    ///
    /// # Errors
    /// [`CreateError::Rejected`] when too many are pending or the random id cannot be made.
    pub fn create(
        &self,
        agent: &str,
        session_id: &str,
        tool: &str,
        summary: &str,
        hold_secs: u64,
    ) -> Result<Approval, CreateError> {
        let mut bytes = [0_u8; 16];
        getrandom::fill(&mut bytes).map_err(|_| CreateError::Rejected)?;
        let id = crate::remote_trust::hex(&bytes);
        let created = self.book().create(
            &NewApproval {
                id: &id,
                agent,
                session_id,
                tool,
                summary,
            },
            now(),
            hold_secs,
        )?;
        self.changed.notify_waiters();
        Ok(created)
    }

    /// Records a decision for the exact request id.
    ///
    /// # Errors
    /// See [`DecideError`].
    pub fn decide(
        &self,
        id: &str,
        decision: Decision,
        by: Decider,
    ) -> Result<Applied, DecideError> {
        let result = self.book().decide(id, decision, by, now());
        if matches!(result, Ok(Applied::Recorded)) {
            tracing::info!(?decision, ?by, "approval decided");
        }
        self.changed.notify_waiters();
        result
    }

    /// Marks a pending request cancelled because the agent stopped waiting.
    pub fn cancel(&self, id: &str) {
        self.book().cancel(id, now());
        self.changed.notify_waiters();
    }

    /// Requests still waiting.
    #[must_use]
    pub fn pending(&self) -> Vec<Approval> {
        self.book().pending(now())
    }

    /// Notes that the console asked for approvals, so it counts as an audience.
    pub fn touch_console(&self) {
        self.console_seen.store(now(), Ordering::Relaxed);
    }

    /// Whether the console is open and polling.
    #[must_use]
    pub fn console_watching(&self) -> bool {
        let seen = self.console_seen.load(Ordering::Relaxed);
        seen != 0 && now().saturating_sub(seen) <= CONSOLE_WATCH_SECS
    }

    fn has_pending_for(&self, session_id: &str) -> bool {
        self.pending()
            .iter()
            .any(|item| item.session_id == session_id)
    }

    /// Waits until the request is decided, expires or is cancelled.
    pub async fn wait(&self, id: &str) -> Outcome {
        loop {
            let notified = self.changed.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            match self.book().state(id, now()) {
                Some(ApprovalState::Allowed) => return Outcome::Allow,
                Some(ApprovalState::Denied) => return Outcome::Deny,
                Some(ApprovalState::Pending) => {}
                _ => return Outcome::NoDecision,
            }
            tokio::select! {
                () = &mut notified => {}
                () = tokio::time::sleep(Duration::from_millis(250)) => {}
            }
        }
    }
}

/// Cancels a still-pending request when a waiting hook disconnects (the agent was answered
/// natively, or the session ended).
struct CancelOnDrop(String);

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        global().cancel(&self.0);
    }
}

/// Request body from the hook.
#[derive(Debug, Deserialize)]
pub struct CreateBody {
    /// Agent slug.
    pub agent: String,
    /// Agent-native session id.
    pub session_id: String,
    /// Tool name.
    pub tool: String,
    /// Summary already redacted by the hook; redacted again on entry.
    pub summary: String,
}

/// `POST /api/v1/approvals`: records a request and says whether the hook should wait.
pub(crate) async fn create_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(body): Json<CreateBody>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let registry = global();
    if !(registry.console_watching() || state.remote.reachable().await) {
        return Ok(Json(json!({"hold": false, "reason": "no_listener"})));
    }
    let Ok(approval) = registry.create(
        &body.agent,
        &body.session_id,
        &body.tool,
        &body.summary,
        HOLD_SECS,
    ) else {
        return Ok(Json(json!({"hold": false, "reason": "busy"})));
    };
    crate::automation::notice(
        &state.db,
        &format!("approval-request:{}", approval.id),
        "",
        "Approval needed. Open Gnomcode to approve or deny.",
    )
    .await;
    Ok(Json(
        json!({"hold": true, "id": approval.id, "wait_secs": HOLD_SECS}),
    ))
}

/// `GET /api/v1/approvals/{id}/wait`: holds until a decision, expiry or disconnect.
pub(crate) async fn wait_handler(
    headers: HeaderMap,
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let _cancel = CancelOnDrop(id.clone());
    let outcome = match global().wait(&id).await {
        Outcome::Allow => "allow",
        Outcome::Deny => "deny",
        Outcome::NoDecision => "none",
    };
    Ok(Json(json!({"outcome": outcome})))
}

/// `GET /api/v1/approvals`: pending requests for the console.
pub(crate) async fn list_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    global().touch_console();
    Ok(Json(json!({"items": global().pending()})))
}

/// Decision body from the console.
#[derive(Debug, Deserialize)]
pub struct DecisionBody {
    /// `allow` or `deny`.
    pub decision: String,
}

/// `POST /api/v1/approvals/{id}/decision`.
pub(crate) async fn decision_handler(
    headers: HeaderMap,
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(body): Json<DecisionBody>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let decision = Decision::parse(&body.decision).ok_or(StatusCode::BAD_REQUEST)?;
    match global().decide(&id, decision, Decider::Console) {
        Ok(applied) => Ok(Json(
            json!({"ok": true, "repeated": applied == Applied::Repeated}),
        )),
        Err(DecideError::NotFound) => Err(StatusCode::NOT_FOUND),
        Err(DecideError::Conflict) => Err(StatusCode::CONFLICT),
        Err(DecideError::Gone) => Err(StatusCode::GONE),
    }
}

/// Body for a `Notification` hook with a permission prompt.
#[derive(Debug, Deserialize)]
pub struct NotifyBody {
    /// Agent-native session id.
    pub session_id: String,
}

/// `POST /api/v1/approvals/notify`: detection only. A permission prompt that nobody could
/// answer from here still produces the generic "needs approval" notice.
pub(crate) async fn notify_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(body): Json<NotifyBody>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    if !global().has_pending_for(&body.session_id) {
        let minute = now() / 60;
        let session: String = body.session_id.chars().take(64).collect();
        crate::automation::notice(
            &state.db,
            &format!("approval-prompt:{session}:{minute}"),
            "",
            "Agent needs approval. Review it on your laptop.",
        )
        .await;
    }
    Ok(Json(json!({"ok": true})))
}

/// Remote requests `approvals` (list) and `approval` (decide), already inside the E2EE channel.
pub(crate) fn remote_command(request: &Value) -> Value {
    let registry = global();
    match request.get("type").and_then(Value::as_str) {
        Some("approvals") => json!({"type": "approvals", "items": registry.pending()}),
        Some("approval") => {
            let id = request
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let Some(decision) = request
                .get("decision")
                .and_then(Value::as_str)
                .and_then(Decision::parse)
            else {
                return json!({"type": "error", "message": "Choose approve or deny."});
            };
            match registry.decide(id, decision, Decider::Phone) {
                Ok(applied) => json!({
                    "type": "approval_ack",
                    "id": id,
                    "decision": decision,
                    "repeated": applied == Applied::Repeated,
                }),
                Err(DecideError::NotFound) => {
                    json!({"type": "error", "message": "That approval is no longer waiting."})
                }
                Err(DecideError::Conflict) => {
                    json!({"type": "error", "message": "That approval was already answered differently."})
                }
                Err(DecideError::Gone) => {
                    json!({"type": "error", "message": "That approval expired. Answer it on your laptop."})
                }
            }
        }
        _ => json!({"type": "error", "message": "Unknown request"}),
    }
}

/// Hook-side client. Plain `std` sockets: no runtime, no dependency, hard time limits.
pub mod client {
    use serde_json::Value;
    use std::io::{Read, Write};
    use std::net::{Ipv4Addr, SocketAddr, TcpStream};
    use std::time::Duration;

    /// Connect budget for the hook (SPEC invariant 3).
    pub const CONNECT_BUDGET: Duration = Duration::from_millis(50);
    /// Budget for the daemon to answer a short request.
    pub const REPLY_BUDGET: Duration = Duration::from_millis(500);

    /// Where the running daemon listens.
    #[derive(Debug, Clone)]
    pub struct Endpoint {
        /// Loopback port.
        pub port: u16,
        /// Bearer token.
        pub token: String,
    }

    impl Endpoint {
        /// Reads `daemon.json`; `None` when the daemon is not running.
        #[must_use]
        pub fn from_daemon_info() -> Option<Self> {
            crate::read_daemon_info().map(|info| Self {
                port: info.port,
                token: info.token,
            })
        }
    }

    /// Sends one request and returns the JSON reply.
    ///
    /// `None` on any failure: no daemon, refused or slow connect, timeout, non-200 reply or
    /// unreadable body. Callers treat `None` as "no decision" and fall through.
    #[must_use]
    pub fn call(
        endpoint: &Endpoint,
        method: &str,
        path: &str,
        body: Option<&Value>,
        read_timeout: Duration,
    ) -> Option<Value> {
        let address = SocketAddr::from((Ipv4Addr::LOCALHOST, endpoint.port));
        let mut stream = TcpStream::connect_timeout(&address, CONNECT_BUDGET).ok()?;
        stream.set_write_timeout(Some(REPLY_BUDGET)).ok()?;
        stream.set_read_timeout(Some(read_timeout)).ok()?;
        let payload = body.map(Value::to_string).unwrap_or_default();
        let request = format!(
            "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
            endpoint.port,
            endpoint.token,
            payload.len()
        );
        stream.write_all(request.as_bytes()).ok()?;
        let mut raw = Vec::new();
        stream.take(64 * 1024).read_to_end(&mut raw).ok()?;
        let text = String::from_utf8(raw).ok()?;
        let (head, rest) = text.split_once("\r\n\r\n")?;
        let status_line = head.lines().next()?;
        if status_line.split_whitespace().nth(1) != Some("200") {
            return None;
        }
        serde_json::from_str(rest.trim()).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::approvals::client::{Endpoint, call};
    use axum::{body::Body, http::Request};
    use gnomcode_store::Database;
    use std::sync::Arc;
    use tokio::sync::Mutex as AsyncMutex;
    use tower::ServiceExt as _;

    fn state() -> AppState {
        AppState {
            db: Arc::new(AsyncMutex::new(
                Database::open_in_memory().expect("database"),
            )),
            bearer_token: "approval-test-token".to_owned(),
            orchestrator: None,
            remote: Arc::new(crate::remote::RemoteManager::new()),
        }
    }

    async fn serve(state: AppState) -> Endpoint {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let port = listener.local_addr().expect("addr").port();
        let token = state.bearer_token.clone();
        let router = crate::api::build_router(state);
        tokio::spawn(async move {
            let _ = axum::serve(listener, router).await;
        });
        Endpoint { port, token }
    }

    #[tokio::test]
    async fn decision_flow_is_single_use_and_rejects_wrong_ids() {
        let registry = Approvals::default();
        let item = registry
            .create("claude-code", "s", "Bash", "ls", 30)
            .expect("create");
        assert_eq!(
            registry.decide("0123456789abcdef", Decision::Allow, Decider::Phone),
            Err(DecideError::NotFound)
        );
        assert_eq!(registry.pending().len(), 1);
        assert_eq!(
            registry.decide(&item.id, Decision::Deny, Decider::Console),
            Ok(Applied::Recorded)
        );
        assert_eq!(
            registry.decide(&item.id, Decision::Allow, Decider::Phone),
            Err(DecideError::Conflict)
        );
        assert_eq!(registry.wait(&item.id).await, Outcome::Deny);
    }

    #[tokio::test]
    async fn wait_resolves_on_decision_and_expires_to_no_decision() {
        let registry = Arc::new(Approvals::default());
        let item = registry
            .create("claude-code", "s", "Bash", "ls", 30)
            .expect("create");
        let waiting = tokio::spawn({
            let registry = registry.clone();
            let id = item.id.clone();
            async move { registry.wait(&id).await }
        });
        tokio::time::sleep(Duration::from_millis(50)).await;
        registry
            .decide(&item.id, Decision::Allow, Decider::Phone)
            .expect("decide");
        assert_eq!(waiting.await.expect("join"), Outcome::Allow);

        let short = registry
            .create("claude-code", "s", "Bash", "ls", 1)
            .expect("create");
        let started = std::time::Instant::now();
        assert_eq!(registry.wait(&short.id).await, Outcome::NoDecision);
        assert!(started.elapsed() < Duration::from_secs(4));
        assert_eq!(
            registry.decide(&short.id, Decision::Allow, Decider::Phone),
            Err(DecideError::Gone)
        );
    }

    #[tokio::test]
    async fn http_requires_auth_and_maps_decision_errors() {
        let app = crate::api::build_router(state());
        let request = |method: &str, uri: &str, auth: &str, body: &str| {
            Request::builder()
                .method(method)
                .uri(uri)
                .header("host", "127.0.0.1:41738")
                .header("authorization", auth)
                .header("content-type", "application/json")
                .body(Body::from(body.to_owned()))
                .expect("request")
        };
        let unauthorized = app
            .clone()
            .oneshot(request("GET", "/api/v1/approvals", "Bearer wrong", ""))
            .await
            .expect("response");
        assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);
        let missing = app
            .clone()
            .oneshot(request(
                "POST",
                "/api/v1/approvals/ffffffffffffffff/decision",
                "Bearer approval-test-token",
                r#"{"decision":"allow"}"#,
            ))
            .await
            .expect("response");
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);
        let invalid = app
            .oneshot(request(
                "POST",
                "/api/v1/approvals/ffffffffffffffff/decision",
                "Bearer approval-test-token",
                r#"{"decision":"always"}"#,
            ))
            .await
            .expect("response");
        assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn hook_round_trip_over_loopback_then_disconnect_cancels() {
        let endpoint = serve(state()).await;
        let create = |endpoint: Endpoint| {
            tokio::task::spawn_blocking(move || {
                call(
                    &endpoint,
                    "POST",
                    "/api/v1/approvals",
                    Some(
                        &json!({"agent":"claude-code","session_id":"sess-e2e","tool":"Bash","summary":"echo hi"}),
                    ),
                    Duration::from_secs(2),
                )
            })
        };
        // Nobody is watching: the hook is told not to wait.
        let reply = create(endpoint.clone())
            .await
            .expect("join")
            .expect("reply");
        assert_eq!(reply["hold"], false);

        // The console polls, so it counts as an audience.
        let poll = {
            let endpoint = endpoint.clone();
            tokio::task::spawn_blocking(move || {
                call(
                    &endpoint,
                    "GET",
                    "/api/v1/approvals",
                    None,
                    Duration::from_secs(2),
                )
            })
        };
        poll.await.expect("join").expect("list");
        let reply = create(endpoint.clone())
            .await
            .expect("join")
            .expect("reply");
        assert_eq!(reply["hold"], true);
        let id = reply["id"].as_str().expect("id").to_owned();

        let waiter = {
            let (endpoint, id) = (endpoint.clone(), id.clone());
            tokio::task::spawn_blocking(move || {
                call(
                    &endpoint,
                    "GET",
                    &format!("/api/v1/approvals/{id}/wait"),
                    None,
                    Duration::from_secs(10),
                )
            })
        };
        tokio::time::sleep(Duration::from_millis(300)).await;
        let decided = {
            let (endpoint, id) = (endpoint.clone(), id.clone());
            tokio::task::spawn_blocking(move || {
                call(
                    &endpoint,
                    "POST",
                    &format!("/api/v1/approvals/{id}/decision"),
                    Some(&json!({"decision":"allow"})),
                    Duration::from_secs(2),
                )
            })
        };
        assert_eq!(decided.await.expect("join").expect("decision")["ok"], true);
        assert_eq!(
            waiter.await.expect("join").expect("wait")["outcome"],
            "allow"
        );

        // A hook that disconnects while waiting cancels its request.
        let reply = create(endpoint.clone())
            .await
            .expect("join")
            .expect("reply");
        let id = reply["id"].as_str().expect("id").to_owned();
        let abandoned = {
            let (endpoint, id) = (endpoint.clone(), id.clone());
            tokio::task::spawn_blocking(move || {
                call(
                    &endpoint,
                    "GET",
                    &format!("/api/v1/approvals/{id}/wait"),
                    None,
                    Duration::from_millis(300),
                )
            })
        };
        assert!(abandoned.await.expect("join").is_none());
        let mut cancelled = false;
        for _ in 0..40 {
            if !global().pending().iter().any(|item| item.id == id) {
                cancelled = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert!(cancelled, "disconnected hook must cancel its request");
    }

    #[test]
    fn hook_client_fails_open_fast_without_a_daemon() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();
        drop(listener);
        let started = std::time::Instant::now();
        let reply = call(
            &Endpoint {
                port,
                token: "x".into(),
            },
            "POST",
            "/api/v1/approvals",
            Some(&json!({})),
            Duration::from_secs(2),
        );
        assert!(reply.is_none());
        assert!(started.elapsed() < Duration::from_millis(500));
    }

    #[test]
    fn hook_client_gives_up_on_a_daemon_that_never_answers() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();
        let started = std::time::Instant::now();
        let reply = call(
            &Endpoint {
                port,
                token: "x".into(),
            },
            "POST",
            "/api/v1/approvals",
            Some(&json!({})),
            Duration::from_millis(300),
        );
        assert!(reply.is_none());
        assert!(started.elapsed() < Duration::from_secs(2));
        drop(listener);
    }

    #[test]
    fn remote_requests_list_decide_and_reject() {
        let item = global()
            .create("claude-code", "remote-test", "Bash", "ls", 30)
            .expect("create");
        let listed = remote_command(&json!({"type": "approvals"}));
        assert!(
            listed["items"]
                .as_array()
                .expect("items")
                .iter()
                .any(|entry| entry["id"] == item.id.as_str())
        );
        let wrong =
            remote_command(&json!({"type":"approval","id":"nope-nope","decision":"approve"}));
        assert_eq!(wrong["type"], "error");
        let bad = remote_command(&json!({"type":"approval","id":item.id,"decision":"always"}));
        assert_eq!(bad["type"], "error");
        let ack = remote_command(&json!({"type":"approval","id":item.id,"decision":"deny"}));
        assert_eq!(ack["type"], "approval_ack");
        let again = remote_command(&json!({"type":"approval","id":item.id,"decision":"deny"}));
        assert_eq!(again["repeated"], true);
        let flipped = remote_command(&json!({"type":"approval","id":item.id,"decision":"approve"}));
        assert_eq!(flipped["type"], "error");
    }
}
