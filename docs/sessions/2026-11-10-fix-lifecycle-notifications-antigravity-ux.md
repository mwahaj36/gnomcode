# Session Record: 2026-11-10 - Fix Lifecycle, Notification Spam, and Antigravity IDE Visibility

## 1. Goals
- Eliminate spurious autostart/respawn behavior when the desktop app is closed.
- Eliminate notification spam caused by intermediate thinking/tool state changes before a turn completes.
- Ensure Google Antigravity IDE agents and workspaces are fully detected, reconciled, and visible by default in the console.
- Enhance the console UI to default to "All sessions" and provide an imported session banner with a direct window focus action for Antigravity.
- Build and cleanly deploy the updated desktop package to the local device.

## 2. Problems & Root Causes
1. **Desktop Shell Respawning on Notification Delivery:**
   - *Problem:* Quitting the desktop application resulted in it reappearing in the system tray a few minutes later.
   - *Root Cause:* In `crates/platform/src/lib.rs`, `send_desktop_notification` on Windows attempted to send notifications by launching `gnomcode-desktop.exe --gnomcode-notification ...`. When the desktop app was not running, this launched a fresh instance in the background tray.
   - *Fix:* Added process inspection via `sysinfo`. If `gnomcode-desktop.exe` is not currently running, `send_desktop_notification` aborts without launching a new process.

2. **Notification Spam After Intermediate Steps:**
   - *Problem:* Notifications fired after every single turn segment or intermediate tool execution rather than at the completion of a response.
   - *Root Cause:* In `crates/daemon/src/automation.rs`, any transition to `SessionState::Idle` immediately triggered an automation notice without waiting for subsequent tool execution or thinking steps.
   - *Fix:* Introduced an 8-second debounce settling window for `SessionState::Idle` transitions to ensure the agent has truly completed its turn before sending a completion notice.

3. **Antigravity IDE Visibility & Retention:**
   - *Problem:* Antigravity IDE sessions disappeared quickly or did not show up in the console.
   - *Root Cause:* `RECENT_ACTIVITY_GRACE_MS` in `crates/core/src/presence.rs` was set to only 120 seconds (2 minutes), causing active sessions to drop from "Ongoing" during normal user reading/thinking pauses. Furthermore, `console.html` defaulted `#conversation-source` to `managed` ("Gnomcode conversations"), which explicitly filtered out externally observed Antigravity IDE sessions. In addition, Antigravity workspace reconciliation was only run periodically instead of every ingest tick.
   - *Fix:* Extended `RECENT_ACTIVITY_GRACE_MS` to 15 minutes (900 seconds), updated `reconcile_antigravity_projects` in `crates/store/src/lib.rs` to respect `GEMINI_HOME`, ran reconciliation on every orchestrator tick, and defaulted `#conversation-source` to `all` ("All sessions"). Added an `#imported-session-banner` with one-click window focusing for Antigravity IDE sessions.

4. **Desktop Daemon Lifecycle Termination:**
   - *Problem:* Quitting the desktop app left daemon processes running or orphaned.
   - *Root Cause:* `apps/desktop/src/main.ts` did not track the spawned daemon child process PID or terminate it on quit.
   - *Fix:* Added `stopDaemon()` in `apps/desktop/src/main.ts` using `taskkill` on Windows and `SIGTERM` on Unix, executed on tray quit and `before-quit`.

## 3. Verification
- `cargo fmt --check`: Passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: Passed.
- `cargo test --workspace`: Passed (all crates).
- `cargo deny check`: Passed.
- `rust-quality-gate`: All 5 gates passed.
- `npm test --prefix apps/desktop`: Passed (25/25 tests).
- `npm test --prefix web`: Passed (30/30 tests).
