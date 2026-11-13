# ADR 0057: Wire Codex and Antigravity protocol approvals into launches (opt-in)

- Status: Accepted
- Date: 2026-11-13

## Context

ADR 0045 (Codex `app-server`) and ADR 0047 (`agy acp`) added approval translators that no launch path used. A terminal (PTY) Codex or Antigravity session cannot route a permission prompt to the phone or console, so those agents kept detection and notification only.

## Decision

1. A per-agent setting `protocol_approvals` (`codex`, `antigravity`), default OFF, stored in `~/.gnomcode/protocol-approvals.json` and changed with `gnomcode protocol-approvals enable|disable|status <agent>`. It stays off until the protocols are verified on more machines.
2. When enabled, a new launch (not a resume) starts a protocol host instead of a PTY host. The host is the same independent `pty-host` process with the same authenticated loopback API (`/health`, `/stream`, `/input`, `/stop`), so the console, Remote and `gnomcode attach` need no change. It spawns the official `codex app-server` or `agy acp` over stdio, with no permission-skipping flags and without overriding the user's approval policy, sandbox or permission mode.
3. The host renders agent text, commands and approval state as plain text in the terminal stream, and turns the bracketed-paste prompt plus Enter that `send_prompt` already sends into `turn/start` (Codex) or `session/prompt` (Antigravity). `/model <name>` sets the model for later turns.
4. Approval requests go through ADR 0032 (`POST /api/v1/approvals`, `GET /api/v1/approvals/{id}/wait`), so the existing console and Remote approval UI shows them. Rules: never auto-approve; Antigravity allows only `allow_once`; no decision, timeout, no listener or a missing daemon answers cancel (`cancel`/`abort`/empty permissions, `cancelled`).
5. PTY mode remains the default and is unchanged.
6. The translators read the wait reply's `decision` key, but the daemon replies `outcome`; every request therefore resolved to no decision. They now read `outcome`.

## Consequences

- A turn is cancelled, not left waiting, when nobody is watching the console or phone. Codex ends the turn on cancel.
- Protocol sessions have no full-screen TUI (no slash-command UI, no colours). Resumed sessions keep the PTY.
- Gnomcode still reads the agent's own history files for chat continuity; the host keeps nothing on disk beyond the session record.
