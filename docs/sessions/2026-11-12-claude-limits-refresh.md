# 2026-11-12 - Stale Claude Code limits

## Goals

Find why the console showed Claude Code `five hour: Reset / Awaiting next reading` and `weekly: 98% left, Resets Thu 8:00 AM` while Claude was in heavy use, and make the app stop presenting that old reading as current.

## Evidence (read-only on the real home, `~/.spawnt`)

- `quota_snapshot` held exactly two Claude rows, both `exact`, both observed `2026-11-12T03:37:17Z` (08:37 local, UTC+5): `five_hour` 28% used, reset 07:40Z; `weekly` 2% used, reset 2026-11-19T03:00Z (Thu 8:00 local). That is the whole of the console's 98% left.
- `~/.claude/.credentials.json` was rewritten at 08:36:53 local (24 seconds before that row) with `expiresAt` exactly eight hours later, 16:36:53 local (11:36Z). It had not been rewritten since, so the access token had been expired for over an hour. `gnomcode limits check claude` reported `Claude sign-in expired`.
- Antigravity rows, polled by the same task, show five-minute observations until 03:37:17Z, then nothing until 12:42:59Z. The daemon was down between those times (`daemon.json` started 12:42:55Z), and its restart poll found the token already expired. Earlier polls that morning also found it expired, which is why the first Claude row only appeared after the 08:36 refresh.
- The `gnomcode statusline` feed is configured in `settings.json` but `claude-limits.json` has never existed. The running Claude Code processes are the Claude desktop app (packaged `Claude_*` app, `claude-code` 2.1.321 in `stream-json` mode), which does not render a status line. Running `gnomcode statusline` against a fake payload in a temp home wrote the file correctly, so the command works; nothing calls it.
- Nothing in the daemon surfaced any of this: `vendor_limits::spawn` only emitted `tracing::warn!` (no log sink is configured for the packaged daemon), and `/api/v1/quota` kept returning the last rows.

## Root cause

1. The only live Claude source is the OAuth usage poll, which reads the access token from `~/.claude/.credentials.json`. By design (ADR 0029) Gnomcode never refreshes it, and the Claude desktop app does not keep that file fresh, so after eight hours of no CLI use every poll fails.
2. The failure was invisible. The weekly row's reset is days away, so the console rendered the nine-hour-old 98% as a live value; the five-hour row showed `Reset` only because its window had elapsed.

Not a poll-loop bug: the loop ticks every five minutes, re-reads the file each time, and recovers by itself once the file holds a live token.

## Fix

- `vendor_limits` records each provider's last poll failure (fixed reason text only, never a token or response body) and clears it on the next good reading or when the provider is disabled. The expired message now says what works: run `claude` in a terminal.
- `GET /api/v1/quota` adds an optional `source_issue` to pools of an affected provider (`quota_views_with_issues`; `quota_views` itself only gets `source_issue: None`).
- The console shows `source_issue` plus the reading age (`seen 9h ago`) in place of the reset text.
- SPEC §3.9 describes the behaviour.

## Verification

- New tests were red first (missing functions), then green: `a_failed_poll_is_reported_until_the_next_good_reading`, `issues_are_looked_up_by_quota_provider_id`, `an_expired_sign_in_names_the_fix_and_a_live_one_is_used`, `quota_pools_say_why_their_live_source_is_stale`.
- Rendered the console quota markup for a nine-hour-old weekly pool with an issue: `98% left` with the sign-in message and `seen 9h ago`.
- `scripts/check-console-presentation.mjs` already fails at an older assertion (line 20, expects `Last received reading`) on `main`; unrelated and left alone.

## User action

Run `claude` once in a terminal (any prompt, or `/login`) to refresh `~/.claude/.credentials.json`; the next poll within five minutes restores five-hour and weekly readings. Because the desktop app does not refresh that file, expect the notice again roughly eight hours later unless the CLI is used.

## Decisions

No token refresh was added: Claude Code rotates refresh tokens and a second refresher could sign it out (ADR 0029). Spawning the official CLI to refresh on demand was considered and left for a separate decision.
