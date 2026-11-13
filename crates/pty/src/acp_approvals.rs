//! Agent Client Protocol (ACP) JSON-RPC approval bridge for `agy acp` (ADR 0047).
//!
//! Translates ACP `session/request_permission` server requests into ADR 0032
//! `/api/v1/approvals` requests. Never auto-approves: `NoDecision`, timeout, or missing
//! listener resolves to `{ "outcome": { "outcome": "cancelled" } }` rather than selecting
//! an allow option.

use crate::Error;
use gnomcode_core::redact::redact_summary;
use serde_json::{Value, json};
use std::time::Duration;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt};

/// Decision outcome mapped from ADR 0032 `GET /api/v1/approvals/{id}/wait`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcpApprovalOutcome {
    /// Approved once by the user.
    Allow,
    /// Denied by the user.
    Deny,
    /// No decision (no listener, expired, or cancelled): never auto-allow.
    NoDecision,
}

/// Parsed ACP `session/request_permission` request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcpPermissionRequest {
    /// JSON-RPC request `id` to echo in the response.
    pub rpc_id: Value,
    /// Normalized tool label.
    pub tool: String,
    /// Redacted one-line summary.
    pub summary: String,
    /// Option id to select when allowed once.
    pub allow_option_id: Option<String>,
    /// Option id to select when denied.
    pub deny_option_id: Option<String>,
}

/// Extracts an ADR 0032 approval request from an ACP JSON-RPC `session/request_permission` message.
#[must_use]
pub fn parse_permission_request(msg: &Value) -> Option<AcpPermissionRequest> {
    let rpc_id = msg.get("id")?.clone();
    if rpc_id.is_null() {
        return None;
    }
    if msg.get("method").and_then(Value::as_str) != Some("session/request_permission") {
        return None;
    }
    let params = msg.get("params")?;
    let tool_call = params.get("toolCall").unwrap_or(&Value::Null);
    let tool = tool_call
        .get("kind")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or("Tool")
        .to_owned();
    let title = tool_call
        .get("title")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or("Approve tool call");

    let mut allow_once = None;
    let mut reject_once = None;
    let mut reject_any = None;

    if let Some(options) = params.get("options").and_then(Value::as_array) {
        for opt in options {
            let Some(option_id) = opt.get("optionId").and_then(Value::as_str) else {
                continue;
            };
            let kind = opt.get("kind").and_then(Value::as_str).unwrap_or("");
            match kind {
                "allow_once" if allow_once.is_none() => allow_once = Some(option_id.to_owned()),
                // A one-time phone "allow" must never become a persistent grant.
                "allow_always" => {}
                "reject_once" if reject_once.is_none() => reject_once = Some(option_id.to_owned()),
                "reject_always" if reject_any.is_none() => reject_any = Some(option_id.to_owned()),
                _ => {
                    if option_id.eq_ignore_ascii_case("deny") && reject_any.is_none() {
                        reject_any = Some(option_id.to_owned());
                    }
                }
            }
        }
    }

    Some(AcpPermissionRequest {
        rpc_id,
        tool: tool.clone(),
        summary: redact_summary(&tool, title),
        allow_option_id: allow_once,
        deny_option_id: reject_once.or(reject_any),
    })
}

/// Builds the ACP JSON-RPC response for `session/request_permission`.
///
/// Safety invariant: `AcpApprovalOutcome::NoDecision` ALWAYS maps to `cancelled` and
/// NEVER selects an allow option.
#[must_use]
pub fn build_permission_response(req: &AcpPermissionRequest, outcome: AcpApprovalOutcome) -> Value {
    let outcome_obj = match outcome {
        AcpApprovalOutcome::Allow => match req.allow_option_id.as_deref() {
            Some(option_id) => json!({ "outcome": "selected", "optionId": option_id }),
            None => json!({ "outcome": "cancelled" }),
        },
        AcpApprovalOutcome::Deny => match req.deny_option_id.as_deref() {
            Some(option_id) => json!({ "outcome": "selected", "optionId": option_id }),
            None => json!({ "outcome": "cancelled" }),
        },
        AcpApprovalOutcome::NoDecision => json!({ "outcome": "cancelled" }),
    };

    json!({
        "jsonrpc": "2.0",
        "id": req.rpc_id,
        "result": {
            "outcome": outcome_obj,
        }
    })
}

