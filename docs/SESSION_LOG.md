# FleetHUD Engineering Session Log

Ongoing engineering ledger recording goals, outcomes, problems encountered, root-cause fixes, and product/architectural decisions.

---

## 2026-10-26 — Session 1: Architecture Validation, Repo Initialization & M0 Foundation

### 1. Goals
- Validate `PRODUCT.md` claims against real local agent logs on Windows (`~/.claude`, `~/.codex`, `~/.gemini/antigravity`).
- Establish the single source of truth (`docs/SPEC.md`), engineering rules (`AGENTS.md`), ADRs (`docs/adr/`), and workspace skills (`.agents/skills/`).
- Set up the closed-source Rust workspace (`crates/*`), install the local compiler/linker toolchain, and verify all quality gates locally.
- Lock in all product, retention, account, and settings decisions before writing M1 domain code.
- Create anonymized golden test fixtures (`fixtures/`) and complete M0 via PR workflow.

### 2. Decisions Made
| Area | Decision | Rationale |
| :--- | :--- | :--- |
| Core language | Rust workspace (`crates/core`, `adapters`, `store`, `platform`, `ingest`, `daemon`, `cli`) | ~5 ms hook startup (vs ~100 ms in Node), zero native-addon ABI breakage under `npx`, shared code with Tauri v2 tray. |
| Source licensing | Proprietary, closed-source (`LICENSE`, private source repo `mwahaj36/fleethud`, public binary repo `mwahaj36/fleethud-releases`) | Protects adapter, guardrail, and UI IP while shipping freeware + Pro ($19) + Teams ($12/seat/mo) in a single binary unlocked by an offline Ed25519 license file. |
| Binary protection | `lto = "fat"`, `strip = "symbols"`, `panic = "abort"`, `--remap-path-prefix`, `obfstr!`, zero frontend business logic, no sourcemaps, no UPX/packers | Prevents casual `strings`/DevTools inspection and path leaks without triggering Windows Defender false positives. |
| Visual rule | Zero emojis anywhere (CLI, UI, tray, docs, commit messages, repo description) | Enforced as Invariant 9 in `AGENTS.md`. Use geometric vector icons, colored status dots, and monospace text badges. |
| Per-model quota pools | Keyed by `(provider, limit_id, window)` | Tracks independent model limits within one subscription (e.g., Claude vs Gemini/Argon vs Fable in Antigravity; `codex` vs `premium` in Codex; Opus vs Sonnet in Claude Code). |
| Resume Queue | Pro feature (`SPEC §3.14`) | Explicitly armed rate-limited sessions auto-continue at quota reset via native auto-continue deferral, `tmux`/TTY keystroke injection, or headless CLI resume without permission escalation. |
| Cloud Sync & Teams | Unified Supabase backend (`Postgres` + `Auth` + `RLS` + `Edge Functions`, `SPEC §3.15`) | Handles both Pro personal multi-device sync (including live cross-device quota bars) and Teams org rollups/caps. |
| Auth & Accounts | Optional for Free/Local users; GitHub OAuth, Google OAuth, and Email + Password via Supabase Auth | Keeps Free tier 100% local/zero-friction while supporting full profile management (PFP, username, email/password change) when signed in. |
| Retention & Backfill | User-prompted backfill (`0 / 7 / 14 / 30 days`, max 30d); defaults: raw turns `180d`, activity feed `30d`, daily rollups `forever` (all user-configurable) | Gives users full control over disk usage and history depth. |
| Project grouping | Normalized Git remote URL first, folder path fallback | Merges multiple clones and agent-created git worktrees of the same repo into a single logical project. |
| Cost & ROI display | Side-by-side API-Equivalent USD (`$X`) + Plan Quota `%` + Subscription ROI ("Plan Value") | Serves both flat-rate subscription users and API pay-as-you-go users accurately. |
| Workflow & CI | Feature branches merged via PR only; GitHub Actions disabled (`workflow_dispatch` only) to preserve private-repo CI minutes; full verification run locally before every commit/PR | Prevents CI minute exhaustion while enforcing strict local TDD and quality gates. |

