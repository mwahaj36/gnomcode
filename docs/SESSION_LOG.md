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
- **M3 PR 1 (`feat/m3-tray-scaffold`):** Created `crates/tray` (native desktop tray controller, `TrayStatus` computation from active sessions, zero-emoji status formatting `[RUN]`, `[WAIT]`, `[IDLE]`, and unit tests). All 5 quality gates passed (`check.ps1`).