/// Sends ACP `initialize` (`protocolVersion: 1`) over stdio.
///
/// # Errors
/// Returns [`Error::Process`] if writing fails.
pub async fn send_initialize<W: AsyncWrite + Unpin>(writer: &mut W, id: u64) -> Result<(), Error> {
    let init = json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "initialize",
        "params": {
            "protocolVersion": 1,
            "clientCapabilities": {},
            "clientInfo": {
                "name": "gnomcode",
                "version": env!("CARGO_PKG_VERSION"),
            }
        }
    });
    let mut line = serde_json::to_string(&init).map_err(|e| Error::Process(e.to_string()))?;
    line.push('\n');
    writer
        .write_all(line.as_bytes())
        .await
        .map_err(|e| Error::Process(e.to_string()))?;
    writer
        .flush()
        .await
        .map_err(|e| Error::Process(e.to_string()))
}

/// Bridges ACP `session/request_permission` requests from `reader` through `decide`,
/// writing JSON-RPC responses to `writer`.
///
/// # Errors
/// Returns [`Error::Process`] on I/O failure.
pub async fn pump_permission_requests<R, W, F, Fut>(
    reader: R,
    mut writer: W,
    mut decide: F,
) -> Result<(), Error>
where
    R: AsyncBufRead + Unpin,
    W: AsyncWrite + Unpin,
    F: FnMut(AcpPermissionRequest) -> Fut,
    Fut: std::future::Future<Output = AcpApprovalOutcome>,
{
    let mut lines = reader.lines();
    while let Some(line) = lines
        .next_line()
        .await
        .map_err(|e| Error::Process(e.to_string()))?
    {
        let Ok(msg) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let Some(req) = parse_permission_request(&msg) else {
            continue;
        };
        let outcome = decide(req.clone()).await;
        let resp = build_permission_response(&req, outcome);
        let mut out = serde_json::to_string(&resp).map_err(|e| Error::Process(e.to_string()))?;
        out.push('\n');
        writer
            .write_all(out.as_bytes())
            .await
            .map_err(|e| Error::Process(e.to_string()))?;
        writer
            .flush()
            .await
            .map_err(|e| Error::Process(e.to_string()))?;
    }
    Ok(())
}

