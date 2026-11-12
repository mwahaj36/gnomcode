# 0032 - Approve Claude permission prompts from the phone and console

- **Status:** Accepted
- **Date:** 2026-11-13

## Context

Gap G1 of `docs/research/2026-11-13-dialpad-cao-comparison.md`: a Remote phone could read a conversation and send prompts but could not answer a tool-permission prompt, so an agent that stopped to ask waited until the person returned to the laptop. `POST /api/v1/sessions/{id}/permissions` only wrote a chat row and never reached the agent. The report listed open facts to verify before design: the full `PermissionRequest` hook output schema, whether a waiting hook delays the native dialog, and fail-open behavior on timeout or error.

### Verified (step 0, Claude Code 2.1.322, temp `CLAUDE_CONFIG_DIR`, local mock model, no permission-skipping flags)

- Hooks reference (code.claude.com/docs/en/hooks): `PermissionRequest` input has `session_id`, `transcript_path`, `cwd`, `permission_mode`, `hook_event_name`, `tool_name`, `tool_input` and optional `permission_suggestions` (no `tool_use_id`). Output `hookSpecificOutput.hookEventName = "PermissionRequest"` with `decision.behavior` `allow` or `deny`; `deny` may carry `message` and `interrupt`; `allow` may carry `updatedInput` and `updatedPermissions`. Deny and ask rules are still evaluated after an allow. Exit code 2 is not honored for this event. `timeout` is in seconds. `Notification` input adds `message`, `title` and `notification_type`; matchers include `permission_prompt` and `idle_prompt`, and the permission notification fires only after the prompt has waited about six seconds.
- `claude -p` run against a mock endpoint with a real `PermissionRequest` hook: payload fields as documented; a hook printing `allow` ran the tool, `deny` blocked it (reported as a permission denial), a hook that exited 1, printed non-JSON, or slept past its 4 s `timeout` produced no decision and the normal flow continued (in `-p`, with nobody to ask, that flow is a denial).
- Interactive Claude through Gnomcode's own PTY host, `permission_mode` default: with no decision (daemon down or nobody watching) the native "Do you want to proceed?" dialog appears. While a hook is waiting the same dialog is shown on the terminal (Claude does not hide it), so the person at the laptop can still answer. When the hook returns `allow`, the dialog closes and Claude prints "Allowed by PermissionRequest hook".
- Answering the dialog at the terminal does not cancel a waiting hook. The hook therefore watches the transcript file named in its payload for a new `tool_result` line and stops waiting by itself.
- Exec-form handlers (`command` plus `args`) and `onFailure: "continue"` are accepted on Windows.

Not verified: behavior on macOS and Linux (same schema, not run), subagent and MCP-tool prompts, and the `idle_prompt` notification payload at runtime (documented only; not installed).

## Decision

Phone and console approvals for Claude Code only, built on `PermissionRequest`.

