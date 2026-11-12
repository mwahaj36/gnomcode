# 0030 - Refresh the Claude sign-in through the official CLI

- **Status:** Accepted (2026-11-12, owner decision). Amends [0029](./0029-api-limits-from-agent-sign-in.md).
- **Date:** 2026-11-12

## Context

ADR 0029 reads `claudeAiOauth.accessToken` from `~/.claude/.credentials.json` and said Gnomcode never refreshes it, because Claude Code rotates refresh tokens and a second refresher could sign it out. The token lives eight hours. The Claude desktop app signs in outside that file and never rewrites it, so users of the desktop app saw Claude limits stop updating once the token expired (a nine-hour-old reading shown as current).

## Decision

When Claude limits are enabled, the daemon asks the official `claude` CLI to refresh the sign-in. Gnomcode still never touches the refresh token, never calls the token endpoint and never rewrites the file; the CLI does that itself.

- Command: `claude auth status`. Verified on Claude Code 2.1.322: with an expired access token it rewrote `.credentials.json` (new expiry eight hours ahead), with a valid token it left the file untouched, and it created no session transcript. It reports login state and sends no model prompt, so it consumes no usage.
- Trigger: the token is expired or expires within 30 minutes. At most one attempt per 15 minutes, regardless of outcome. The poll that follows in the same five-minute pass reads the refreshed file.
- Execution: the CLI is resolved by `platform::executables` (PATH, native installer, desktop-bundled copy), started without a shell and with no window on Windows, stdin/stdout/stderr discarded (output is never read or logged), killed after 30 seconds. No permission-skipping flags are ever passed.
- Fail open: any failure (missing CLI, non-zero exit, timeout) is logged as a fixed reason and leaves the existing `source_issue` visible.
- Setting: on by default when `gnomcode limits enable claude` is run; `gnomcode limits enable claude --no-refresh` turns it off (`claude_refresh: false` in `vendor-limits.json`). Nothing runs unless Claude limits are enabled.

## Consequences

Claude limits keep updating for desktop-app users without a terminal. Gnomcode now starts a user-installed official CLI in the background on a schedule; the user approved this explicitly. If a future CLI version makes `auth status` stop refreshing, the stale-reading notice still appears. The CLI and any running Claude process could refresh at the same moment; the CLI owns that file's rotation, so this is the same race two Claude windows already have.

## Alternatives considered

- Call the OAuth token endpoint from the daemon: breaks refresh-token rotation and stores credentials. Rejected (the original reason in 0029).
- Run a model prompt (`claude -p`) to force a refresh: consumes usage. Not needed.
- Ask the user to run `claude` manually: the previous behavior; stale readings every eight hours.
