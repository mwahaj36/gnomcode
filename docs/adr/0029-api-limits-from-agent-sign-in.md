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

## G7 addendum: verified native usage fallback (2026-11-13)

The existing Antigravity opt-in also authorizes the official authenticated CLI's own no-inference `/usage` metadata command when the running language-server source is unavailable or empty. This is not the unauthenticated CLI-launch status path rejected above. Prefer a good server result without starting a CLI. The background poll still checks the existing enable flag; an explicit CLI limits check remains an owner-requested metadata operation. No new setting or transport is added.

Verified with installed agy 1.5.7 in a disposable Windows workspace: `agy --output-format text -p=/usage` exited successfully within eight seconds and emitted five actual-tab-separated rows. Each row is group, window label, remaining percentage with `%`, and RFC3339 reset. Native diagnostic logs showed retrieveUserQuotaSummary and no cascade/generation markers; a separate controlled marker-generation turn logged cascade/streamGenerateContent. Raw logs, account identifiers and credentials are not checked in. Anonymized metadata fixtures preserve the observed quota fields.

Run a hidden shell-free metadata probe with closed stdin in a fresh temporary workspace, sharing one eight-second deadline across version and usage commands. Require exact verified `--version` output 1.5.7 before sending `/usage`, so unverified versions cannot turn a presumed slash command into an inference prompt. Reject other versions with a fixed issue; their existing running-server source remains available. Bound each stdout to 64 KiB, discard stderr and terminate the owned probe on timeout or oversize. Never persist or log raw output. The official CLI may handle its own sign-in; Gnomcode adds no credential read or config modification.

Pure Rust parsing validates the percentage range/finite value, known five-hour/weekly window labels, bounded group identity and RFC3339 reset. A model-specific `<group> Weekly Limit` label is supported from the real fixture. Preserve native pool names so server and CLI readings share canonical identities. Convert percent remaining to percent used with `api` trust and the completion observation time; unknown rows/reset/percentage are skipped, duplicates ignored, empty results unavailable. Failures keep prior readings and surface a fixed source issue. `source_label: Native limits` honestly covers both sources.

No model request was observed in the verified command's native trace. Independent wire-level verification, other versions and macOS/Linux native acceptance remain open; no broader availability claim is implied. The version gate is deliberate until new versions receive the same verification.
