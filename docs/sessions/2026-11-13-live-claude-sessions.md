# 2026-11-13 - Live Claude desktop sessions and subagents

## Goals
Fix Home showing "Recent sessions 0 / No ongoing sessions" while a Claude Code session (desktop app, `claude-desktop` entrypoint, several background subagents) was running, and show subagents nested under their parent.

## Root causes (evidence)
- Installed v0.9.4 DB copy: the live session row `claude-code:3ded3e8c-...` had `parent_session_id` equal to its own id, so every session list (`parent_session_id IS NULL`) dropped it. Its subagent rows were correct children.
- Subagent transcripts (`<session-id>/subagents/agent-<id>.jsonl`) contain `api-request`, `api-request-blob` and `api-request-shape` lines with `agentId` and the parent's `sessionId` but no `isSidechain`. The adapter keyed identity on `isSidechain:true` and built one session per chunk from the first line, so a live-tail chunk starting on such a line produced a session with the parent's id, the subagent's timestamps and `Idle` state, and a later sidechain line in the same chunk set its parent to itself.
- The same lines carry no `entrypoint`, so the surface was reset to CLI; the upsert never updated an existing surface.
- Reproduced on main: fresh daemon on a copy of the transcripts, then appending one `api-request` plus one sidechain `assistant` line to a subagent file dropped the parent (`total_count` 2 to 1, `ongoing_count` 0).
- Subagents were hidden by design (`parent_session_id IS NULL` in the sessions list, `include_subagents=false` default in conversations) with no nested view.
- A parent waiting on background subagents has a quiet own log; after 15 minutes it left the Running group although subagents worked.
- Unrelated, found while verifying: `console-presentation.js` imports `./provider-logos.js` (added in #176) but the daemon had no route for it, so the local console failed to load Home and Sessions on main.

## Fixes
- Adapter: subagent identity from the file path or `agentId` unless `isSidechain:false`; multi-session chunks; session-less lines skipped; never self-parent; subagent title only on linked sessions; surface only from `entrypoint`/`clientPlatform`.
- Store: surface upsert keeps a non-CLI surface; sessions list nests `subagents`, `subagent_count`, `subagents_ongoing`; parents with live subagents are ongoing in SQL filters, counts and presence.
- Daemon: conversations rows nest subagents and the `active`/`ongoing` filters honor them; `/provider-logos.js` route and test.
- UI: shared `renderSubagents` for Home rows and the Workspace list (count badge, expandable, open state kept across refreshes).
- Fixtures `fixtures/claude-code/v2.1.321/desktop-{main,subagent}.jsonl` (anonymizer now preserves `agentId` structure and `clientPlatform`).

## Verification
Cargo fmt, clippy `-D warnings`, `cargo test --workspace`; web `npm ci`, test, typecheck, build. Live check on a daemon built from this branch against a copy of the real transcripts and a copy of the installed DB (temp `GNOMCODE_HOME`, port 41990/41991): the live session is listed, Running, with 26 nested subagents (3 active).

## Follow-up
Open PRs #198/#199 (React Home/Workspace) must render the new `subagents` fields.
