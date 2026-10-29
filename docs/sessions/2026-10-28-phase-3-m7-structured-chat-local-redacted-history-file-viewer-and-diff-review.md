## 2026-10-28 — Phase 3: M7 structured chat, local redacted history, file viewer, and diff review

- Implemented ADR 0017: structured chat events, local redacted conversation history in SQLite, user-controlled retention and hard deletion, permission approval handling, and read-only file/diff inspection.
- Domain types (`crates/core`): Added `ChatRole` (`user`, `assistant`, `tool`, `system`) and `ChatEvent`. Added `redact_chat_content` in `spawnt_core::redact` to redact API keys, tokens, and sensitive environment variables from conversation text without length truncation.
- SQLite storage (`crates/store`): Added migration version 8 creating `chat_event` table and index. Added `insert_chat_event`, `get_session_chat` (ordered by turn index), `delete_session_chat` (permanent hard deletion), and `purge_chat_events_older_than` (retention). Verified with unit tests.
- Daemon endpoints (`crates/daemon`):
  - `GET /api/v1/sessions/{id}/chat`: paginated structured chat events.
  - `DELETE /api/v1/sessions/{id}/chat`: permanent hard deletion of session conversation history.
  - `POST /api/v1/sessions/{id}/permissions`: records user approve/deny decisions without permission escalation.
  - `GET /api/v1/repos/file`: read-only file viewer strictly contained inside repository root (rejects `..`, root dir, and symlink escapes with 400 Bad Request).
  - `GET /api/v1/repos/diff`: read-only Git diff and status inspection.
- Console UI: Added `renderChatCards` and `updateChat` in `web/lib/console-presentation.js`, and chat loading, deletion, and approval handlers in `crates/daemon/src/console-app.js`.
- All quality gates verified: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (all 82 tests passing), `cargo deny check`, and web/desktop/npm test suites passing.
