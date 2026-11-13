//! Codex `app-server` JSON-RPC protocol driver for phone and console approvals (ADR 0045).
//!
//! Translates Codex server-initiated approval requests (`item/commandExecution/requestApproval`,
//! `item/fileChange/requestApproval`, `item/permissions/requestApproval`, `applyPatchApproval`,
//! `execCommandApproval`) into ADR 0032 `/api/v1/approvals` requests. Nothing is ever
//! auto-approved: `NoDecision`, timeout, or missing listener resolves to `cancel`/`abort`/empty
//! permissions rather than `accept`.

use crate::Error;
use gnomcode_core::redact::redact_summary;
use serde_json::{Value, json};
use std::time::Duration;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt};

/// Decision outcome mapped from ADR 0032 `GET /api/v1/approvals/{id}/wait`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalOutcome {
    /// Approved once by the user.
    Allow,
    /// Denied by the user.
    Deny,
    /// No decision (no listener, expired, or cancelled): never auto-allow.
    NoDecision,
}

/// Parsed Codex `app-server` approval request ready for ADR 0032 registration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexApprovalRequest {
    /// JSON-RPC request `id` to echo in the response.
    pub rpc_id: Value,
    /// JSON-RPC method name.
    pub method: String,
    /// Normalized tool label (`Bash`, `Edit`, `Permissions`).
    pub tool: String,
    /// Redacted one-line summary (no file contents or secrets).
    pub summary: String,
    /// Raw `permissions` object when `method == "item/permissions/requestApproval"`.
    pub requested_permissions: Option<Value>,
}

/// Extracts an ADR 0032 approval request from a Codex `app-server` JSON-RPC server request.
/// Returns `None` for notifications or non-approval requests.
#[must_use]
pub fn parse_server_request(msg: &Value) -> Option<CodexApprovalRequest> {
    let rpc_id = msg.get("id")?.clone();
    if rpc_id.is_null() {
        return None;
    }
    let method = msg.get("method").and_then(Value::as_str)?;
    let params = msg.get("params").unwrap_or(&Value::Null);

    match method {
        "item/commandExecution/requestApproval" | "execCommandApproval" => {
            let command = params
                .get("command")
                .and_then(|v| {
                    v.as_str().map(str::to_owned).or_else(|| {
                        v.as_array().map(|arr| {
                            arr.iter()
                                .filter_map(Value::as_str)
                                .collect::<Vec<_>>()
                                .join(" ")
                        })
                    })
                })
                .filter(|s| !s.is_empty())
                .or_else(|| {
                    params
                        .get("reason")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                })
                .unwrap_or_else(|| "Run command".to_owned());
            Some(CodexApprovalRequest {
                rpc_id,
                method: method.to_owned(),
                tool: "Bash".to_owned(),
                summary: redact_summary("Bash", &command),
                requested_permissions: None,
            })
        }
        "item/fileChange/requestApproval" | "applyPatchApproval" => {
            let detail = params
                .get("reason")
                .and_then(Value::as_str)
                .or_else(|| params.get("grantRoot").and_then(Value::as_str))
                .unwrap_or("Apply workspace file changes");
            Some(CodexApprovalRequest {
                rpc_id,
                method: method.to_owned(),
                tool: "Edit".to_owned(),
                summary: redact_summary("Edit", detail),
                requested_permissions: None,
            })
        }
        "item/permissions/requestApproval" => {
            let detail = params
                .get("reason")
                .and_then(Value::as_str)
                .unwrap_or("Additional sandbox permissions");
            Some(CodexApprovalRequest {
                rpc_id,
                method: method.to_owned(),
                tool: "Permissions".to_owned(),
                summary: redact_summary("Permissions", detail),
                requested_permissions: params.get("permissions").cloned(),
            })
        }
        _ => None,
    }
}

/// Builds the JSON-RPC response payload for a Codex `app-server` approval request.
///
/// Safety invariant: `ApprovalOutcome::NoDecision` NEVER maps to `accept` / `approved`.
#[must_use]
pub fn build_approval_response(req: &CodexApprovalRequest, outcome: ApprovalOutcome) -> Value {
    let result = match req.method.as_str() {
        "item/commandExecution/requestApproval" | "item/fileChange/requestApproval" => {
            let decision = match outcome {
                ApprovalOutcome::Allow => "accept",
                ApprovalOutcome::Deny => "decline",
                ApprovalOutcome::NoDecision => "cancel",
            };
            json!({ "decision": decision })
        }
        "execCommandApproval" | "applyPatchApproval" => {
            let decision = match outcome {
                ApprovalOutcome::Allow => "approved",
                ApprovalOutcome::Deny => "denied",
                ApprovalOutcome::NoDecision => "abort",
            };
            json!({ "decision": decision })
        }
        "item/permissions/requestApproval" => {
            let permissions = match outcome {
                ApprovalOutcome::Allow => req
                    .requested_permissions
                    .clone()
                    .unwrap_or_else(|| json!({})),
                ApprovalOutcome::Deny | ApprovalOutcome::NoDecision => json!({}),
            };
            json!({ "permissions": permissions })
        }
        _ => json!({ "decision": "cancel" }),
    };

    json!({
        "jsonrpc": "2.0",
        "id": req.rpc_id,
        "result": result,
    })
}

