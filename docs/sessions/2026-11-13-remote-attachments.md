# 2026-11-13 - Workspace attachment upload API and Remote RPC (ADR 0041)

## Goals
- Provide bounded, allowlisted attachment staging inside the session workspace over `POST /api/v1/sessions/{id}/attachments` and E2EE Remote `upload_attachment` -> `attachment_ack`.

## Outcomes
- Added `crates/daemon/src/attachments.rs` with extension allowlist, 15 MiB per-file and 200 MiB per-session caps, basename sanitization + random prefix, and canonical containment in `<workspace>/.gnomcode/attachments/`.
- Wired route in `crates/daemon/src/api.rs` (22 MiB body limit for base64 JSON), Remote `upload_attachment` arm in `crates/daemon/src/remote.rs`, and `mod attachments` in `crates/daemon/src/lib.rs`.
- Added ADR 0041 (`docs/adr/0041-remote-attachments.md`) and updated `docs/SPEC.md`.

## Verification
- `attachments::tests::remote_upload_attachment_saves_sanitized_file_and_enforces_limits`
- Full Rust gate (`cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`) with `CARGO_INCREMENTAL=0`.
