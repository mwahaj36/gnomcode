# 0017 — Structured drivers and local redacted conversation history

- **Status:** Accepted
- **Date:** 2026-10-28

## Context

Users switching models or continuing sessions across agents require conversation history. In the original design, Spawnt stored no conversation text whatsoever. Owner review approved local, Spawnt-owned persistent history specifically for session continuity and model switches, while preserving agent-owned history and native session IDs.

However, storing raw prompt text or tool outputs presents privacy, security, and leak risks:
1. Conversation history must not be added to the unencrypted cloud metadata sync.
2. Sensitive credentials (API keys, environment variables, passwords) must never be persisted unredacted.
3. Users must have full ownership and hard-deletion capabilities to permanently purge stored chat records without affecting agent log files.
4. File and diff inspection tools must not allow directory traversal or arbitrary system file access.

## Direction

1. **Local Redacted Chat Store (`crates/store`)**:
   - Spawnt maintains a dedicated SQLite table `chat_event` keyed by `session_id`.
   - All user prompts, assistant outputs, and tool summaries must pass through `spawnt_core::redact` before insertion.
   - History is strictly local: the table is excluded from cloud sync queries and payloads.
   - Configurable retention automatically prunes events older than the user-configured window.
   - Hard deletion (`DELETE /api/v1/sessions/{id}/chat`) permanently deletes all stored conversation rows for the specified session.

2. **Structured Drivers & Approval Handling (`crates/daemon`)**:
   - Structured events represent user turns, assistant reasoning, tool execution, and permission approvals.
   - Permission approval requests (`POST /api/v1/sessions/{id}/permissions`) record the user's decision without ever escalating agent permissions (auto-resume and guardrails never inject `--dangerously-skip-permissions` or `--yolo`).

3. **Read-Only File and Diff Inspection**:
   - `GET /api/v1/repos/file` provides a safe, read-only file viewer for files referenced by agents. Paths are strictly canonicalized and contained within the repository root; traversal attempts (`..`, symlink escapes) return 400 Bad Request.
   - `GET /api/v1/repos/diff` inspects workspace Git status and unified diff without writing or modifying files.

## Consequences

Chat history enables structured cards in the desktop and web console and seamless zero-token handoffs in Phase 4. Hard deletion gives users guaranteed local data hygiene. SQLite migrations preserve existing databases idempotently.
