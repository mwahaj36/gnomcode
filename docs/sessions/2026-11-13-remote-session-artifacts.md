# 2026-11-13 - Read-only session artifacts API and Remote RPC (ADR 0039)

## Goals
- Expose read-only session artifacts (created/edited files, HTML, Markdown, images) and `changes_count` over `GET /api/v1/sessions/{id}/artifacts` and E2EE Remote `artifacts`.

## Outcomes
- Added `crates/daemon/src/session_artifacts.rs` with workspace confinement, prioritized rich-artifact ordering, 30-item / 256 KiB text / 512 KiB image caps, and `git status --porcelain=v1 -z` inspection.
- Wired route in `crates/daemon/src/api.rs`, Remote `artifacts` arm in `crates/daemon/src/remote.rs`, and `mod session_artifacts` in `crates/daemon/src/lib.rs`.
- Added ADR 0039 (`docs/adr/0039-remote-session-artifacts.md`) and updated `docs/SPEC.md`.

## Verification
- `session_artifacts::tests::remote_artifacts_collects_html_markdown_and_image_outputs`
- Full Rust gate (`cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`) with `CARGO_INCREMENTAL=0`.