/// Forwards an ACP permission request for `agent` (`"antigravity"` or `"gemini-cli"`)
/// to the daemon's ADR 0032 `/api/v1/approvals` API.
pub async fn request_daemon_approval(
    daemon_port: u16,
    bearer_token: &str,
    agent: &str,
    session_id: &str,
    req: &AcpPermissionRequest,
) -> AcpApprovalOutcome {
    let Ok(client) = reqwest::Client::builder()
        .no_proxy()
        .connect_timeout(Duration::from_millis(50))
        .timeout(Duration::from_secs(95))
        .build()
    else {
        return AcpApprovalOutcome::NoDecision;
    };
    let create_url = format!("http://127.0.0.1:{daemon_port}/api/v1/approvals");
    let Ok(create_resp) = client
        .post(&create_url)
        .header("Authorization", format!("Bearer {bearer_token}"))
        .json(&json!({
            "agent": agent,
            "session_id": session_id,
            "tool": req.tool,
            "summary": req.summary,
        }))
        .send()
        .await
    else {
        return AcpApprovalOutcome::NoDecision;
    };
    let Ok(created) = create_resp.json::<Value>().await else {
        return AcpApprovalOutcome::NoDecision;
    };
    if created.get("hold").and_then(Value::as_bool) != Some(true) {
        return AcpApprovalOutcome::NoDecision;
    }
    let Some(id) = created.get("id").and_then(Value::as_str) else {
        return AcpApprovalOutcome::NoDecision;
    };
    let wait_url = format!("http://127.0.0.1:{daemon_port}/api/v1/approvals/{id}/wait");
    let Ok(wait_resp) = client
        .get(&wait_url)
        .header("Authorization", format!("Bearer {bearer_token}"))
        .send()
        .await
    else {
        return AcpApprovalOutcome::NoDecision;
    };
    let Ok(waited) = wait_resp.json::<Value>().await else {
        return AcpApprovalOutcome::NoDecision;
    };
    match waited.get("outcome").and_then(Value::as_str) {
        Some("allow") => AcpApprovalOutcome::Allow,
        Some("deny") => AcpApprovalOutcome::Deny,
        _ => AcpApprovalOutcome::NoDecision,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_decision_always_cancels_and_never_auto_allows_acp() {
        let raw = json!({
            "jsonrpc": "2.0",
            "id": 42,
            "method": "session/request_permission",
            "params": {
                "sessionId": "s1",
                "toolCall": { "toolCallId": "tc1", "kind": "execute", "title": "Run cargo test" },
                "options": [
                    { "optionId": "allow", "name": "Allow once", "kind": "allow_once" },
                    { "optionId": "deny", "name": "Deny", "kind": "reject_once" }
                ]
            }
        });
        let req = parse_permission_request(&raw).expect("acp request");
        let resp = build_permission_response(&req, AcpApprovalOutcome::NoDecision);
        assert_eq!(resp["result"]["outcome"]["outcome"], json!("cancelled"));
        assert!(resp["result"]["outcome"].get("optionId").is_none());
    }

    #[test]
    fn allow_never_selects_an_always_option() {
        let raw = json!({
            "jsonrpc": "2.0", "id": 7, "method": "session/request_permission",
            "params": { "toolCall": { "kind": "execute", "title": "Run tests" },
                "options": [
                    { "optionId": "always", "kind": "allow_always" },
                    { "optionId": "no", "kind": "reject_once" }
                ] }
        });
        let req = parse_permission_request(&raw).expect("acp request");
        let resp = build_permission_response(&req, AcpApprovalOutcome::Allow);
        assert_eq!(resp["result"]["outcome"]["outcome"], json!("cancelled"));
    }

    #[tokio::test]
    async fn pump_permission_requests_bridges_allow_and_deny() {
        let input = concat!(
            "{\"jsonrpc\":\"2.0\",\"method\":\"session/update\",\"params\":{}}\n",
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"session/request_permission\",\"params\":{\"sessionId\":\"s1\",\"toolCall\":{\"kind\":\"edit\",\"title\":\"Edit main.rs --token secret99\"},\"options\":[{\"optionId\":\"opt-allow\",\"kind\":\"allow_once\"},{\"optionId\":\"opt-deny\",\"kind\":\"reject_once\"}]}}\n",
            "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"session/request_permission\",\"params\":{\"sessionId\":\"s1\",\"toolCall\":{\"kind\":\"execute\",\"title\":\"rm -rf\"},\"options\":[{\"optionId\":\"opt-allow\",\"kind\":\"allow_once\"},{\"optionId\":\"opt-deny\",\"kind\":\"reject_once\"}]}}\n",
        );
        let mut out = Vec::new();
        pump_permission_requests(input.as_bytes(), &mut out, |req| async move {
            assert!(!req.summary.contains("secret99"));
            if req.rpc_id == json!(1) {
                AcpApprovalOutcome::Allow
            } else {
                AcpApprovalOutcome::Deny
            }
        })
        .await
        .expect("pump");

        let lines: Vec<Value> = String::from_utf8(out)
            .expect("utf8")
            .lines()
            .map(|l| serde_json::from_str(l).expect("json"))
            .collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0]["id"], json!(1));
        assert_eq!(
            lines[0]["result"]["outcome"]["optionId"],
            json!("opt-allow")
        );
        assert_eq!(lines[1]["id"], json!(2));
        assert_eq!(lines[1]["result"]["outcome"]["optionId"], json!("opt-deny"));
    }
}