/// Sends `initialize` and `initialized` to a Codex `app-server` stdio stream.
///
/// # Errors
/// Returns [`Error::Process`] if writing fails.
pub async fn send_initialize<W: AsyncWrite + Unpin>(writer: &mut W, id: u64) -> Result<(), Error> {
    let init = json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "initialize",
        "params": {
            "clientInfo": {
                "name": "gnomcode",
                "title": "Gnomcode",
                "version": env!("CARGO_PKG_VERSION"),
            },
            "capabilities": {
                "experimentalApi": true,
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
        .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"initialized\"}\n")
        .await
        .map_err(|e| Error::Process(e.to_string()))?;
    writer
        .flush()
        .await
        .map_err(|e| Error::Process(e.to_string()))
}

/// Bridges Codex `app-server` approval requests from `reader` through `decide`,
/// writing JSON-RPC responses to `writer`.
///
/// # Errors
/// Returns [`Error::Process`] on I/O failure.
pub async fn pump_approvals<R, W, F, Fut>(
    reader: R,
    mut writer: W,
    mut decide: F,
) -> Result<(), Error>
where
    R: AsyncBufRead + Unpin,
    W: AsyncWrite + Unpin,
    F: FnMut(CodexApprovalRequest) -> Fut,
    Fut: std::future::Future<Output = ApprovalOutcome>,
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
        let Some(req) = parse_server_request(&msg) else {
            continue;
        };
        let outcome = decide(req.clone()).await;
        let resp = build_approval_response(&req, outcome);
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

/// Forwards one approval request to the daemon's ADR 0032 `/api/v1/approvals` API
/// and waits up to the daemon's hold window for a decision.
pub async fn request_daemon_approval(
    daemon_port: u16,
    bearer_token: &str,
    session_id: &str,
    req: &CodexApprovalRequest,
) -> ApprovalOutcome {
    let Ok(client) = reqwest::Client::builder()
        .no_proxy()
        .connect_timeout(Duration::from_millis(50))
        .timeout(Duration::from_secs(95))
        .build()
    else {
        return ApprovalOutcome::NoDecision;
    };
    let create_url = format!("http://127.0.0.1:{daemon_port}/api/v1/approvals");
    let Ok(create_resp) = client
        .post(&create_url)
        .header("Authorization", format!("Bearer {bearer_token}"))
        .json(&json!({
            "agent": "codex",
            "session_id": session_id,
            "tool": req.tool,
            "summary": req.summary,
        }))
        .send()
        .await
    else {
        return ApprovalOutcome::NoDecision;
    };
    let Ok(created) = create_resp.json::<Value>().await else {
        return ApprovalOutcome::NoDecision;
    };
    if created.get("hold").and_then(Value::as_bool) != Some(true) {
        return ApprovalOutcome::NoDecision;
    }
    let Some(id) = created.get("id").and_then(Value::as_str) else {
        return ApprovalOutcome::NoDecision;
    };
    let wait_url = format!("http://127.0.0.1:{daemon_port}/api/v1/approvals/{id}/wait");
    let Ok(wait_resp) = client
        .get(&wait_url)
        .header("Authorization", format!("Bearer {bearer_token}"))
        .send()
        .await
    else {
        return ApprovalOutcome::NoDecision;
    };
    let Ok(waited) = wait_resp.json::<Value>().await else {
        return ApprovalOutcome::NoDecision;
    };
    match waited.get("decision").and_then(Value::as_str) {
        Some("allow") => ApprovalOutcome::Allow,
        Some("deny") => ApprovalOutcome::Deny,
        _ => ApprovalOutcome::NoDecision,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_decision_never_auto_approves_codex_requests() {
        let cases = [
            json!({
                "jsonrpc": "2.0",
                "id": 10,
                "method": "item/commandExecution/requestApproval",
                "params": { "command": "cargo test", "threadId": "t1", "turnId": "u1", "itemId": "i1" }
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 11,
                "method": "item/fileChange/requestApproval",
                "params": { "reason": "Write src/main.rs", "threadId": "t1", "turnId": "u1", "itemId": "i2", "startedAtMs": 1 }
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 12,
                "method": "item/permissions/requestApproval",
                "params": { "reason": "Read /tmp", "permissions": { "network": { "enabled": true } } }
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 13,
                "method": "applyPatchApproval",
                "params": { "reason": "Patch file" }
            }),
        ];

        for raw in cases {
            let req = parse_server_request(&raw).expect("approval request");
            let resp = build_approval_response(&req, ApprovalOutcome::NoDecision);
            let result = resp.get("result").expect("result");
            assert_ne!(
                result.get("decision").and_then(Value::as_str),
                Some("accept")
            );
            assert_ne!(
                result.get("decision").and_then(Value::as_str),
                Some("approved")
            );
            if req.method == "item/permissions/requestApproval" {
                assert_eq!(result.get("permissions"), Some(&json!({})));
            }
        }
    }

    #[tokio::test]
    async fn pump_approvals_bridges_allow_and_deny_responses() {
        let input = concat!(
            "{\"jsonrpc\":\"2.0\",\"method\":\"turn/started\",\"params\":{}}\n",
            "{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"item/commandExecution/requestApproval\",\"params\":{\"command\":\"git status --token secret123\"}}\n",
            "{\"jsonrpc\":\"2.0\",\"id\":8,\"method\":\"item/fileChange/requestApproval\",\"params\":{\"reason\":\"Edit config\"}}\n",
        );
        let mut out = Vec::new();
        pump_approvals(input.as_bytes(), &mut out, |req| async move {
            assert!(!req.summary.contains("secret123"));
            if req.rpc_id == json!(7) {
                ApprovalOutcome::Allow
            } else {
                ApprovalOutcome::Deny
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
        assert_eq!(lines[0]["id"], json!(7));
        assert_eq!(lines[0]["result"]["decision"], json!("accept"));
        assert_eq!(lines[1]["id"], json!(8));
        assert_eq!(lines[1]["result"]["decision"], json!("decline"));
    }
}
