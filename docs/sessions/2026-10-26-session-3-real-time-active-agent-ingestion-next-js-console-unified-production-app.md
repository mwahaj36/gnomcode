## 2026-10-26 — Session 3: Real-Time Active Agent Ingestion, Next.js Console & Unified Production App

### 1. Goals
- Fix incremental foreign key cascades that caused active agents to freeze and quota pools to stick at 99.0%.
- Track mid-session model shifts in Codex and Antigravity.
- Transition legacy static dashboard to a unified, production-grade Next.js 15 Web & Native Console (`/console`, `/docs`, `/pricing`, `/downloads`, `/settings`, `/auth`).
- Unify command naming: `spawnt console` replacing legacy `dashboard`.
- Build native Windows installer with system tray background service and autostart.

### 2. Problems & Root Causes
| Problem | Root Cause | Fix |
| :--- | :--- | :--- |
| Zero active agents & stale 99.0% quota | Incremental chunks lacked line 1 `session_meta`, triggering foreign key rollbacks in SQLite and aborting subsequent scans | Extracted exact 36-char session UUID from file path in `codex_session_id_from_path()`; updated `fallback_model` in ingest to preserve prior turn model. |
| Dashboard was disconnected static HTML | Legacy M2 used static `dashboard.html` instead of the Next.js app | Unified on Next.js 15 Console at `/console`, added `spawnt console` CLI command, and configured native tray/shortcuts to launch the unified console with token authentication. |
| CLI command produced no output on Windows | `#![windows_subsystem = "windows"]` detached stdout before CRT startup | Removed GUI subsystem flag from CLI; background tasks use `wscript.exe` with `0, False` for zero-window execution. |

### 3. Verification
- `cargo test --workspace`: 36 unit, golden, and integration tests passed cleanly in 3.1s.
- `cargo clippy --workspace --all-targets -- -D warnings`: 0 warnings, 0 errors.
- `cargo fmt --all -- --check`: 100% formatted.
- `npm run build --prefix web`: 12/12 static routes compiled cleanly in 1.5s.
- Live test: `spawnt status` and `spawnt report` correctly detect running agents and display updated quota pools (93.0% / 30.0%).

---
