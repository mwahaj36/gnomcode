# ADR 0039: Read-only session artifacts alongside workspace changes

Date: 2026-11-13
Status: Accepted

## Context
The phone companion and local console need to inspect created or edited files—especially HTML, Markdown, and image outputs—alongside workspace change counts without exposing arbitrary host paths.

## Decision
Add `crates/daemon/src/session_artifacts.rs` exposing `GET /api/v1/sessions/{id}/artifacts` and E2EE Remote `{ "type": "artifacts", "session_id": "..." }`:
- Resolves `session_id` to its workspace directory on the daemon.
- Uses read-only `git status --porcelain=v1 -z` (with `GIT_OPTIONAL_LOCKS=0`, `GIT_TERMINAL_PROMPT=0`, `core.quotepath=off`, `core.fsmonitor=false`, `LC_ALL=C`) to collect created and modified files and `changes_count`.
- Confines all reads via `canonicalize()` within the workspace root, prioritizes images/HTML/Markdown, caps at 30 items, 256 KiB per text/HTML/Markdown artifact, and 512 KiB per image (`data:<mime>;base64,...`).

## Consequences
- Phone companion (#192) can preview HTML (sandboxed iframe on client), Markdown, and images over E2EE Remote alongside change counts.
