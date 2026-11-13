# ADR 0047: Antigravity `agy acp` Protocol Approvals

- Status: Accepted
- Date: 2026-11-13

## Context

Interactive `agy` PTY sessions cannot safely answer tool permission dialogs from the phone or console. `agy acp` (verified live against `Antigravity ACP Server 1.5.7` via stdio JSON-RPC `initialize`) exposes structured Agent Client Protocol (ACP) `session/request_permission` requests.

## Decision

1. Implement `crates/pty/src/acp_approvals.rs` to parse ACP `session/request_permission` requests, redact tool call summaries with `gnomcode_core::redact::redact_summary`, and forward them to ADR 0032 (`POST /api/v1/approvals` + `GET /api/v1/approvals/{id}/wait`).
2. Enforce fail-closed behavior: `NoDecision` (no listener, timeout, or cancellation) maps to `{ "outcome": { "outcome": "cancelled" } }` and NEVER selects an allow option.

## Consequences

- Reuses ADR 0032's single-use approval state machine, HTTP API, and Remote push/dispatch.
- Never passes `--dangerously-skip-permissions`.
