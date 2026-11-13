# 2026-11-13 - Unix process-group stop escalation (G12)

## Goals
- Ensure `gnomcode_platform::stop_owned_process` terminates interactive TUIs and child tool processes on Unix rather than sending a single `SIGINT` to one PID.
- Keep Windows `taskkill /F /T` behavior unchanged.

## Outcomes
- Added `#[cfg(unix)] mod unix_stop` in `crates/platform/src/lib.rs` with `SIGINT` -> `SIGTERM` -> `SIGKILL` escalation (250 ms wait per stage) targeting the child's process group when distinct from Gnomcode's own process group (`getpgid(pid) != getpgrp()`), falling back to single-PID signaling if they match.
- Added pure unit test `unix_stop_plan_escalates_and_guards_own_process_group` and `#[cfg(unix)]` test `unix_stop_escalates_past_ignored_sigint`.
- Updated `docs/SPEC.md` §3.3.

## Decisions
- Guarded `killpg` on `target_pgid != own_pgid` so a test or non-setsid child never signals Gnomcode's own process group.
- Kept `#[cfg(windows)]` completely untouched.

## Verification
- `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` on Windows (`CARGO_INCREMENTAL=0`).
- Note: `#[cfg(unix)]` code paths and `unix_stop_escalates_past_ignored_sigint` require macOS/Linux CI verification.
