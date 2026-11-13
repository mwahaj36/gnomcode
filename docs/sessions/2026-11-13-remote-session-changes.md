# 2026-11-13 - Read-only per-session changes and diff over Remote (ADR 0037)

## Goals
- Expose read-only per-session git status and unified diff over `GET /api/v1/sessions/{id}/changes` and E2EE Remote `changes`.

## Outcomes
- Added `crates/daemon/src/session_changes.rs` with session workspace resolution, `GIT_OPTIONAL_LOCKS=0`, `GIT_TERMINAL_PROMPT=0`, `GIT_LITERAL_PATHSPECS=1`, `LC_ALL=C`, `core.quotepath=off`, `--porcelain=v1 -z`, 1 MiB / 10 s limits, and lossy UTF-8 decoding.
- Wired route in `crates/daemon/src/api.rs`, Remote `changes` arm in `crates/daemon/src/remote.rs`, and `mod session_changes` in `crates/daemon/src/lib.rs`.
- Added ADR 0037 (`docs/adr/0037-remote-session-changes.md`) and updated `docs/SPEC.md`.

## Verification
- `session_changes::tests::remote_changes_returns_git_status_and_diff_and_handles_non_utf8_lossily`
- `session_changes::tests::remote_changes_rejects_non_git_workspace`
- Full Rust gate (`cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`) with `CARGO_INCREMENTAL=0`.
