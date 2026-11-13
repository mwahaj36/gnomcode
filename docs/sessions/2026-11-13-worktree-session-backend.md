# 2026-11-13 - Worktree per session backend (F1)

## Goals
- Build the daemon side of roadmap F1 that Worker B's UI (PR #206, ADR 0036) calls: worktree launch option, per-session review, Merge, Discard and Open PR over local HTTP and E2EE Remote, with cleanup of the worktree and its build directories.

## Outcomes
- `crates/daemon/src/worktree_session.rs` (git operations and records), `worktree_api.rs` (HTTP and Remote handlers, launch helpers, row decoration), and small hooks: launch fields in `api.rs` and `remote.rs`, session-row fields, and a worktree-aware diff base in `session_changes.rs` (ADR 0037, already merged).
- Checkouts live under `GNOMCODE_HOME/worktrees`, not the system temp folder. Merge never switches or forces the user's branch; Open PR is gated on `gh` being installed and signed in.
- ADR 0051, SPEC 3.14 paragraph and ADR row, roadmap F1 ticked (backend).

## Problems encountered
- `git add -A` committed an untracked `target/` in the first merge test because the fixture repo had no `.gitignore`. Real repositories ignore build output, so the fixture now does; the merge deliberately honours `.gitignore` rather than guessing which directories are build output.
- Git on Windows writes CRLF into checkouts, so tests compare normalised text.
- The remote name for the review diff differs between UI PRs: #190 and the merged #207 use `changes`, #206 uses `session_changes`. Both are served (alias).
- The shell tool mangled backslashes and long heredocs; edits were made with the editor tool and script files.

## Decisions
- Reuse `/changes` instead of adding a second diff route (a duplicate route would panic at router build); it diffs a managed worktree against its base commit.
- Records are JSON files, not a schema change.
- Merge auto-saves uncommitted work on the session branch because the UI has no commit step.
- Refuse Merge and Discard while a hosted agent is inside the worktree rather than stopping it.

## Verification
- 19 new tests across `worktree_session` (create, subfolder start, bad input, merge, no-op merge, unsafe states, conflict abort, discard, diff base, rollback, Open PR with fake `gh` and a bare remote) and `worktree_api` (busy refusal, 404, merge/discard outcomes, Remote shapes, HTTP auth, launch refusal over HTTP and Remote).
- Full gate: see PR.