- **Install.** `gnomcode integrate claude` adds a `PermissionRequest` handler (all tools, `timeout` 100) and a `Notification` handler (matcher `permission_prompt`, `timeout` 2) beside the existing `PreToolUse` guardrail handler, all `gnomcode hook`. It backs up `settings.json` first, never edits other handlers (including ones sharing a matcher group with an older Gnomcode handler), and `--remove` deletes only Gnomcode's. `--no-approvals` skips the two new handlers. Nothing else modifies agent config (invariant 2).
- **Hook.** `gnomcode hook` reads the payload, reduces the call to the tool name and a redacted one-line summary in `core::approval` (command, short path, URL without query, pattern; never file contents or edit bodies; unknown tools carry the name only), then talks to the daemon over the loopback API with bearer auth. Connect budget is 50 ms; a short request that does not answer within 500 ms is abandoned. Only if the daemon answers `hold: true` does the hook issue a second, long request.
- **Daemon.** `crates/daemon/src/approvals.rs` keeps pending approvals in memory only (`core::approval::ApprovalBook`; never written to disk or the chat store). A request is held only while someone can answer: a remembered phone is reachable (desktop connected to the relay, device trusted) or the console polled in the last 15 seconds. Otherwise the daemon answers `hold: false` and the hook exits at once with no output, so Claude shows its own dialog without delay. A held request records a generic notice ("Approval needed...", no content) through the existing notification path.
- **Rules.** A request has a random 128-bit id, a hold of 90 seconds (`HOLD_SECS`, `approval_wait_delay_secs` stays what it is: the awaiting-approval detection delay) and a cap of 16 pending. A decision names the exact id; the first decision wins, an identical repeat succeeds without effect, a different later decision is a conflict (409), an unknown id is 404, and an allow after expiry or cancellation is gone (410). Deny works on any known request unless an allow was already delivered. Expiry, cancellation (the waiting hook disconnected or saw the transcript advance) and every error resolve to no decision, which prints nothing and leaves the native dialog. The hook never prints `allow` unless the daemon returned `allow` for its own request id, and the printed `allow` carries no `updatedInput` or `updatedPermissions`, so it covers one call and creates no standing rule. Claude's own deny and ask rules still apply (invariant 7).
- **Surfaces.** Console: `GET /api/v1/approvals`, `POST /api/v1/approvals/{id}/decision`, a card above the views with Deny and Approve once. Remote: new E2EE request types `approvals` (list) and `approval` (id, decision); the phone polls with its existing two-second refresh. Both record who decided in a log line without content. Decisions never travel through cloud push payloads.
- **Other agents: detection only in this change.** Codex, Antigravity and Gemini CLI keep today's behavior (awaiting-approval state and notice; the person approves on the laptop). Screen-verified keystrokes need the screen model from the research plan and are a later PR.
- **Detection-only fallback for Claude.** When nobody could be asked, the `Notification` `permission_prompt` handler posts a session id only; the daemon records the generic "Agent needs approval. Review it on your laptop." notice (skipped when a held request already exists for that session).

## Consequences

- With a phone paired, every Claude permission prompt in an integrated install is offered to the phone for up to 90 seconds while also staying answerable at the laptop. The hook is a long-running process in that window; Claude's `timeout` of 100 s outlives the 90 s hold.
- A prompt answered natively leaves a card for at most about half a second to one second (the hook polls the transcript twice a second); if Claude does not write a transcript (transcripts disabled) the card stays until expiry, and tapping it has no effect.
- Parallel prompts: a `tool_result` for another call can end the wait early. The effect is fail-open (the native dialog remains), never an unintended allow.
- Re-running `gnomcode integrate claude` on an existing install adds the two new handlers; `--no-approvals` opts out. The settings backup is overwritten by each run with the file as it was before that run.
- The old `POST /api/v1/sessions/{id}/permissions` chat-row endpoint is unchanged and unused by the UI; it is not an approval path.
- Threat model: a compromised paired phone or account can allow a tool call the agent asked about (it cannot run an arbitrary command; it can only answer a prompt Claude raised, and Claude's deny rules still win). This matches the existing account-wide Remote trust (SPEC 3.15). Summaries can still show a command a person must judge on a small screen; the 120-character cap truncates long commands, so the card says nothing beyond that.

## Alternatives considered

- `--permission-prompt-tool` / `--permission-prompts host` or the stream-json control protocol: needs a headless runtime for every conversation (research B12) and changes how Gnomcode hosts the official TUI. Not verified here and a much larger change.
- Screen scraping plus keystrokes for Claude: fragile across releases; a documented hook is exact.
- Hold every request regardless of audience: delays the native dialog for no benefit when nobody is watching. Rejected.
- A single long request from the hook: a slow daemon would hold the agent up to 90 seconds, violating fail open. The two-step protocol bounds the cost of an unresponsive daemon to 500 ms.
- Auto-allow on timeout or a "do not ask again" button: violates invariant 7.
