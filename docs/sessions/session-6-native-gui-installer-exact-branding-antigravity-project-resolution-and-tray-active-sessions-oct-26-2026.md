## Session 6 — Native GUI Installer, Exact Branding, Antigravity Project Resolution, and Tray Active Sessions (Oct 26, 2026)

### 1. Goals & User Requests
- Fix installer UX: replace console PowerShell window with a native, dark-themed GUI installer (`Spawnt-Setup.exe`) matching Antigravity and VS Code.
- Fix brand icon: restore exact Illustrator SVG geometry (`fill="#0d1117"` polyline triangle and rect inside white squircle), replace wireframe/diamond logo in console and tray badge, and add favicon routes so native app window displays the icon.
- Fix Antigravity `unknown` project names in sessions table by reconciling from `conversation_summaries.db`.
- Clean up legacy `model-placeholder-m319` records and remove fabricated 69.8% estimations for Antigravity, while keeping exact Codex 5h/weekly pools with date/countdown formatting (`in 6d 14h (Nov 2, 11:51 AM)`).
- Implement requested tray features (SPEC §3.12, §3.13, §4.5): show up to 5 active sessions with `[RUN]` / `[WAIT]` badges and repo names, click-to-focus window dispatcher, explicit taskbar pinning dialog, and 1-click token copy.
- Remove raw Supabase endpoint URL from Settings/Auth tab, replace with `Cloud Telemetry Mirror: Connected & Ready`, and add a dedicated Quick Account Pairing card for 1-click token/session paste.
- Cleanly uninstall previous local files from Windows so the user can test downloading and installing fresh.

### 2. Problems & Root Causes
| Problem | Root Cause | Fix |
| :--- | :--- | :--- |
| Installer opened a PowerShell terminal window | Packaging ran `install.ps1` in console mode | Built `scripts/Setup.cs` as a native Windows GUI executable (`/target:winexe`) via `csc.exe` with dark `#0d1117` UI, squircle logo, smooth progress indicators, auto-shortcuts, and zero terminal popups. |
| App icon differed from user SVG | Previous edit added `fill="none" stroke="..."` and diamond polygon | Restored solid Illustrator geometry: `<polyline points="290.67 1756.53 290.67 291.47 1755.73 291.47" fill="#0d1117"/>` and `<rect ... fill="#0d1117"/>` in `spawnt.svg`, regenerated multi-res `spawnt.ico`, and served `/favicon.ico` + `/spawnt.svg` via daemon axum routes. |
| Antigravity sessions showed `unknown` project | Prompt parser checked only `Active Workspaces:\n- ` in user requests; conversation metadata is stored in Antigravity's local DB | Added `find_antigravity_workspace` in ingest and `reconcile_antigravity_projects` in store, reading `~/.gemini/antigravity/conversation_summaries.db` to resolve all historical and live sessions to `spawnt`, `notii`, `discount-daddy`. |
| Leaked `model-placeholder-m319` and fake 69.8% quota | Synthetic `stmt_roll` divided token sums by hardcoded 10M caps, including context cache hits | Removed synthetic estimation loop; cleaned legacy placeholder rows in SQLite to `Gemini 3.8 Flash (Default)`. Only confirmed exact pools (OpenAI Codex) and rate-limit events are surfaced. Added `formatResetCountdown` (`in 6d 14h (Nov 2, 11:51 AM)`). |
| Tray lacked active session details and pin did nothing | Tray menu had static text and used `ShowBalloonTip` which Focus Assist silences | Added dynamic active session slots (up to 5) with `[RUN]` / `[WAIT]`, repo names, and `POST /api/v1/sessions/:id/focus` dispatch. Replaced pin hint with an explicit WinForms `MessageBox`. |
| Desktop app lacked token paste mechanism | Auth view only offered external OAuth links | Added Quick Account Pairing input card with `pairTokenDirectly()` and `[Copy Pairing Token for Desktop App]` button on web console. |

### 3. Verification
- `cargo test --workspace`: 36 tests passed in 2.7s.
- `cargo clippy --workspace --all-targets -- -D warnings`: 0 warnings, 0 errors.
- `cargo fmt --all -- --check`: clean.
- `npm run build --prefix web`: Next.js 15.5 production build clean.
- Native GUI installer `Spawnt-Setup.exe` compiled cleanly (3.9 MB) with embedded payload and zero console popups.
- Verified clean uninstallation of previous local files, shortcuts, and registry entries.

