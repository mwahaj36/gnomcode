# Hosted open-tool presence - 2026-11-13

Goal: Keep Gnomcode-hosted sessions in `Running` past the 10-minute `OPEN_EXECUTION_LIMIT_MS` cutoff when the host process is positively alive and the latest activity event is an open `tool_start`, while keeping non-hosted sessions, dead hosts, subagent rules, and approval limits unchanged.

Outcome and verification: Added `classify_presence_with_host` in `crates/core/src/presence.rs`, `native_ids_for_hosts` and `list_sessions_filtered_at` / `list_sessions_filtered_with_hosts` / `list_sessions_paginated_with_hosts` in `crates/store`, and wired `live_hosted_native_ids` in `crates/daemon/src/api/sessions.rs` and `crates/daemon/src/sync.rs`. Fixed-timestamp unit tests verify hosted+alive+open `tool_start` stays `Running` past 10 minutes, hosted+dead and non-hosted degrade to `Inactive` (`unknown`), and `active`/`ongoing` counts match returned rows.

Decision: Pass live hosted native session IDs from the daemon (resolved via `gnomcode_pty::try_list_active_sessions`, `conversation_member`, and `session_bindings`) into pure/SQLite Rust classification without altering `core`'s zero-I/O boundary.
