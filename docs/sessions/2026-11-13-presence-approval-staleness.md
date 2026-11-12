# 2026-11-13 - Approval-wait staleness limit (research gap G9)

## Goals
- Stop a transcript-derived `awaiting_approval` session from staying in Needs you for 24 hours without fresh evidence.

## Outcomes
- `core::presence`: `STALE_EXECUTION_LIMIT_MS` (24 h) replaced by `APPROVAL_WAIT_LIMIT_MS`, equal to `OPEN_EXECUTION_LIMIT_MS` (2 h). Older approval waits classify as `Inactive`, never `Ended`.
- `store` session list SQL uses the same constant, so counts and filters match the classifier.
- SPEC 3.7 and ADR 0010 amended.

## Problems encountered
- Gap verified: `presence.rs` used the 24 h limit for `awaiting_approval` while running states used 15 min and 2 h (audit D6 fixed only those).
- `launch_lifecycle::fail_open_agent_process_survives_independent_lifecycle` failed once in the CLI crate on Windows during the first workspace run (stop endpoint returned 500); see verification.

## Decisions
- Reused the existing 2 h open-execution bound instead of inventing a new value. The unused `approval_wait_delay_secs` setting was left alone.

## Verification
- Test-first: updated classifier test failed to compile, then passed with fixed ages (1 h, 2 h, 2 h + 1 ms, 24 h).
- cargo fmt, clippy -D warnings and cargo test --workspace (see PR).
