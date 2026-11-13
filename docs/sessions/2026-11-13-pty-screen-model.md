# 2026-11-13 - PTY VT100 screen model and Enter recovery

## Goals
- Implement G3: replace raw scrollback CSI regex stripping with a `vt100` terminal screen model in `crates/pty`, keeping the Codex OSC fast path.
- Implement G14: add guarded swallowed-Enter recovery (`POST /retry-enter` and `retry_enter`) that fires only when the rendered screen proves the prompt is still in the composer and no dialog is open.

## Outcomes
- Added `vt100 = "0.15"` (MIT) to workspace dependencies and verified `cargo deny check`.
- Updated `Scrollback`, `screen_text`, and `codex_prompt_ready` in `crates/pty/src/lib.rs` to render via `vt100`.
- Added `composer_has_unsubmitted_prompt`, `host_retry_enter_handler`, `retry_enter`, and unit tests.
- Ticked G3/G14 in `docs/ROADMAP_1.0.md` and updated `docs/SPEC.md`.

## Verification
- `cargo deny check`
- `CARGO_INCREMENTAL=0 cargo fmt --all`
- `CARGO_INCREMENTAL=0 cargo clippy --workspace --all-targets -- -D warnings`
- `CARGO_INCREMENTAL=0 cargo test --workspace`
