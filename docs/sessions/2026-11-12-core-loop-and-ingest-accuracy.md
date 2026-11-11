# 2026-11-12 — Core loop fixes and ingest accuracy

## Goals
Fix the verified audit defects in the core loop and prove Claude Code, Codex and Antigravity data match their raw logs.

## Outcomes
- Claude Code bundled with the Claude desktop app is detected and launchable; `claude-desktop` sessions use the desktop surface.
- Ingest pass on a real 17k-file Antigravity tree: ~1260 ms to ~75 ms (fixed transcript paths instead of recursive walk).
- Observed sessions no longer show Running hours after their last log line (2 h open-execution cap; list uses shared presence rules).
- Limits page shows percent left, reset countdown, exhausted and reset-passed states; nav rail has hover intent.
- New discovery: Codex `archived_sessions` and `.jsonl.zst` archives (23 to 131 sessions on the test machine), legacy `antigravity-ide` (68 to 183 sessions).
- Accuracy: Codex re-emitted token counts no longer double-count (~1%); forked Codex rollouts keep their own identity (7 missing sessions recovered); inverted Antigravity parent links fixed and repaired from Antigravity's summaries; Claude subagents become linked child sessions.

## Root causes
- Archived Codex rollouts were invisible because only `.jsonl` under `sessions` was scanned.
- Every Codex `session_meta` overwrote identity; forks replay the parent header.
- Antigravity treated any `sender=` system message as the parent, including replies from created subagents.

## Decisions
- New dependency `ruzstd` (MIT, pure Rust, decode-only) for Codex archives; avoids a C toolchain on all OSes.
- Codex totals that include tokens never reported per turn (0.002% observed) are accepted and documented.

## Verification
- `cargo test --workspace`, `cargo clippy -D warnings`, web and desktop tests.
- `scripts/verify-ingest.py` on a fresh ingest: Claude and Antigravity exact on every token class; Codex exact except one documented session (0.3%).

## Addendum: API limits (ADR 0029)
- `gnomcode limits enable|disable|status|check <claude|antigravity>`; daemon polls opted-in providers every 5 min.
- Claude: OAuth usage endpoint with Claude Code's own token; never refreshed. On this machine the CLI token was expired (desktop-only sign-in), so a one-time `claude auth login` is needed.
- Antigravity: language server `GetUserStatus` via its loopback descriptor; the `agy` CLI endpoint returned 401 without a token in PTY and non-PTY tests, so CodexBar's CLI path was not adopted.
