# Subagent identities, session token display, and auto-compacting IDE relay bypass

Date: 2026-11-04

## Goal and root cause

Three related issues were reported in ongoing session monitoring and context management:

1. **Subagents clobbering parent sessions and model names:** In Codex rollouts, subagent threads contained `session_id` equal to the parent session's UUID, with their own unique thread ID in `id`. Using `session_id` directly meant subagents overwrote the parent session record in SQLite, leading to `gpt-6-luna` clobbering `gpt-6-astra` in session cards and showing unknown titles.
2. **Missing token counts in ongoing sessions:** Ongoing session cards did not display the total recorded token count for the individual session, despite `total_tokens` being computed in `list_sessions_filtered`.
3. **Spurious context relay handoffs on auto-compacting IDEs:** Agents operating inside IDE surfaces or auto-compacting environments (Antigravity, Cursor, Cline, and any session with `surface: ide`) automatically compact and summarize context in-place. Context Relay treated them identically to standard terminal CLIs, triggering desktop alerts and automated rotation handoffs that disrupted active IDE turns.
4. **Subagent visibility across all adapters:** Subagent roles and parent session relationships were not recognized or displayed for Codex or other supported agents.

## Fix and decision

1. **Adapter-wide subagent identity and parent linking:**
   - `crates/adapters/src/codex.rs`: Uses the unique rollout thread ID / path UUID before falling back to `session_id`. Extracts `parent_thread_id` and constructs `subagent_title` from agent nicknames/roles (e.g. `Goodall (control) · Subagent`).
   - `crates/adapters/src/antigravity.rs`: Extracts parent sender UUID from system message envelopes (`sender=<parent_uuid>`) and sets subagent title and parent ID.
   - `crates/adapters/src/claude.rs`: Captures `parentSessionId` / `parentId` and subagent mode/title.
   - `crates/adapters/src/cline.rs`: Captures `parentTaskId` / `parentSessionId` and mode name.
   - `crates/adapters/src/cursor.rs`: Captures `parentSessionId` and subagent name.
   - `crates/adapters/src/gemini.rs`: Captures `parentSessionId` and subagent title.
2. **Store query enhancement:**
   - `crates/store/src/lib.rs`: Added `parent_session_id: Option<String>` to `SessionListItem` and selected `s.parent_session_id` in `list_sessions_filtered`.
3. **Console presentation updates:**
   - `web/lib/console-presentation.js`: In `renderOngoing`, displayed recorded token count (`${escape(tokens(session.total_tokens))} tokens`), subagent label (`· Subagent`), subagent title, and added `Recorded tokens` and `Parent session` to the drawer `<dl>`. Also updated `renderSessions` and `renderSessionList`.
   - `scripts/check-console-presentation.mjs`: Added assertions verifying token count rendering and subagent badge formatting in `renderOngoing`.
4. **Context Relay bypass for auto-compacting agents:**
   - `crates/daemon/src/pro.rs`: Updated `run_context_relay` to check if a session is an auto-compacting environment (`s.surface == "ide" || matches!(s.agent.as_str(), "antigravity" | "cursor" | "cline")`). If so, Context Relay skips reserve alerts and automated rotation, deferring to the IDE's native context management.
5. **Specification synchronization:**
   - Updated `docs/SPEC.md` §3.5 and §3.14 to document subagent session identity retention and Context Relay bypass for auto-compacting environments.

## Verification

- **Adapters unit tests:** `tests::codex_subagent_rollout_preserves_unique_id_and_parent_and_title` verifies parent linking, independent session IDs, model preservation, and subagent title extraction.
- **Daemon unit tests:** `pro::tests::context_relay_skips_auto_compacting_ides_and_agents` verifies that IDE-surfaced and auto-compacting agent sessions do not trigger context relay saturation notices or rotations.
- **Workspace tests:** `cargo test --workspace` passed across all 11 crates.
- **Linter and formatter:** `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` passed with zero errors or warnings.
- **Presentation checks:** `node scripts/check-console-presentation.mjs` passed.

## Acceptance limits

Pure Rust logic maintained in core/adapters/store. Fail-open and read-only behavior preserved. No emojis used anywhere in code, markdown, tests, or UI. Real native model activation and packaged installation checks remain governed by the launch checklist.
