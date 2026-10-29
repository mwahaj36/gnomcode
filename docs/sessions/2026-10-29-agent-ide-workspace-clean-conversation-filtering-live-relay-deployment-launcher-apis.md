## 2026-10-29 — Agent IDE workspace, clean conversation filtering, live relay deployment & launcher APIs

- **Branch**: `feat/chat-launcher-and-remote`
- **Clean conversation extraction (`crates/adapters/src/antigravity.rs`)**:
  - Filtered out 113+ raw tool execution logs (`Created At:`, `Command output:`, `Task:`) from being parsed as Assistant messages.
  - Implemented `extract_antigravity_user_prompt` to extract clean user request text inside `<USER_REQUEST>` and strip metadata tags (`<ADDITIONAL_METADATA>`, `<USER_SETTINGS_CHANGE>`).
  - Updated adapter tests in `crates/adapters/src/lib.rs` (18/18 passing).
- **Backend API additions (`crates/daemon/src/api.rs`, `crates/pty/src/lib.rs`)**:
  - Added `POST /api/v1/sessions/{id}/prompt`: delivers prompt directly into active hosted PTY sessions via loopback WebSocket without requiring shell keystroke hacks.
  - Added `GET /api/v1/sessions/hosted`: lists currently active PTY hosted sessions.
  - Added `GET /api/v1/agents/installed`: inspects host system PATH (`where.exe` on Windows / `which` on Unix) to check if Claude Code, Codex, or Gemini CLI are installed, providing installation hints (`npm install -g ...`) and login commands.
  - Extended `spawnt_pty` to support `"terminal"` and `"shell"` (`powershell.exe` on Windows, `$SHELL` on Unix), allowing in-app terminal sessions for tool installation and login.
- **Production Cloudflare Remote Relay (`workers/remote-relay`)**:
  - Fixed Durable Objects configuration migration to `new_sqlite_classes = ["RelayRoom"]` for free tier compatibility.
  - Deployed live Cloudflare Worker relay: `https://spawnt-remote-relay.suave-station.workers.dev` (`wss://.../relay`).
  - Added default fallback in `crates/daemon/src/remote.rs` to point to the live relay if `SPAWNT_REMOTE_RELAY_URL` is unset, resolving the 503 "Remote relay is not configured" error.
  - Created standalone Node.js fallback dev relay (`workers/remote-relay/dev-server.mjs`).
- **Remote dispatch extensions (`crates/daemon/src/remote.rs`)**:
  - Added `launch` command to encrypted remote WebSocket protocol, allowing remote session creation from phone/browser.
  - Enabled direct prompt delivery to hosted session IDs over encrypted remote channel.

