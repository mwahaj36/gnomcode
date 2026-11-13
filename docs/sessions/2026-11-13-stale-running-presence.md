# 2026-11-13 - Stale Running presence

## Goal
Three Antigravity protocol-approval probe sessions showed Running with "Last activity 1h ago".

## Root cause
`OPEN_EXECUTION_LIMIT_MS` was two hours and `classify_presence` kept a `tool` or `thinking` state Running while the latest activity was an unmatched tool start. A killed agent or a denied or cancelled tool call never writes a matching end, so the state stayed Running for two hours. The Antigravity adapter also never resolved cancelled or denied steps, and nothing ended sessions whose Gnomcode host exited.

## Changes
- `core::presence`: open-execution limit is ten minutes; the approval-wait limit is its own two-hour constant. Thinking/tool is Running only within the limit. The #223 subagent rule is untouched (SQL child clause).
- `store`: the six presence SQL clauses use the shorter cutoff only; `end_exited_hosted_sessions` marks sessions ended when their bound host is not live.
- `daemon`: the automation tick calls the sweep using `gnomcode_pty::list_active_sessions` (platform process check); skipped when the host directory is unreadable.
- `adapters`: Antigravity CANCELED/DENIED/REJECTED/INTERRUPTED/ABORTED steps emit a tool-end and set idle.
- Keep-awake keeps its two-hour window via `APPROVAL_WAIT_LIMIT_MS`.

## Decisions
No live process evidence extends Running past ten minutes yet; a silent long tool run shows Idle until the log moves. Wiring host liveness into the list query is a follow-up.

## Verification
Unit tests for the presence boundary, the store ten-minute window, the host-exit sweep, and Antigravity fixtures `tool-pending.jsonl` and `tool-cancelled.jsonl`. Workspace fmt, clippy and tests.
