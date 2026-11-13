# 2026-11-13 - Codex app-server protocol approvals

## Goals
- Verify `codex app-server` live in a temp workspace and implement the ADR 0032 approvals bridge for Codex protocol mode.

## Outcomes
- Probed `codex app-server` (`codex-cli 0.166.0`) live in a temp workspace (`initialize` + `generate-json-schema` for `ServerRequest` and approval response schemas).
- Added `crates/pty/src/codex_app_server.rs`, `docs/adr/0045-codex-app-server-approvals.md`, and updated `docs/SPEC.md`.

## Verification
- `CARGO_INCREMENTAL=0 cargo fmt --all`
- `CARGO_INCREMENTAL=0 cargo clippy --workspace --all-targets -- -D warnings`
- `CARGO_INCREMENTAL=0 cargo test --workspace`
