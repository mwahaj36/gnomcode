# 2026-11-13 - Phone approvals for Claude Code

## Goals

Gap G1 of `docs/research/2026-11-13-dialpad-cao-comparison.md`: answer a Claude Code tool-permission prompt from the phone (Remote) or the console, fail open, never auto-allow, with step 0 verification of the hooks schema before design.

## Outcomes

- ADR 0032, SPEC 3.10 / 2.1 / 3.15 / section 9, threat model entry.
- `core::approval` (pure): request/decision state machine (single use, idempotent repeat, conflict, expiry, cancel, cap of 16), tool-call summarizer (identifying field only, redacted, 120 characters), Claude hook output builder, transcript-answer detector.
- `daemon::approvals`: in-memory registry, `GET/POST /api/v1/approvals`, `/{id}/wait`, `/{id}/decision`, `/notify`; hold only while a remembered phone is reachable or the console is polling; disconnect of a waiting hook cancels the request; std-socket hook client (50 ms connect, 500 ms short reply).
- Remote request types `approvals` and `approval` (E2EE only).
- CLI: `gnomcode hook` handles `PermissionRequest` and `Notification`; `gnomcode integrate claude` adds both with backup, chaining and `--remove`, `--no-approvals` to skip.
- UI: approval card (Deny, Approve once) in the local console above every view and in the Remote chat; phone targets 44px, neutral controls.

## Step 0 evidence (Claude Code 2.1.322, temp `CLAUDE_CONFIG_DIR`, mock model endpoint, no permission-skipping flags)

- Docs: payload and `hookSpecificOutput.decision` schema, `timeout` in seconds, exit 2 ignored, `permission_prompt` notification after about six seconds.
- `claude -p`: allow ran the tool, deny blocked it, crash / non-JSON / timeout produced no decision and the normal flow continued.
- Interactive, through the Gnomcode PTY host: no decision shows the native dialog; while the hook waits the dialog stays visible and answerable; a hook `allow` closes it ("Allowed by PermissionRequest hook"); answering the dialog natively does not cancel the hook.
- Not verified: macOS and Linux, subagent and MCP prompts, `idle_prompt` runtime payload.

## Problems and root causes

- A card stayed pending after the dialog was answered at the terminal: Claude does not cancel a running hook. Root cause fix: the hook follows the transcript path from its payload and exits when a tool result appears; exit closes the socket and the daemon cancels the request.
- First end-to-end run showed no pending card: the Node test driver blocked its own event loop with a synchronous child process. Driver fixed; product code was fine.
- The first interactive run started in auto mode (inherited environment and defaults), so no prompt appeared. Re-run with `permissions.defaultMode: default` in the temp settings and the parent Claude variables removed.
- Link step failed with Permission denied while a test-launched PTY host still held the debug binary; the host process tree was ended.

## Decisions

- `approval_wait_delay_secs` stays the awaiting-approval detection delay; the hold is its own constant (90 s) with a 100 s hook timeout.
- Two requests from the hook (short create, long wait) so a slow daemon costs at most 500 ms.
- Codex, Antigravity and Gemini CLI remain detection only.
- Only `permission_prompt` notifications are forwarded; `idle_prompt` is not installed (completion notices already exist).

## Verification

- `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`; web `npm ci`, `npm test`, `npm run typecheck`, `npm run build`.
- Real `claude -p` plus real daemon and hook: allow, deny and no-listener cases; interactive hold, console allow and native-answer-first cases.

## Open items

- Physical-phone check of the card and the push notice; macOS and Linux runs of the hook.
- Screen-verified approvals for the other three agents (research plan items 5 and 11).
- An approval audit line in the chat history (kept out: invariant 1 design not done).
