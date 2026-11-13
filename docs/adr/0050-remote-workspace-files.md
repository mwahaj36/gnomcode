# ADR 0050: Read-only workspace file listing and reading over local API and Remote

Date: 2026-11-13
Status: Accepted

## Context
The phone companion and local console need to browse directories and inspect source files inside a session's workspace without mutating files or exposing arbitrary host paths outside the workspace. Local `GET /api/v1/repos/file` already existed, but directory listing and E2EE Remote access were absent.

## Decision
Add `GET /api/v1/repos/files` (local, bearer-authenticated) and E2EE Remote request types `repo_files` and `repo_file` (`crates/daemon/src/workspace_files.rs`), strictly read-only and confined to the session's resolved workspace root:
- Remote requests resolve `session_id` to its workspace folder via the hosted PTY `repo_path` or `Database::get_session_focus_target`, never accepting an arbitrary folder path from Remote.
- Relative paths reject `..`, root/prefix components, and control characters.
- Both workspace root and target paths are canonicalized (`canonicalize()`), and any target or directory entry whose canonical path does not start with the canonical workspace root (e.g., symlink or junction escape) is rejected or skipped.
- Directory listings skip `.git`, sort directories before files, and cap at 200 entries (`truncated: true`).
- File reads enforce a 512 KiB size limit and require valid UTF-8.

## Consequences
- Phone and local console can browse and read workspace files safely over E2EE Remote without exposing arbitrary filesystem paths or allowing writes.
- Binary or >512 KiB files return an explicit error rather than partial or corrupted content.
