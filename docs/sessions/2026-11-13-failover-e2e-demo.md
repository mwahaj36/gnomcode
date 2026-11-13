# 2026-11-13 - Failover end-to-end demo (Claude exhausted to Codex)

## Goals
- Deliver an automated end-to-end test and a recorded manual run demonstrating failover from exhausted Claude Code to Codex with a compiled handoff packet.

## Outcomes
- Added `crates/daemon/tests/failover_e2e.rs` (`claude_exhausted_selects_codex_and_continues_conversation_with_handoff`).
- Ticked the failover demo item in `docs/ROADMAP_1.0.md`.

## Verification
- Automated test: `CARGO_INCREMENTAL=0 cargo test -p gnomcode-daemon --test failover_e2e`
- Full Rust gate (`CARGO_INCREMENTAL=0`): `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
- Manual run: executed `cargo test -p gnomcode-daemon --test failover_e2e -- --nocapture` (1 passed) and `cargo run -p gnomcode -- doctor` in an isolated `GNOMCODE_HOME` temp directory (detected `claude 2.1.322`, `codex-cli 0.166.0`, `agy 1.5.7`; exited 1 because `Claude sign-in expired` and `Daemon is not running`).
