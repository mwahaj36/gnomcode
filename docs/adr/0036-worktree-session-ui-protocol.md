# ADR 0036: Worktree-Per-Session UI Contract (Local Console and Encrypted Remote)

- **Status:** Accepted. UI contract only; the daemon behavior is specified in [ADR 0051](./0051-worktree-per-session.md), which is authoritative where the two differ.
- **Date:** 2026-11-13
- **Owners:** Worker B (UI), Worker A (daemon/git worktree backend)

## Context
Gnomcode roadmap Section F ("Worktree per session with diff review and merge") requires letting users isolate an agent launch inside a temporary Git worktree, inspect the resulting diff read-only, and choose `Merge`, `Discard`, or `Open PR` without putting any Git logic or permission escalation inside TypeScript/React.

Worker A implemented the backend in ADR 0051 (merged before this UI). This ADR records the shapes the UI consumes.

## Decision
Define a minimal HTTP + encrypted Remote WebSocket contract owned by the Rust daemon:

1. **Launch request:**
   - Local HTTP `POST /api/v1/sessions/launch` and Remote WS `{ "type": "launch", ... }` accept optional fields:
     - `worktree?: boolean`
     - `worktree_branch?: string`
   - Daemons without worktree support ignore unknown fields or return an explicit error without silently altering repository state.
2. **Session metadata:**
   - `LocalSessionSummary` and Remote `Session` objects MAY include optional fields:
     - `worktree?: boolean`
     - `worktree_branch?: string`
     - `worktree_path?: string`
3. **Read-only diff review:**
   - Local HTTP reuses `GET /api/v1/sessions/{id}/changes` and Remote WS reuses `{ "type": "session_changes", "session_id": "<id>" }` returning `{ repo?, branch?, status?, diff?, lossy? }`.
4. **Worktree lifecycle actions (always user-confirmed in UI; all Git execution stays in Rust):**
   - `POST /api/v1/sessions/{id}/worktree/merge` / Remote WS `{ "type": "worktree_merge", "session_id": "<id>" }`
   - `POST /api/v1/sessions/{id}/worktree/discard` / Remote WS `{ "type": "worktree_discard", "session_id": "<id>" }`
   - `POST /api/v1/sessions/{id}/worktree/open-pr` / Remote WS `{ "type": "worktree_open_pr", "session_id": "<id>" }`
   - Local actions return `{ ok: true, message, url?, merged_commit? }` on success and a non-2xx `{ ok: false, message }` on refusal (404 no worktree, 409 agent still inside, dirty base, conflicts, 422 `gh` missing). Remote replies `worktree_action_ack` or an `error` with the same message. The UI shows the daemon's message verbatim and never retries a refused action.

## Consequences
- Zero Git parsing, branch creation, merge execution, or PR CLI calls occur in browser JavaScript/TypeScript.
- Existing non-worktree launches continue to work identically when `worktree` is unchecked.
