# 2026-11-12 - Claude sign-in refresh through the official CLI

## Goals

After the stale-limits fix (PR 157), keep the Claude OAuth sign-in fresh without the user opening a terminal, which the desktop app cannot do for them. The owner approved running the official CLI in the background.

## Finding a zero-usage refresh command

`claude auth --help` (Claude Code 2.1.322) lists `login`, `logout` and `status`. On this machine the access token had been expired for over an hour (`expiresAt` 16:36:53 local, file mtime 08:36:53). Before and after running `claude auth status`, comparing only file mtime and `expiresAt`:

- before: mtime 08:36:53, expires 16:36:53
- after: mtime 18:02:55, expires 02:02:55 the next day (eight hours ahead)
- a second run with a valid token: file untouched (mtime unchanged)
- no new session transcript was written under `~/.claude/projects`, and no model prompt was sent
- `gnomcode limits check claude` immediately returned live five-hour and weekly readings

So `claude auth status` refreshes through the CLI's own token handling and sends no model request. No `-p` prompt is used. No token value was printed at any point.

## Outcome

- `vendor_limits::Refresher`: when Claude limits are enabled and refresh is not disabled, a token that is expired or expires within 30 minutes triggers `claude auth status` (resolved by `platform::executables::background_agent_command`, so PATH, native install and the desktop-bundled copy are covered; no shell; no window on Windows; output discarded; 30 second kill; at most one attempt per 15 minutes whether it succeeds or not). The poll in the same pass reads the refreshed file. Failures only log a fixed reason and leave `source_issue` as is.
- Setting: `gnomcode limits enable claude` turns refresh on; `--no-refresh` turns it off (`claude_refresh` in `vendor-limits.json`); `limits status claude` prints it.
- ADR 0030 (amends 0029), SPEC §3.9 and §9 row.

## Problems and decisions

- Whether the CLI refreshes when the token is merely close to expiry (inside our 30 minute window but not yet expired) was not observable here; if it does not, the attempt is rate-limited and retried until the token actually expires, so readings recover within one cycle after expiry.
- Tests use a fake CLI script in a temp dir and a temp consent file; the real home is never read.

## Verification

Tests (red first, since the types did not exist): refresh runs only with `auth status`; expiry window (31 minutes no, 29 minutes yes); 15 minute rate limit; hung CLI killed at the timeout and counted as an attempt; missing CLI or sign-in is not an error; consent defaults (no file, limits off, limits on, `--no-refresh`); setting persistence. Full gate results are in the pull request.
