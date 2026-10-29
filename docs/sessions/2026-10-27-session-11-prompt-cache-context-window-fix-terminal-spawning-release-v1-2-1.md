## 2026-10-27 — Session 11: Prompt Cache Context Window Fix, Terminal Spawning & Release v1.2.1

### 1. Goals & User Requests
- Fix Context Window Saturation Tracking: In Antigravity sessions with prompt caching, context fill remained frozen at `17.8k / 128.0k (14%)` despite 50M+ cached prompt tokens processed.
- Fix Cross-Platform Relay Process Spawning: Spawning background processes with raw CLI names failed on Windows due to `.cmd` wrappers (e.g., global npm packages) and ran invisibly without a visible console window.
- Fix Relay Dismiss Latching: Dismissing a context relay alert permitted the 5-second polling loop to re-create the pending entry immediately.
- Fix Repository Root Discovery: Context handoff fell back to daemon working directory for CLI sessions when `find_antigravity_workspace` returned `None`.
- Bump version to `1.2.1` across all 8 files, recompile all release binaries and installers, verify production Next.js build, and cut GitHub release.

### 2. Implementation & Root Causes
| Area | Problem / Gap | Resolution |
| :--- | :--- | :--- |
| **Antigravity Token Accounting** | Google Antigravity logs prompt tokens split between `input_tokens` (uncached delta) and `cache_read_tokens` (cached history). Adapter set `context_tokens: input_tokens`, completely ignoring cached history. Pre-cache turn 20 recorded 17,752, while subsequent cached turns recorded only 2k-3k, freezing `MAX(context_tokens)` at 17.8k. | Updated `crates/adapters/src/antigravity.rs` to compute `context_tokens: input_tokens.saturating_add(cache_read)`. Live context immediately updated to accurate >140k load. |
| **SQLite Session Query** | `list_sessions_paginated` queried `COALESCE(MAX(u.context_tokens), 0)`, which preserved historical pre-compaction peaks rather than current active context. | Updated query in `crates/store/src/lib.rs` to fetch latest turn `context_tokens` via subquery `ORDER BY u2.rowid DESC LIMIT 1`. Added schema migration `v7` to retroactively update `context_tokens = input_tokens + cache_read_tokens` for past rows. |
| **Terminal Process Spawning** | Headless `Command::new("claude").spawn()` executed invisibly in the background without a console window and failed to resolve Windows `.cmd` files. | Added `spawn_agent_in_terminal` in `crates/platform/src/lib.rs`. On Windows, launches in an interactive Command Prompt (`cmd /c start "Spawnt Relay" cmd.exe /k ...`); on macOS, uses AppleScript Terminal tab; on Linux, uses `x-terminal-emulator`. Focuses editor for IDE agents. |
| **Relay Dismiss Latching** | Dismissing a relay set status to `"dismissed"`, but ticker check only skipped `"pending"` and `"executed"`, immediately recreating the alert. | Updated `crates/daemon/src/pro.rs` line 282 to include `r.status == "dismissed"`. |
| **Session Root Resolution** | CWD lookup checked only Antigravity SQLite DB, defaulting CLI agents to daemon root. | Updated `crates/daemon/src/pro.rs`, `sync.rs`, and `api.rs` to query `db.get_session_focus_target(&s.id)` to obtain the canonical `project.repo_root`. |

### 3. Verification & Quality Gate
- `cargo fmt --all --check`: Clean formatting across all crates.
- `cargo clippy --workspace --all-targets -- -D warnings`: 0 warnings, 0 errors.
- `cargo test --workspace`: 49 tests passed across all 9 crates with 0 failures.
- `npm run build --prefix web`: Next.js 15.5 production build verified (14 static routes compiled cleanly).
- Version Lockstep: Bumped version to `1.2.1` in `Cargo.toml`, `scripts/Setup.cs`, `web/package.json`, `npm/package.json`, `npm/spawnt/package.json`, `scripts/install.ps1`, `web/public/downloads/install.ps1`, and `web/app/downloads/page.tsx`.
- Release Packager: Compiled native release `spawnt.exe`, compiled installer `Spawnt-Setup.exe`, and packaged `Spawnt-v1.2.1-windows-x64.zip`.
- Live Verification: Daemon reloaded locally; live session API immediately reflected dynamic context window (`158,723 / 128,000`).
