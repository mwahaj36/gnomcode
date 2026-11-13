# ADR 0051: Worktree per session with diff review, merge, discard and pull request (backend)

- **Status:** Accepted
- **Date:** 2026-11-13
- **Owners:** Worker A (daemon). Fills in and amends the contract assumed by Worker B's ADR 0036 / PR #206, which is not on `main` yet.

## Context
Roadmap F1 lets a launch run in its own git worktree so an agent's work can be reviewed and merged, or thrown away, without touching the user's checkout. AGENTS.md requires Rust to own the logic, never to escalate permissions, to fail open and to keep Remote end-to-end encrypted.

## Decision
New `worktree_session.rs` (git and records) and `worktree_api.rs` (HTTP and Remote), registered with appended lines in `api.rs`, `lib.rs` and `remote.rs`.

1. **Where.** Checkouts live in `<GNOMCODE_HOME>/worktrees/checkouts/<session id>/<repo folder name>` (resolved through `gnomcode_platform::gnomcode_home`, never the system temp folder); records in `<GNOMCODE_HOME>/worktrees/records/<session id>.json`. No database migration.
2. **Launch.** `POST /api/v1/sessions/launch` and Remote `launch` accept `worktree` (bool) and `worktree_branch` (string, optional). Rust requires a git repository with at least one commit and an attached branch, validates the name with `git check-ref-format --branch`, refuses an existing branch, then runs `git worktree add -b <branch> <path> <base commit>`. The default branch is `gnomcode/<session id>`. The agent starts in the same subfolder the user chose, when it exists in the checkout. If the agent fails to start the worktree, branch and record are removed. Replies add `worktree`, `worktree_branch` and `worktree_path`; refusals are HTTP 422 `{ok:false,message}` or Remote `error`. Untracked and ignored files of the original checkout (for example `.env`, `node_modules`) are not copied; uncommitted edits in the original are not in the worktree.
3. **Session rows.** Local and Remote session lists add `worktree`, `worktree_branch` and `worktree_path` to rows that have a record.
4. **Review.** The existing `GET /api/v1/sessions/{id}/changes` and Remote `changes` (ADR 0037) diff a managed worktree against the commit it was cut from, so committed and uncommitted tracked work both show. Remote `session_changes`, the name the worktree UI sends, is an alias that replies with type `session_changes` and adds `branch`.
5. **Actions** (`POST /api/v1/sessions/{id}/worktree/{merge,discard,open-pr}`; Remote `worktree_merge`, `worktree_discard`, `worktree_open_pr` replying `worktree_action_ack`). Success is `{ok:true,message,url?,merged_commit?}`; failure is `{ok:false,message}` with 404 (no worktree), 409 (agent still inside, base branch not checked out, repository dirty, branch exists, conflicts), 422 (`gh` missing or signed out, no `origin`, bad request state) or 500. One action runs at a time.
   - **Merge** saves uncommitted work on the session branch (`git add -A` honours `.gitignore`, so `target/` is not committed), then merges into the base branch with `--no-ff` only when the original repository already has that branch checked out and no uncommitted tracked changes. Gnomcode never switches the user's branch, never forces, rebases or resets, and aborts a conflicting merge before reporting the files. It then removes the checkout (including `target/` and other build output), prunes git's records and deletes the merged branch with `-d`.
   - **Discard** removes the checkout and its build directories and deletes the session branch (`-D`); the base branch is untouched. The UI asks for confirmation first.
   - **Open PR** needs `gh` installed and `gh auth status` succeeding, and an `origin` remote; otherwise it returns a clear 422. It saves work, runs `git push -u origin <branch>` (never forced) and `gh pr create --fill --base <base> --head <branch>`, and returns the URL. The worktree stays for follow-up work; use Discard to clean up after the PR lands.
   - Merge and Discard refuse while a hosted agent is running inside the worktree; the user stops the session first. After either, the conversation's saved folder points back at the original repository so a later resume does not look for a deleted folder.
6. **Safety.** All paths and branches used by actions come from Gnomcode's records, never request text; record ids are restricted to `[A-Za-z0-9._-]`; git runs with `GIT_TERMINAL_PROMPT=0`, no window on Windows and a 120 second timeout; the diff base is accepted only if it is hexadecimal.

## Consequences
- A crash between creating the worktree and saving its record leaves an orphan folder under `worktrees/checkouts`; Discard cannot reach it. A janitor is future work.
- An agent started by hand in a worktree terminal outside Gnomcode is not detected as "in use".
