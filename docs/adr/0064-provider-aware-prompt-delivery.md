# ADR 0064: Provider-aware prompt delivery and safe turn-boundary queuing

- **Status:** Accepted
- **Date:** 2026-11-14
- **Issues:** #303, #304

## Context

Gnomcode launches and observes three distinct official agent CLIs (`claude` 2.1.323, `codex-cli` 0.166.0, `agy` 1.5.8). Users expect the Workspace composer to remain available for drafting at all times—including while an agent is mid-turn or awaiting approval—without silently injecting keystrokes into busy terminals, approval prompts, or unknown states, and without silently promoting observed external sessions into Gnomcode-managed sessions.

Verified runtime capabilities:
- **Claude Code (`claude`):** Interactive PTY sessions accept bracketed-paste + Enter only when the rendered `vt100` screen confirms the composer prompt is visible and no trust/login/approval dialog is open (`claude_ready`). Mid-turn injection is unsafe.
- **OpenAI Codex (`codex`):** Interactive PTY sessions accept input only when `codex_prompt_ready` confirms the prompt box is active without an open dialog. Opt-in `codex app-server` protocol hosts (ADR 0045/0057) accept structured JSON-RPC turns at safe boundaries.
- **Google Antigravity (`agy`):** Interactive PTY sessions accept input only when idle at the prompt without an open dialog. Opt-in `agy acp` protocol hosts (ADR 0047/0057) accept structured ACP `session/prompt` turns.
- **Observed external sessions:** Have no Gnomcode PTY or protocol host. Direct prompt injection is impossible and must never silently start or adopt a managed host without explicit user action (`Continue here`, ADR 0053).

## Decision

1. **Rust owns capability, readiness, queuing, and delivery.** Session rows expose a Rust-computed `delivery_capability` (`mode`: `immediate` | `queue_until_ready` | `observed_read_only`, `can_queue`: bool, `reason`: string).
2. **Immediate vs. queued delivery:** `POST /api/v1/sessions/{id}/prompt` delivers immediately only when the session has a managed host and verified prompt readiness. When the managed session is busy, awaiting approval, or not yet ready and `queue_if_busy: true` is supplied with a client-generated idempotency `id`, Rust persists the redacted prompt in `scheduled_message` (`due_at = 0`, `status = 'pending'`) and delivers it on the next confirmed safe turn boundary via `automation::deliver_due`.
3. **Observed sessions stay read-only:** Observed external sessions return `observed_read_only` and reject prompt delivery/queuing unless the user explicitly invokes `Continue here`.
4. **Idempotency and non-replay:** Client-generated `id` keys prevent duplicate sends across double-clicks, reconnects, and crashes. Startup recovery (`recover_scheduled_messages`) marks interrupted `delivering` rows as `uncertain` and never auto-replays them.
5. **Queue management:** Pending queued prompts can be inspected (`GET /api/v1/sessions/{id}/queue`), edited (`PATCH /api/v1/sessions/{id}/queue/{msg_id}`), or cancelled (`DELETE /api/v1/sessions/{id}/queue/{msg_id}`) before delivery begins.

## Consequences

- Drafts and queued prompts survive navigation and reconnects without corrupting active terminal turns or approval dialogs.
- No speculative slash commands, `--yolo`, or permission-skipping flags are ever used.