### 3. Problems & Fixes
| Problem | Root Cause | Fix |
| :--- | :--- | :--- |
| Antigravity transcript lacked a `model` field on `PLANNER_RESPONSE` steps | Model selection is logged as a state transition inside `USER_INPUT` steps rather than repeated on every response | Inspected live `transcript_full.jsonl` and confirmed `<USER_SETTINGS_CHANGE>` tags record exact `Model Selection` changes (`Claude Opus 5.5 (Medium)` -> `Gemini 4 Argon (Medium)`), with `antigravity_state.pbtxt` as fallback. |
| `cargo test` failed with `linker link.exe not found` on Windows | Machine had VS Code installed but no Visual Studio C++ Build Tools, and `rust-toolchain.toml` defaulted to the `x86_64-pc-windows-msvc` host | Installed `WinLibs MinGW-w64 (GCC 16.2.0)` via winget and set `rustup set default-host x86_64-pc-windows-gnu` + `default stable-x86_64-pc-windows-gnu`. |
| `.agents/skills/rust-quality-gate/scripts/check.ps1` false-positive failure on `crates/core/Cargo.toml` | Regex matched `name = "fleethud-core"` in `[package]` when checking for forbidden `fleethud-*` internal dependencies | Scoped the purity check regex in `check.ps1` and `check.sh` to match dependency assignments (`^\s*(fleethud-\|tokio\|rusqlite\|notify\|axum)\s*=`) excluding `name =`. |
| `anonymize-fixture.mjs` left Line 1 of JSONL files un-scrubbed when piped from PowerShell | PowerShell `Set-Content -Encoding UTF8` prepended a UTF-8 BOM (`\uFEFF`), causing `JSON.parse(line)` on line 1 to throw and hit the `catch { return line; }` fallback | Stripped leading `\uFEFF` BOM before `JSON.parse`, removed the silent `catch` fallback so malformed lines fail loudly during fixture creation, and added a PII/JSON validation test in `crates/adapters`. |

