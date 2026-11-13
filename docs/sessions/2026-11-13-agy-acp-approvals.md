# 2026-11-13 - Antigravity ACP protocol approvals

## Goals
- Verify `agy acp` live in a temp workspace and implement the ADR 0032 approvals bridge for `agy acp`.

## Outcomes
- Probed `agy acp` (`Antigravity ACP Server 1.5.7`) live in a temp workspace (`initialize` JSON-RPC 2.0 `protocolVersion: 1`).
- Added `crates/pty/src/acp_approvals.rs`, `docs/adr/0047-agy-acp-approvals.md`, and updated `docs/SPEC.md`.

## Verification
- `CARGO_INCREMENTAL=0 cargo fmt --all`
- `CARGO_INCREMENTAL=0 cargo clippy --workspace --all-targets -- -D warnings`
- `CARGO_INCREMENTAL=0 cargo test --workspace`
