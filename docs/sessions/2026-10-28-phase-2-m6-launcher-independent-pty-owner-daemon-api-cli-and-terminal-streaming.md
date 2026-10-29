## 2026-10-28 — Phase 2: M6 launcher, independent PTY owner, daemon API, CLI, and terminal streaming

- Implemented `crates/pty` independent session owner architecture per ADR 0016. Agents are spawned in a detached process (`spawnt pty-host`) holding the ConPTY (Windows via `conpty-oxide`) or POSIX pseudoterminal (Unix via `portable-pty`) master. The daemon and desktop connect as loopback clients via WebSocket; agent processes survive daemon and Electron desktop exits.
- Verified agent process survival with integration test `fail_open_agent_process_survives_independent_lifecycle` in `crates/cli/tests/launch_lifecycle.rs`, confirming that killing the parent and detaching clients leaves the agent running until an explicit stop request.
- Implemented bounded in-memory `Scrollback` (256 KB) in `crates/pty`. Terminal output is kept in-memory and never written to the metadata database or cloud sync.
- Implemented daemon authenticated endpoints: `POST /api/v1/sessions/launch`, `POST /api/v1/sessions/{id}/stop`, `GET /api/v1/sessions/{id}/pty` (bidirectional WebSocket proxy), and `GET /api/v1/models` (listing verified model catalog with context window, pricing, and live quota headroom).
- Implemented CLI commands: `spawnt launch <agent> [--repo <path>] [--model <model>]`, `spawnt stop <session-id>`, `spawnt attach <session-id>` (interactive streaming via WebSocket), and `spawnt models`.
- Desktop IPC wired via `contextBridge` in Electron (`desktop:launch-session`, `desktop:stop-session`) with strict origin, frame, and daemon navigation validation.
- All quality gates verified: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (all 77 tests passing), `cargo deny check`, and desktop/web/npm lint and typecheck suites passing. ADR 0016 marked Accepted.

# Spawnt Engineering Session Log

Ongoing engineering ledger recording goals, outcomes, problems encountered, root-cause fixes, and product/architectural decisions.