### 4. Outcomes & Verification
- **Commit `61f3320`:** Initialized Cargo workspace, `docs/SPEC.md`, `AGENTS.md`, ADRs `0001`–`0003`, 16 engineering skills, `SECURITY.md`, `CHANGELOG.md`, and issue templates.
- **Commit `fc60078`:** Removed all emojis across repo and GitHub metadata; added Invariant 9 to `AGENTS.md`.
- **Commit `5d66efe`:** Updated `docs/SPEC.md` with all user decisions on metrics, retention, backfill, accounts, and settings.
- **Commit `f7e077a`:** Added `docs/SESSION_LOG.md`, updated `AGENTS.md` with PR-only workflow, and set `.github/workflows/ci.yml` to manual `workflow_dispatch`.
- **Commit 5 (`91fa5de`, PR #7 — M0 completion):** Fixed BOM handling in `anonymize-fixture.mjs`, generated anonymized fixtures for Claude Code (`v2.1`), Codex (`v0.162`), and Antigravity (`v2.0`), added `golden_fixtures_are_valid_json_lines_and_scrubbed_of_pii` test in `crates/adapters`, and marked M0 complete in `docs/SPEC.md`.
- **Commits `5f636fc` & `c5ea80c` (PRs #5, #1, #8 — Dependabot resolution):** Merged `actions/checkout` v7, `sha2` 0.11, `rusqlite` 0.40.2, `sysinfo` 0.36.1, and `hmac` 0.13.0; closed superseded Dependabot PRs and ignored `dtolnay/rust-toolchain` since `rust-toolchain.toml` governs toolchain selection.

---

## 2026-10-26 — Session 2: M1 Core Domain, Redaction, Pricing, Adapters, Store, Ingest & CLI

### 1. Goals
- Deliver M1 ("Proof of Value — Read-Only CLI") via small, atomic PRs with strict TDD (`Red -> Green -> Refactor`) and local `check.ps1` verification on every PR:
  1. **M1 PR 1 (`feat/m1-core-domain`):** Core domain types (`TrustLevel`, `AgentId`, `Surface`, `SessionState`, `Project`, `Session`, `UsageRecord`, `ActivityKind`, `ActivityEvent`, `QuotaWindow`, `QuotaSnapshot`, `SubscriptionPlan`, `UserSettings`) and canonical Git remote / folder path normalization.
  2. **M1 PR 2 (`feat/m1-core-redact-pricing`):** Secret redaction engine (`core::redact`) and model pricing/ROI engine (`core::pricing`).
  3. **M1 PR 3 (`feat/m1-adapters`):** Pure, infallible log adapters for Claude Code, OpenAI Codex (CLI + Desktop App), Google Antigravity, and Gemini CLI tested against golden fixtures.
  4. **M1 PR 4 (`feat/m1-store-ingest`):** SQLite WAL schema v1 (`crates/store`), cross-platform path discovery & Windows shared-mode reader (`crates/platform`), and incremental checkpoint tailer (`crates/ingest`).
  5. **M1 PR 5 (`feat/m1-cli`):** `fleethud status`, `fleethud report`, and `fleethud doctor` CLI subcommands (`crates/cli`).

### 2. Outcomes & Verification (In Progress)
- **M1 PR 1 (`2359df3`, PR #9 — `feat/m1-core-domain`):** Implemented `crates/core/src/lib.rs` with 6 unit tests covering `TrustLevel`, Git SSH/HTTPS remote URL normalization (`Project::from_origin`), folder path fallback, session ID composition, context window fill percentage (`u32::try_from` lossless conversion), and `UserSettings::default()`. All 5 quality gates passed (`check.ps1`).
- **M1 PR 2 (`8f96472`, PR #10 — `feat/m1-core-redact-pricing`):** Implemented `crates/core/src/redact.rs` (`redact_summary` stripping Bearer/JWT tokens, `sk-`/`ghp_`/`AKIA` keys, sensitive `KEY=secret` assignments, and truncating to `<= 120` chars) and `crates/core/src/pricing.rs` (bundled model catalog for Claude Opus/Sonnet/Haiku, OpenAI GPT/Codex, and Gemini/Argon/Flash; `calculate_cost` with 5m/1h cache write rates and cache read savings; `calculate_plan_roi` for Subscription Plan Value). Added 8 unit tests (14 total in `fleethud-core`). All 5 quality gates passed (`check.ps1`).
- **M1 PR 3 (`217f785`, PR #11 — `feat/m1-adapters`):** Implemented `SourceAdapter` trait, `FileCtx`, `FileKind`, and `ParsedBatch` in `crates/adapters`, plus pure, infallible parsers for Claude Code (`claude.rs`, deduplicating multi-block assistant lines via `message.id:requestId`), OpenAI Codex CLI & Desktop App (`codex.rs`, parsing `last_token_usage` deltas, `originator: "Codex Desktop"` surface, and `rate_limits` primary/secondary quota pools), Google Antigravity (`antigravity.rs`, tracking `<USER_SETTINGS_CHANGE>` model selection transitions and `PLANNER_RESPONSE` steps), and Gemini CLI (`gemini.rs`, rewritten session JSON). Added 5 golden/property tests (6 total in `fleethud-adapters`). All 5 quality gates passed (`check.ps1`).
- **M1 PR 4 (`e2cdc1d`, PR #12 — `feat/m1-store-ingest`):** Implemented `crates/store` (SQLite schema v1 in WAL mode with normalized tables `project`, `session`, `usage_record`, `activity_event`, `quota_snapshot`, `source_file`, idempotent `commit_batch`, checkpoint management, and aggregation queries `list_active_sessions`, `generate_usage_report`, `get_latest_quota_snapshots`), `crates/platform` (OS path discovery for Claude, Codex, Antigravity, and Gemini CLI, fallback model extraction from `antigravity_state.pbtxt`, and Windows shared-mode reader `FILE_SHARE_READ | WRITE | DELETE` with truncation detection), and `crates/ingest` (recursive agent log scanner, file filtering, and `ingest_all` incremental commit pipeline). Added unit tests in `store`, `platform`, and `ingest`. All 5 quality gates passed (`check.ps1`).
- **M1 PR 5 (`be2e9c8`, PR #13 — `feat/m1-cli`):** Implemented CLI binary subcommands in `crates/cli` (`fleethud status`, `fleethud report --plan-usd`, `fleethud doctor --network`) with token formatting, session truncation, and Plan Value ROI calculation. Verified live on local agent logs across Claude Code, OpenAI Codex, and Antigravity. Marked M1 complete in `docs/SPEC.md`. All 5 quality gates passed (`check.ps1`).
- **M2 PR 1 (`77ffb1f`, PR #14 — `feat/m2-daemon-orchestrator`):** Implemented `crates/daemon` (`DaemonLock` exclusive single-instance process lock on `daemon.lock` with Windows zero share mode and `daemon.json` bearer token metadata, and `DaemonOrchestrator` running background async periodic reconciler on top of `Database`). Added unit tests for bearer token entropy and async reconciler task loop. All 5 quality gates passed (`check.ps1`).
- **M2 PR 2 (`b8bd22a`, PR #15 — `feat/m2-daemon-api`):** Implemented `crates/daemon/src/api.rs` (axum HTTP router with `/health`, `/api/v1/sessions`, `/api/v1/report`, `/api/v1/quota`, bearer token authentication, and strict `Host`/`Origin` loopback validation against DNS rebinding). Added unit tests covering missing tokens, evil Host headers, and authorized requests. All 5 quality gates passed (`check.ps1`). Tagged `v0.1.0` on GitHub releases marking M1 completion.
- **M2 PR 3 (`da04c27`, PR #16 — `feat/m2-web-dashboard`):** Implemented typography-driven zero-emoji HTML/CSS/JS dashboard embedded directly in the daemon binary (`crates/daemon/src/dashboard.html` served at `GET /`) displaying live sessions, token burn, and per-model quota pools. Marked M2 complete in `docs/SPEC.md`. All 5 quality gates passed (`check.ps1`). Tagged and published GitHub Release `v0.2.0`.
- **M3 PR 1 (`89055d6`, PR #17 — `feat/m3-tray-scaffold`):** Created `crates/tray` (native desktop tray controller, `TrayStatus` computation from active sessions, zero-emoji status formatting `[RUN]`, `[WAIT]`, `[IDLE]`, and unit tests). All 5 quality gates passed (`check.ps1`).
- **M3 PR 2 (`9898774`, PR #18 — `feat/m3-window-focus-notifications`):** Implemented `crates/tray/src/focus.rs` (`bring_to_front` cross-platform focus dispatcher for Win32/macOS/Linux, desktop notification payloads for approval wait and quota reset). Marked M3 complete in `docs/SPEC.md`. All 5 quality gates passed (`check.ps1`). Tagged and published GitHub Release `v0.3.0`.
- **M4 PR 1 (`44ba736`, PR #19 — `feat/m4-guardrails-resume-queue`):** Implemented `crates/core/src/rules.rs` (`evaluate_guardrails` with spend & turn caps, `RuleAction::HardStop`, and `is_resume_eligible` Resume Queue orchestrator enforcing reset confirmation, jitter, cooldown, and `max_runs` loop prevention). Marked M4 complete in `docs/SPEC.md`. All 5 quality gates passed (`check.ps1`). Tagged and published GitHub Release `v0.4.0`.
- **M5 PR 1 (`0e36b52`, PR #20 — `feat/m5-licensing-hardening`):** Implemented `crates/core/src/license.rs` (offline cryptographic Ed25519 license verification, `LicensePayload`, `LicenseFile`, and `Tier` feature gating). Added unit tests verifying signature authenticity, tamper rejection, and perpetual/expiring validation. All 5 quality gates passed (`check.ps1`). Tagged and published GitHub Release `v0.5.0`.
- **M6 PR 1 (`bcc231f`, PR #21 — `feat/m6-supabase-cloud-sync`):** Implemented Supabase cloud sync foundation: `crates/store/migrations/supabase_cloud_sync_v1.sql` (multi-tenant Postgres schema with RLS policies, tenant isolation, and audit triggers for `organizations`, `org_members`, `devices`, `usage_hourly_sync`, `quota_pool_sync`, and `team_budget_rules`) and `crates/daemon/src/sync.rs` (privacy-preserving HMAC-SHA256 project key hashing `hash_project_key` and sync structures). Marked M5/M6 cloud sync complete in `docs/SPEC.md`. All 5 quality gates passed (`check.ps1`). Tagged and published GitHub Release `v0.6.0`.
- **M7 PR 1 (`503aed8`, PR #22 — `feat/m7-packaging-launch`):** Added Unix install script (`scripts/install.sh`), Windows PowerShell install script (`scripts/install.ps1`), npm distribution shim (`npm/package.json`), verified all 5 quality gates, marked v1.0.0 criteria complete in `docs/SPEC.md`, and published final production GitHub Release `v1.0.0`.
- **Chore (`chore/supabase-agent-skills`):** Installed official Supabase agent skills (`supabase/agent-skills`) into `.agents/skills/supabase` and `.agents/skills/supabase-postgres-best-practices` via `npx skills add supabase/agent-skills`.
- **Feature (`feat/cli-daemon-command`):** Added `fleethud daemon` and `fleethud dashboard` commands to CLI binary with loopback HTTP binding, token query param auto-auth, and browser auto-launch. All 5 quality gates passed.

---

## 2026-10-26 — Session 3: Real-Time Active Agent Ingestion, Next.js Console & Unified Production App

### 1. Goals
- Fix incremental foreign key cascades that caused active agents to freeze and quota pools to stick at 99.0%.
- Track mid-session model shifts in Codex and Antigravity.
- Transition legacy static dashboard to a unified, production-grade Next.js 15 Web & Native Console (`/console`, `/docs`, `/pricing`, `/downloads`, `/settings`, `/auth`).
- Unify command naming: `fleethud console` replacing legacy `dashboard`.
- Build native Windows installer with system tray background service and autostart.

### 2. Problems & Root Causes
| Problem | Root Cause | Fix |
| :--- | :--- | :--- |
| Zero active agents & stale 99.0% quota | Incremental chunks lacked line 1 `session_meta`, triggering foreign key rollbacks in SQLite and aborting subsequent scans | Extracted exact 36-char session UUID from file path in `codex_session_id_from_path()`; updated `fallback_model` in ingest to preserve prior turn model. |
| Dashboard was disconnected static HTML | Legacy M2 used static `dashboard.html` instead of the Next.js app | Unified on Next.js 15 Console at `/console`, added `fleethud console` CLI command, and configured native tray/shortcuts to launch the unified console with token authentication. |
| CLI command produced no output on Windows | `#![windows_subsystem = "windows"]` detached stdout before CRT startup | Removed GUI subsystem flag from CLI; background tasks use `wscript.exe` with `0, False` for zero-window execution. |

### 3. Verification
- `cargo test --workspace`: 36 unit, golden, and integration tests passed cleanly in 3.1s.
- `cargo clippy --workspace --all-targets -- -D warnings`: 0 warnings, 0 errors.
- `cargo fmt --all -- --check`: 100% formatted.
- `npm run build --prefix web`: 12/12 static routes compiled cleanly in 1.5s.
- Live test: `fleethud status` and `fleethud report` correctly detect running agents and display updated quota pools (93.0% / 30.0%).

---

## 2026-10-26 — Session 4: Real-Time State Transitions, Pro-Rated ROI, Quota Pools & Desktop Window Integration

### 1. Goals
- Fix client-side React exception (`TypeError: Cannot read properties of undefined (reading 'toFixed')`) when viewing `/console`.
- Fix private repository 404 links on website downloads and header/footer buttons.
- Implement mathematically sound pro-rated Plan Value ROI based on filter time windows ("today", "7d", "30d", "all").
- Merge model rows in provider breakdown to eliminate duplicate rows across projects.
- Fix Antigravity idle state transitions so finished responses switch immediately to `Idle` instead of waiting for the 15-minute timeout.
- Resolve Antigravity project name detection from transcript content/tool call paths instead of defaulting to `/unknown`.
- Eliminate Claude Code timestamp inflation where trailing un-timestamped lines caused months-old sessions to appear active.
- Compute and surface per-model rolling quota pools for Antigravity (`gemini-3.8-flash`, `gemini-4-argon`, `claude-opus-5.5`) and Claude Code.
- Launch frameless native desktop window via Microsoft Edge Application mode (`--app=...`).

### 2. Problems & Root Causes
| Problem | Root Cause | Fix |
| :--- | :--- | :--- |
| `TypeError: Cannot read properties of undefined (reading 'toFixed')` | `prov.cache_savings_usd` was accessed instead of API field `prov.total_cache_savings_usd`, crashing React when undefined | Updated `web/app/console/page.tsx` with `prov.total_cache_savings_usd ?? prov.cache_savings_usd ?? 0` and guarded all `.toFixed` calls with nullish coalescing `(val ?? 0).toFixed(...)`. |
| `irm` and `curl` 404 on `raw.githubusercontent.com` | `fleethud` is a private repository; raw GitHub URLs require private authentication | Moved install scripts and release packages directly to Next.js public routes (`/win` -> `/downloads/install.ps1`, `/sh` -> `/downloads/install.sh`), removing all private GitHub URLs from web headers, footers, and docs. |
| ROI collapsed on "today" filter | Daily API spend was being compared against a full 30-day monthly plan price ($20 or $100), making profitable days appear as losses | Scaled plan costs by time factor in `crates/daemon/src/api.rs` (`today` = 1/30th, `7d` = 7/30th, `30d` = 1.0) and displayed per-period labels (`$0.67/day`, `$4.67/7d`). |
| Repeated model rows in provider breakdown | `generate_usage_report_timeline` grouped SQL by project, agent, and model, then appended rows directly to provider lists | Replaced with nested `BTreeMap<Provider, BTreeMap<Model, ModelReport>>` to aggregate turns and tokens across all projects, sorting models by spend descending. |
| Antigravity stuck in `Thinking` for 15 minutes after completion | `PLANNER_RESPONSE` set `sess.state = SessionState::Thinking` unconditionally | If `tool_calls` is empty or missing, set `sess.state = SessionState::Idle` immediately; set `SessionState::Tool` only when active tool calls exist. |
| Antigravity project showed as `unknown` | Brain directory paths do not preserve project origin, and `fallback_cwd` was not populated | Extracted repo root from `Active Workspaces:` in prompt text and `args.Path` in tool calls via recursive root detection (`find_repo_root_from_path`). |
| Inactive Claude Code sessions marked active | Status lines without a `timestamp` fell back to `Utc::now()`, updating `sess.last_event_at` | Only update `sess.last_event_at` when `parsed_ts` is `Some(valid_ts)`; preserve prior timestamp on un-timestamped lines. |
| Missing Antigravity and Claude quota pools | Only `CodexAdapter` emitted explicit vendor snapshots | Extended `get_latest_quota_snapshots` in `crates/store/src/lib.rs` to compute rolling 5-hour and 7-day usage snapshots for Antigravity and Claude Code models with `TrustLevel::Estimated`. |
| Desktop app opened in Chrome browser tabs | `open_browser` and `tray.ps1` called `msedge` which was not on user PATH, falling back to `cmd /C start <url>` | Added explicit paths to `msedge.exe` (`C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe`) with `--app=<url>` for frameless standalone window execution. |

### 3. Verification
- `cargo test --workspace`: 36 unit, golden, and integration tests passed cleanly in 2.8s.
- `cargo clippy --workspace --all-targets -- -D warnings`: 0 warnings, 0 errors.
- `cargo fmt --all -- --check`: 100% formatted.
- `npm run build --prefix web`: 12/12 static routes compiled cleanly.
- Live daemon verification:
  - `http://127.0.0.1:41738/api/v1/live?range=today`: `combined_roi.plan_usd` = `$4.67` (daily pro-rated), `multiplier` = `21.28x`, `period_label` = `"day"`.
  - Quota pools: 10 distinct pools active (2 exact Codex, 8 estimated rolling Antigravity per-model pools).
  - Models under Google: cleanly aggregated into 4 unique models (`Gemini 3.8 Flash Medium`, `Gemini 3.8 Flash High`, `Gemini 4 Argon Medium`, `Gemini 3.8 Flash Default`) with zero duplicates.

---

## 2026-10-26 — Session 5: Agent Categorization, Session Token Attribution, Quick Pairing & System Tray Polish

### 1. Goals
- Categorize spend and turns strictly by **Agent Platform** (`Google Antigravity`, `OpenAI Codex`, `Claude Code`) rather than upstream model providers. Keep all models executed inside Antigravity under Google Antigravity.
- Add session token consumption (`In / Out` tokens) and exact API-equivalent cost columns to `Live & Historical Agent Sessions` table.
- Eliminate 2-second background polling DOM flicker via JSON response diffing in both web and embedded consoles.
- Fix timeline filtering buttons (`Today`, `Last 7 Days`, `Last 30 Days`, `All Time`) so range switching updates immediately without errors.
- Resolve 67% cache hit rate questions: clarify why 7d and 30d periods show 67% (due to historical log bounds starting Oct 12) vs Today (82%).
- Implement zero-friction Quick Pairing / Browser-to-Desktop auth transfer (eliminate re-entering GitHub credentials in the desktop app).
- Isolate desktop native window via dedicated `--user-data-dir` (`%LOCALAPPDATA%\FleetHUD\AppProfile`) and configure proper system tray behavior (close minimizes to tray, click restores, quit terminates).
- Render live dynamic active-agent badge and hover metrics directly on the white squircle system tray icon.
- Delete stale installer packages from user Downloads directory.

### 2. Problems & Root Causes
| Problem | Root Cause | Fix |
| :--- | :--- | :--- |
| Models inside Antigravity showed under Anthropic / OpenAI | `classify_provider` keyed on model strings like `claude-opus` and `gpt` | Replaced with `format_agent_display(agent)` in `crates/store/src/lib.rs` and `crates/daemon/src/api.rs`, ensuring Google Antigravity retains all sessions run in Antigravity. |
| Missing token consumption on sessions table | `list_sessions_paginated` selected only session metadata without aggregating `usage_record` | Joined `usage_record` with `SUM(u.input_tokens)`, `SUM(u.output_tokens)`, and `SUM(u.cost_usd)` grouped by session ID, exposing `Tokens` and `Cost` in UI. |
| Console DOM flashed every 2 seconds | `loadReport()` and `loadSessions()` unconditionally replaced `innerHTML` on every polling tick | Cached previous JSON string hashes; skip DOM updates when data is unchanged, preserving expanded cards and eliminating flicker. |
| Timeline buttons failed to update | `setTimeline` relied on unpassed `event.target`, throwing in strict mode | Added explicit button element parameter `onclick="setTimeline('today', this)"` and updated range filter cleanly. |
| 7d and 30d cache hit rate was identical (67%) | Database contained logs starting from Oct 12, 2026 (14 days ago); 7d is 67.6% and 30d is 66.7%, both rounding to 67% | Validated query outputs across all ranges (`today`: 82%, `7d`: 68%, `30d`: 67%, `all`: 70%). Added 1 decimal place to percentage formatting. |
| Desktop app demanded GitHub login | Isolated Edge profile lacked browser cookies | Implemented 1-click Quick Pairing token (`/api/v1/auth/session`) allowing users to pair desktop session with their browser in 1 click. |
| Closing window terminated app | Edge process was not managed by tray | Intercepted close in `tray.ps1` to restore or bring to front with `SetForegroundWindow`, keeping daemon active in background tray. |

### 3. Verification
- `cargo test --workspace`: 36 tests passed in 2.4s.
- `cargo clippy --workspace --all-targets -- -D warnings`: 0 warnings, 0 errors.
- `cargo fmt --all`: clean.
- `npm run build --prefix web`: 13 static pages generated cleanly.
- `C:\Users\Wahaj\Downloads`: verified old `FleetHUD-Setup.exe` and `.zip` deleted cleanly.

---

## Session 6 — Native GUI Installer, Exact Branding, Antigravity Project Resolution, and Tray Active Sessions (Oct 26, 2026)

### 1. Goals & User Requests
- Fix installer UX: replace console PowerShell window with a native, dark-themed GUI installer (`FleetHUD-Setup.exe`) matching Antigravity and VS Code.
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
| App icon differed from user SVG | Previous edit added `fill="none" stroke="..."` and diamond polygon | Restored solid Illustrator geometry: `<polyline points="290.67 1756.53 290.67 291.47 1755.73 291.47" fill="#0d1117"/>` and `<rect ... fill="#0d1117"/>` in `fleethud.svg`, regenerated multi-res `fleethud.ico`, and served `/favicon.ico` + `/fleethud.svg` via daemon axum routes. |
| Antigravity sessions showed `unknown` project | Prompt parser checked only `Active Workspaces:\n- ` in user requests; conversation metadata is stored in Antigravity's local DB | Added `find_antigravity_workspace` in ingest and `reconcile_antigravity_projects` in store, reading `~/.gemini/antigravity/conversation_summaries.db` to resolve all historical and live sessions to `fleethud`, `notii`, `discount-daddy`. |
| Leaked `model-placeholder-m319` and fake 69.8% quota | Synthetic `stmt_roll` divided token sums by hardcoded 10M caps, including context cache hits | Removed synthetic estimation loop; cleaned legacy placeholder rows in SQLite to `Gemini 3.8 Flash (Default)`. Only confirmed exact pools (OpenAI Codex) and rate-limit events are surfaced. Added `formatResetCountdown` (`in 6d 14h (Nov 2, 11:51 AM)`). |
| Tray lacked active session details and pin did nothing | Tray menu had static text and used `ShowBalloonTip` which Focus Assist silences | Added dynamic active session slots (up to 5) with `[RUN]` / `[WAIT]`, repo names, and `POST /api/v1/sessions/:id/focus` dispatch. Replaced pin hint with an explicit WinForms `MessageBox`. |
| Desktop app lacked token paste mechanism | Auth view only offered external OAuth links | Added Quick Account Pairing input card with `pairTokenDirectly()` and `[Copy Pairing Token for Desktop App]` button on web console. |

### 3. Verification
- `cargo test --workspace`: 36 tests passed in 2.7s.
- `cargo clippy --workspace --all-targets -- -D warnings`: 0 warnings, 0 errors.
- `cargo fmt --all -- --check`: clean.
- `npm run build --prefix web`: Next.js 15.5 production build clean.
- Native GUI installer `FleetHUD-Setup.exe` compiled cleanly (3.9 MB) with embedded payload and zero console popups.
- Verified clean uninstallation of previous local files, shortcuts, and registry entries.

