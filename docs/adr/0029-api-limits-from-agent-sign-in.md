# ADR 0029: Exact limits from provider APIs using the agent's own sign-in

## Status

Accepted (2026-11-12, owner decision). Amended by [0030](./0030-claude-signin-refresh.md): the official CLI, not Gnomcode, may refresh the Claude sign-in.

## Context

Local logs give exact limits only for Codex. Claude Code and Antigravity users, including Claude desktop app users, saw no limits. Comparable open-source tools (CodexBar) read the agent's local sign-in and ask the provider for usage. SPEC §3.8 previously forbade reading provider credential files.

## Decision

Add an opt-in source per provider (`gnomcode limits enable <claude|antigravity>`), polled every 5 minutes by the daemon:

- **Claude:** read `claudeAiOauth.accessToken` from `CLAUDE_CONFIG_DIR/.credentials.json` at fetch time and call `GET https://api.anthropic.com/api/oauth/usage` (`anthropic-beta: oauth-2025-04-20`). Windows `five_hour`, `seven_day`, `seven_day_opus`, `seven_day_sonnet` become `exact` quota rows. Expired tokens are skipped; Gnomcode never refreshes or rewrites the token because Claude Code rotates refresh tokens.
- **Antigravity:** read the running language server's loopback descriptor (`GEMINI_HOME/antigravity/daemon/hub_*.json`) and call its `GetUserStatus` on `127.0.0.1` with the descriptor's CSRF token. Per-model `remainingFraction`/`resetTime` become `api` quota rows. Nothing leaves the machine. Works only while an Antigravity language server is running.

Tokens are never persisted, logged, synced or sent anywhere except their own provider (Claude) or loopback (Antigravity). Status line and log sources remain.

## Consequences

Claude limits work for every Claude surface, including chats on claude.ai, because the pool is account-wide. These are undocumented endpoints and can change without notice; failures leave limits unknown rather than wrong. The Antigravity CLI (`agy`, language server 2.31) rejected unauthenticated status calls in testing, so CodexBar's CLI-launch path is not adopted. This is opt-in vendor API traffic under invariant 4.
