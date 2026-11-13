# ADR 0045: Codex `app-server` Protocol Approvals

- Status: Accepted
- Date: 2026-11-13

## Context

Interactive Codex PTY sessions cannot safely answer permission prompts from the phone or console. `codex app-server` (verified live on `codex-cli 0.166.0` via `initialize` + `generate-json-schema`) exposes structured JSON-RPC 2.0 server requests for command execution, file changes, patch approvals, and sandbox permissions.

## Decision

1. Implement `crates/pty/src/codex_app_server.rs` to parse Codex `ServerRequest` approval methods (`item/commandExecution/requestApproval`, `item/fileChange/requestApproval`, `item/permissions/requestApproval`, `applyPatchApproval`, `execCommandApproval`), redact summaries with `gnomcode_core::approval::redact_summary`, and forward them to ADR 0032 (`POST /api/v1/approvals` + `GET /api/v1/approvals/{id}/wait`).
2. Enforce fail-closed behavior: `NoDecision` (no listener, timeout, or cancellation) maps to `cancel` / `abort` / `{}` permissions and NEVER maps to `accept` or `approved`.

## Consequences

- Reuses the existing ADR 0032 approvals UI, Remote push/dispatch, and single-use decision state machine.
- Does not modify `~/.codex` or pass any permission-bypass flags.
