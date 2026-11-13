# ADR 0053: Continue here for imported sessions (backend)

- **Status:** Accepted
- **Date:** 2026-11-13
- **Owners:** Worker A (daemon). Amends the contract assumed by Worker B's UI in PR #209.

## Context
Imported provider sessions (started outside Gnomcode) are view-only. `POST /api/v1/sessions/{id}/continue` and Remote `resume_session` resume only conversations Gnomcode already manages, and answer `403`/`ImportedSession` for imported ones. PR #209 adds a "Continue here" action that calls `POST /api/v1/sessions/{id}/continue-here` and the E2EE `continue_here` request.

## Decision
New `crates/daemon/src/continue_here.rs`:
1. `plan()` (pure, no process started) reads `Database::get_session_focus_target(id)`, which prefers the session's own recorded `cwd`. It refuses when the session is unknown (404), the agent is not Codex, Claude Code, Gemini CLI or Antigravity (422), the recorded folder is missing or no longer a directory (422) or the id is not `<agent>:<native id>` with a native id accepted by `gnomcode_pty::resume_command` (422).
2. `continue_here()` takes the conversation control lock, resumes through `gnomcode_pty::resume_session` in that folder, then `reattach_conversation(id, host, Some(id))` and `save_conversation_launch`, so the conversation id and history stay stable and the session becomes managed (`source: "managed"`, `can_prompt` rules unchanged). A session that already has a launch record goes through `ensure_conversation_running`.
3. Responses: HTTP `200 {ok:true,session_id,new_session_id,host_session_id}`; failures `{ok:false,message}` with 404, 422, 500 or 502. Remote replies `continue_here_ack` with the same fields, or `{type:"error",message}`. `new_session_id` equals `session_id`, which the UI already accepts (`new_session_id || session_id`).

## Consequences
- The command is only `resume <id>` or `--resume <id>`; no permission-skipping flag exists on this path (tested), no prompt is injected and no chat is persisted by this action.
- The UI's fallback (open the launcher prefilled) is still used when the daemon refuses; the refusal message is user-readable and may be shown as is.
- Antigravity reuses the plan-mode native resume from G6 (`--conversation <id> --mode plan`), so a resumed Antigravity conversation never starts in Auto.
