# 2026-11-13 - Remote quick-actions storage

## Goals
- Implement Section C quick-actions storage APIs for HTTP and E2EE Remote.

## Outcomes
- Added `crates/daemon/src/quick_actions.rs` with bounded validation, atomic persistence under `GNOMCODE_HOME/quick_actions.json`, HTTP `GET`/`PUT /api/v1/quick-actions`, and Remote `quick_actions` dispatch.
- Added `docs/adr/0043-remote-quick-actions.md` and updated `docs/SPEC.md`.

## Verification
- `CARGO_INCREMENTAL=0 cargo fmt --all`
- `CARGO_INCREMENTAL=0 cargo clippy --workspace --all-targets -- -D warnings`
- `CARGO_INCREMENTAL=0 cargo test --workspace`
