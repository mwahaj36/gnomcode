# ADR 0037: Read-only per-session changes and diff over E2EE Remote

Date: 2026-11-13
Status: Accepted

## Context
Local `GET /api/v1/repos/diff` already existed for inspecting git status and diff by repository path, but the phone companion had no E2EE Remote `changes` command and needed session-scoped resolution without passing raw host paths from Remote.

## Decision
Add `crates/daemon/src/session_changes.rs` exposing `GET /api/v1/sessions/{id}/changes` and E2EE Remote `{ "type": "changes", "session_id": "..." }`:
- Resolves `session_id` to its workspace directory on the daemon (hosted PTY `repo_path` or `Database::get_session_focus_target`).
- Runs read-only `git --no-pager -c core.quotepath=off diff --no-ext-diff HEAD` and `status --porcelain=v1 -z` with `GIT_TERMINAL_PROMPT=0`, `GIT_OPTIONAL_LOCKS=0`, `GIT_LITERAL_PATHSPECS=1`, and `LC_ALL=C`.
- Enforces the 1 MiB output cap and 10-second timeout per command, decodes output lossily with `lossy: bool`, and exposes no revert/stage/commit mutations.

## Consequences
- Phone companion (#190) can inspect session git status (including untracked files) and unified diffs over E2EE Remote with zero index-lock contention against running agents.
