# FleetHUD Engineering Session Log

Ongoing engineering ledger recording goals, outcomes, problems encountered, root-cause fixes, and product/architectural decisions.

## 2026-10-28 — ongoing-first console and truthful pricing coverage

- Local API diagnostics confirmed the connected daemon had priced totals but aggregate unknown trust due to an unpriced model. Added Rust-priced/unpriced invocation counts at timeline, interval, provider, model, session and project levels; exclude unpriced records from displayed subtotals and label incomplete coverage explicitly.
- Overview leads with ongoing-session title/project, agent/model, inferred state, latest activity and context, independent of history filters/pagination. Limits occupy a separate desktop column and a collapsible region; history remains in Sessions. Both consoles share presenters and CSS.
- Today now uses actual local-midnight boundaries, Julian-day comparisons across RFC3339 offsets and hourly UTC buckets displayed as local time. Added a source-owned calendar-month report/snapshot and period labels. Missing hours remain unobserved, not fabricated zeros.
- Project cards navigate to a native full-query project filter before pagination. Web filtering uses only already-published session names; private project rollups remain local.
- Reduced repeated trust tags, added plain-language partial warnings, restructured Account controls, hid activation when licensed and kept explicit Replace/Remove controls. Aligned license/export actions and prevented Add-button wrapping.
- Regression coverage verifies mixed known/unknown costs, project coverage/filtering, mismatched timestamp offsets around local midnight, hourly buckets, month labels, shared presentation and ongoing identity. Windows Rust gate including cargo-deny and production web build passed; final source/package checks are run before push. Browser visual checks remain unavailable and macOS builds remain deferred.

## 2026-10-28 — Windows tray badge and icon recovery

- Reproduced GDI+ DrawIcon/ToBitmap throwing on PNG-compressed small ICO frames; the tray swallowed the failure and never applied its running-count overlay. Generate small Windows ICO frames as DIBs from the same canonical SVG, retaining a compressed large frame.
- Windows ICO frames now use an opaque #070708 canvas behind the red mark. Marketing PNG/SVG marks retain their existing backgrounds. Use ASCII tray separators to avoid Windows PowerShell UTF-8-without-BOM tooltip corruption.
- Native regression renders count one, restores count zero, reads the resulting icon bitmap and asserts an opaque brand-black corner. It failed before the ICO encoding fix and passes afterward. Tray pricing captions also distinguish partial estimates from complete estimates.

## 2026-10-28 — v1.5.1 follow-up packages

- Rebuilt Windows x64 engine/setup and Linux x64/ARM64 binaries from the navigation and compact-tour fixes. Updated all release metadata, installer mirrors and checksum manifests to 1.5.1; retained published historical artifacts.
- Full Windows Rust gate including cargo-deny, Rust 1.87 compatibility, Linux clippy/workspace tests, ARM64 version smoke, production Next build, native navigation/JavaScript, rendered tour density, shared presenters, settings failures and version synchronization passed.
- Installer payload/version validation, isolated Windows updater tests, Unix install/corrupt-manifest preservation and packaging architecture/permissions tests passed. PowerShell Compress-Archive could not read the running executable with its sharing mode; the existing Python multi-platform packager completed and its resulting Windows archive passed updater validation.
- Projects reads use the same actionable local authorization errors as other views. Native navigation regression now runs before release packaging.
- Final review reproduced an additional direct cause of the native unavailable screen: lastReportJson, lastSessionsJson and lastQuotaJson were read without declarations. Added explicit initial values and invalidate caches on failed reads so identical successful retries replace error placeholders. The regression executes a healthy quota response followed by failure and identical recovery; it failed before the declaration fix and passed afterward. Rebuilt packages again from this corrected source.
- Browser access was checked again and remains disabled in Work settings, so desktop/narrow visual inspection is not claimed. macOS builds remain explicitly deferred. Assumption: the landing tour should summarize Overview without clipping or changing the actual consoles.

## 2026-10-28 — landing overview preview density

- Replaced the full daily grid and five-column session table in the Overview tour with a compact seven-day chart and three short agent/state/token rows, preserving the same illustrative fixtures, KPIs and trust labels.
- Kept detailed sessions in their dedicated tab, reduced tour-only row/padding density and retained reduced-motion behavior. The actual web/local console layouts are unaffected.
- Verification: a rendered-component regression failed on the full table before the change, then passed for three compact rows/seven bars and retained Sessions details. Production Next build passed. Browser visual tooling remains unavailable.

## 2026-10-28 — native console authorization survives navigation

- User screenshot showed a rendered console with all local reads unavailable. Direct authenticated daemon probes returned 200 for report, sessions and quota. Reproduction showed setView replaced the URL with `/` or `/#view`, dropping the bearer query; a subsequent page reload lost authorization. The brand link also navigated to an unauthenticated root.
- Preserve the complete current query when changing views or following the brand link. No auth bypass, automatic credential recovery or persistent browser token storage was added.
- Local reads now expose actionable authorization failures, and failed report reads resolve the daily-activity loading state. Render caches are committed only after successful rendering so a transient UI failure can be retried.
- Verification: functional navigation/reload and failed-401 regression reproduced the issue before the fix; native script/target checks and full Rust gate passed.

## 2026-10-28 — reconcile final redesign with current main

- Final fetch found main had advanced through PRs 44–46, including the owner's withdrawal of hosted cloud sessions/browser device pairing. Integrated that main history before merge, retaining private desktop credential handling, desktop summary sync and the existing manual account-token flow while removing hosted-session reads/collectors from the redesigned web UI.
- Retained shared presenters, truthful freshness and reload behavior. Fixed merge-time missing status targets and a test AppState initializer. A regression reproduced retired hosted usage leaking into reports; all current report/daily/project queries now exclude those rows without deleting history.
- Renumbered redesign ADRs to 0006–0008 to preserve main's withdrawn ADR 0005 and avoid duplicate identities. Historical published release archives are retained from main; the current release remains 1.5.0.
- Windows full Rust gate including cargo-deny, Linux clippy/tests, Rust 1.87 check, production web build and source-matching Windows/Linux packaging passed. Native asset MIME/body tests and embedded syntax/runtime-target checks passed before authorized merge/publication. macOS remains deferred; browser visual checks remain unavailable.

- GitGuardian on the integration PR flags two historical occurrences of the same invalid mock JWT in main commits 0d4f190/5c2df63. PR 45 documents its synthetic signature and remediation; the fixture file is absent from current source after PR 46. No scanner configuration or exemptions were changed.

## 2026-10-28 — final website version audit and release authorization

- The user deferred macOS builds and explicitly authorized final testing as far as available tools permit, followed by merge. Browser visual checks remain unavailable and live cloud publication still awaits re-pairing; these limits are disclosed rather than treated as passes.
- Audited every displayed website release version and versioned archive/checksum link. Downloads and Docs read package metadata; both render 1.5.0. Added a permanent synchronization check for website output, workspace/npm versions, installer metadata/mirrors and release files.
- Removed the obsolete root/public Daytona prototype after the completed presenters replaced it. Preserved current domain and download paths, canonical branding and the marketing navbar.
- Windows/Linux verification passed; ready stack and v1.5.0 draft are being finalized for authorized merge/publication. Existing 1.4.0/1.4.1 tags remain unchanged.

## 2026-10-28 — release version collision correction

- Verification of the new draft exposed pre-existing published v1.4.0 and v1.4.1 tags, despite this checkout's original 1.3.0 workspace version. The initial assumption of 1.4.0 was incorrect.
- Corrected the redesign release to 1.5.0, rebuilding every available platform artifact and renaming only the draft created in this session. Existing published releases/tags remain untouched. Removed session-generated 1.4.0 archive files so they cannot be mistaken for those older tags.

## 2026-10-28 — release verification and external blockers

- Rebuilt Windows x64 setup/zip and Linux x64/ARM64 archives from final shared-console sources. Linux ARM64 executes under QEMU; Linux x64 clippy and workspace tests pass. Windows Rust gate includes installed cargo-deny; Rust 1.87 workspace check and production Next build pass. Windows setup dark render inspected without installation.
- Native GitHub CI run 40635697474 never started any job: account payments/spending limits blocked execution. Browser inventory still reports local browser use disabled after the user's enable attempt. Live desktop pairing renewal is rejected; the user chose to reconnect later.
- Prepared v1.4.0 as a draft only. Merge/publication remain held because macOS artifacts and final visual parity cannot be verified. Existing distribution paths and user data stay compatible; no public repository or new domain was created.
- Packaging now works with Ubuntu 22.04 Python 3.10 using the standard library. Regression checks cover architecture headers, executable launcher permissions, canonical app icons, bundled font notices and combined checksum preservation. Generated Python bytecode is ignored rather than committed.

## 2026-10-28 — final console parity and safety review

- Kept Settings inside the shared console rail and heading; the existing `/settings` URL forwards to its console destination. Extracted the settings panel without changing its independent account data path.
- Replaced separate web/local provider breakdowns with one keyboard-accessible presenter that preserves expanded rows on polling. Removed boxed trust treatments, made chart entry sequential, and labeled missing session details unknown.
- Escaped local automation, relay and pricing metadata, including safe inline action arguments; added a persistent regression check. Moved report and project SQLite queries to blocking tasks. Local project date filters now reject obsolete responses and use the same local-day boundary as reports.
- Verification: presenter/settings/local-script checks and full Windows Rust gate passed. Production web build and Rust 1.87 minimum-version check passed. Source-matching Windows/Linux artifacts are rebuilt; macOS and visual checks remain externally blocked. Browser visual parity cannot be asserted while browser tools remain disabled.

## 2026-10-28 — v1.4 platform packaging

- Assumption: use minor version 1.4.0 for the redesign. Keep binary/package/data identifiers and published domain/download paths compatible.
- Added native Intel/Apple Silicon macOS and x86_64/ARM64 Linux packaging, canonical app/launcher icons, font notices and cross-platform executable archive modes. Unix upgrades retain user data and verify the replacement before atomic installation. macOS browser-console app is unsigned; native Unix trays and notarization remain future work under ADR 0007.
- Windows setup uses absolute compiler inputs, version-checked embedded payloads, device-theme colors and canonical assets. Restricted upgrade/uninstall process cleanup to the installed executable/profile/tray, replacing broad process-name kills. Shortcuts display spawnt while installation/registry identities remain compatible.
- Added release architecture/permission/manifest regression checks and native CI packaging/updater verification on four Unix targets. Windows/Linux release builds passed; macOS CI is blocked before execution by account billing.

## 2026-10-28 — truthful cloud settings writes

- Removed unauthenticated loopback reads/writes from signed-in Settings. Cloud preferences and local desktop configuration have separate authority, matching the console boundary.
- Rejected saves now display errors, signed-out saves are blocked, duplicate saves are prevented, and pricing rows update only after persistence succeeds. Missing rates stay unknown; zero-valued plan preferences no longer revert to defaults. Added explicit field names and nonnegative plan constraints.
- Verification: mocked component regression exercises successful/rejected/signed-out writes; production web build and diff checks passed. Live account mutation was not performed.
## 2026-10-28 — shared console activity, sessions and quota freshness

- Added shared framework-neutral presenters and a common navigation rail, daily chart/token grid, quota panels, compact expandable session table, and project rollup presentation. Local project totals are computed in Rust and do not enter cloud snapshots.
- Added Rust daily/session/project token totals that count cached tokens once, source-generated snapshot times, remaining quota capacity, and an authenticated cloud publication-status endpoint. Fixed stored estimated quota provenance being returned as exact. Fixed web deep links that previously produced nonexistent console subroutes; query-based navigation supports Back/Forward.
- Live polling preserves expanded day/session details and focus, pauses in hidden web tabs, and does not replay chart entrance motion. Removed the cloud navbar's unauthenticated loopback probe and removed misleading immediate-completion messages for queued rotation. Rotation requires explicit confirmation and prevents concurrent clicks.
- Replaced the landing's generic console mock tables with the completed shared session/activity/quota presenters. The illustrative week totals 296k tokens and $3.78, with 39.3k tokens and $0.86 on its final day; the three daily sessions reconcile to those day totals. Added keyboard navigation to tour tabs and reused the platform selector in documentation. Shared summary cells now have identical order and layout in both consoles; missing pricing stays unknown.
- Live diagnosis: the installed desktop pairing token is expired and its refresh grant is rejected. The user elected to re-pair later. No credentials or activity content were printed or added to the repository.
- Verification: Rust regression tests reproduced quota trust and project/report omissions before fixes. Production web builds, embedded script parsing, shared presenter trust/escaping checks, and the full Rust gate including cargo-deny passed. Visual inspection remains blocked by the browser tool reporting disabled after the user enabled access. No merge or release has been claimed yet.

## 2026-10-28 — platform brand assets and font parity

- Added a reproducible generator using the canonical spawnt SVG for ICO, ICNS, Apple, and Linux PNG sizes. Windows setup now uses that generated image, the approved canvas/text palette, plain spawnt branding, and a red progress track. Installation paths and registry identity remain compatible with existing installs.
- Bundled Inter and JetBrains Mono with their licenses for the web and embedded local console, avoiding platform-dependent font substitution and external runtime requests. Moved local view navigation below the heading to match the web console's hierarchy; corrected local help URLs.
- Verification: Windows installer source compiles with embedded canonical mark. Full Rust gate (including cargo-deny) and production web build passed. Platform packages will be recreated after console completion.

## 2026-10-28 — tour usage metric layout

- Replaced the unstyled inline Usage summary with the same divided label/value/trust grid as Overview. Trust labels are small upright mono text; the explanatory caption has its own row. Mixed API and estimated costs now declare EST.
- Verification: Next.js production build passed. Browser visual review could not run because local browser use is disabled in Work settings.
- Follow-up: moved command-copy actions inside their code boxes with a clipboard/check icon and accessible feedback; corrected the remaining CLI prose to spawnt. Installed cargo-deny 0.20.2 and verified advisories, bans, licenses, and sources pass (duplicate-version warnings only).

---

## 2026-10-28 — Codex context compaction accounting

- Provider-wide follow-up: audited all seven adapters; the other adapters already use request snapshots. Added shared-store regression coverage for Claude, Codex, Antigravity, Gemini CLI, Cline/Roo, Cursor, and Aider proving a newer 20k context replaces a previous 190k snapshot. Context remains last-observed until the provider emits new usage; compaction content is never stored.

- Goal: keep Codex's displayed context-window usage aligned with the active context after compaction.
- Outcome: Codex usage records now take `context_tokens` from `last_token_usage.total_tokens`, while cumulative `total_token_usage` remains reserved for session-wide accounting. Existing persisted Codex rows are corrected at query time from their latest turn input/output counts, without a database rewrite. Added an anonymized synthetic rollout fixture demonstrating context dropping from 190k to 20k after `context_compacted` even while cumulative tokens increase.
- Verification: adapter and store regression tests each reproduced the cumulative-token bug before their fixes and passed after. Full Rust quality gate passed with the store compatibility correction; `cargo-deny` was skipped because it is not installed.

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

## 2026-10-26 — Session 7: Product Integrity, Tray Polish & Complete Pro Feature Implementation

### 1. Goals & User Requests
- Complete all audit findings and Pro deliverables:
  - Fix unauthenticated API routes and tighten token security with OS randomness.
  - Fix partial JSONL ingestion truncation and rewritten file detection (Gemini, Cline, Cursor).
  - Fix false "thinking" state across agents (Antigravity turn completion, Codex turn token transitions).
  - Fix cost double-counting on reasoning tokens and correct trust level declarations.
  - Deliver all promised Pro features: Resume Queue executor and arm/disarm UI, Guardrail notifications and fail-open IPC hook (< 30s freshness guarantee), licensed CSV/JSON exports, and license activation UX.
  - Complete adapter coverage for Cline/Roo Code, Cursor, and Aider.
  - Resolve cross-agent project fragmentation via Git worktree and remote canonicalization.
  - Eliminate duplicate tray instances and desktop freezes (named mutex, old PowerShell cleanup, single-instance guard).
  - Invert FleetHUD logo colors while preserving exact original vector geometry.

### 2. Implementation & Root Causes
| Area | Problem / Gap | Resolution |
| :--- | :--- | :--- |
| **Resume Queue (Pro)** | Quota resets required manual resume commands | Implemented `resume_queue` table in SQLite with arm/disarm API (`/api/v1/resume-queue`), UI controls, and daemon background executor (`pro::run_resume_queue`) running non-escalated headless CLI resumes (`claude --resume` / `codex resume`). |
| **Guardrails & Hook (Pro)** | Hook could block agent if FleetHUD was down | Implemented strict fail-open guardrail hook (`fleethud hook`) checking `blocked-sessions.json` with a 30s mtime freshness check. If daemon is down or stale, hook exits 0 immediately. Integration removal handles upgrades by matching binary path suffixes. |
| **Adapters Coverage** | Missing Cline, Cursor, and Aider parsers | Added `cline.rs` (infallible parser for `ui_messages.json` with approval tracking), `cursor.rs` (transcript role & token metrics), and `aider.rs` (append-only JSONL analytics reader). |
| **Agent State Inaccuracies** | Antigravity and Codex remained stuck in `Thinking` | Antigravity parser switches to `Idle` on `step_type == "GENERIC"` or `status == "DONE"`, and `AwaitingApproval` on `ask_question`. Codex switches to `Idle` on token generation completion. |
| **Project Canonicalization** | Git worktrees fragmented single projects into separate folders | Implemented `resolve_git_repo_and_remote` resolving `.git` file pointers and `commondir` back to the parent repository root and remote URL. |
| **Tray & Desktop Stability** | Multiple tray processes running concurrently caused freezes | Added named Mutex `Global\FleetHUD_Tray_Mutex` and killed legacy PowerShell tray processes before launching. Restored high-contrast dark menu and single live poll. |
| **Branding & Assets** | Previous attempt inverted into outline wireframe | Restored exact solid squircle and polyline/rect geometry, inverting colors to crisp white glyph on dark squircle across SVGs, ICOs, and console. |
| **License & Exports (Pro)** | Missing UI and CLI for license management and CSV export | Added `fleethud license` (`status`, `activate`, `remove`), `/api/v1/license` endpoints, `/api/v1/export?format=csv` generator, and Pro & Guardrails console interface. |

### 3. Verification & Quality Gate
- `cargo test --workspace`: 41 tests passed across all crates with 0 failures.
- `cargo clippy --workspace --all-targets -- -D warnings`: 0 warnings, 0 errors.
- `cargo fmt --all -- --check`: Clean formatting across entire workspace.
- `npm run build --prefix web`: Next.js 15.5 production build clean with all 13 routes compiled.
- `csc.exe ... Setup.cs`: Clean .NET Windows GUI executable compilation.

## 2026-10-27 — Session 8: Tray Polish, Icon Proportions & Antigravity State Flapping Fix

### 1. Goals & User Requests
- Fix Desktop Cockpit JavaScript syntax error breaking tab navigation and controls.
- Fix brandmark glyph proportion: scale down oversized mark to approved Option A (0.65x scale) with generous margins.
- Fix tray badge legibility: replace unreadable small badge text with large, high-visibility 18px badge using Arial bold typography.
- Remove tray context menu clutter: delete "Copy Local Bearer Token" and "Pin FleetHUD to Taskbar Tray".
- Modernize tray context menu styling: user explicitly requested a sleek dark theme while enforcing **"keep it square, not rounded"** — implemented `DarkMenuColorTable` with `#0d1117` background, 1px `#30363d` hairline border, and `#21262d` subtle dark hover.
- Fix Antigravity agent state flapping: tool outputs (`GENERIC` step) were incorrectly transitioning session to `Idle` while agent was actively working.
- Recompile native Windows release binary `fleethud.exe`, installer `FleetHUD-Setup.exe`, and distribution zip.

### 2. Implementation & Root Causes
| Area | Problem / Gap | Resolution |
| :--- | :--- | :--- |
| **Cockpit Navigation** | Duplicate closing brace `}` at line 909 broke console script evaluation | Removed rogue brace from `crates/daemon/src/console.html`; verified clean compilation with JS AST/Function parser. |
| **Antigravity State** | Antigravity showed `Idle` mid-task during tool runs | Removed faulty `sess.state = SessionState::Idle` on `GENERIC` steps in `crates/adapters/src/antigravity.rs`. End-of-turn is now properly triggered by empty `tool_calls` on `PLANNER_RESPONSE`. |
| **Brandmark Icon** | Glyph scaled edge-to-edge looked distorted and cramped | Generated visual approval showcase; user approved Option A (0.65x scale). Regenerated `fleethud.ico`, updated SVGs, `Setup.cs` coordinates, and `console.html` header. |
| **Tray Badge Visibility** | Small 7.5pt badge was illegible when downscaled to 16x16 by Windows taskbar | Enlarged badge circle to 18x18 at (13,13), switched font to Arial 9.5pt Bold, and enabled `SingleBitPerPixelGridFit` for sharp sub-pixel contrast. |
| **Tray Context Menu** | WinForms default menu used blue gradient highlights and lacked padding; user requested square theme | Created `DarkMenuColorTable` in `scripts/tray.ps1` with `#0d1117` solid background, 1px `#30363d` hairline border, and `#21262d` dark hover highlight. Strictly maintained square geometry. Removed redundant token/pin items. |
| **Windows Compilation** | `aws-lc-sys` failed on Windows MinGW with undefined `nanosleep64` | Switched `reqwest` in `Cargo.toml` to `native-tls` (SChannel on Windows), eliminating C crypto dependencies and linking cleanly. |
| **Add or Remove Programs** | Windows lacked uninstaller entry in Settings / Control Panel | Added dual-mode uninstaller in `scripts/Setup.cs` registering `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\FleetHUD` with `DisplayName`, `DisplayVersion`, `Publisher`, `DisplayIcon`, and `UninstallString` (`FleetHUD-Uninstall.exe --uninstall`). Cleanly terminates services, removes autostart, cleans PATH, removes shortcuts, and purges files. |
| **Favicon & Web Icons** | Web favicon needed sync with approved 0.65x geometry | Copied approved 0.65x icon to `web/app/favicon.ico`, `web/public/favicon.ico`, and added explicit `icons` metadata in `web/app/layout.tsx`. |

### 3. Verification & Quality Gate
- `cargo test -p fleethud-core -p fleethud-adapters -p fleethud-store`: 32 tests passed with 0 failures.
- `cargo.exe build -p fleethud --release`: Clean native Windows compilation (5.18 MB).
- `npm run build --prefix web`: Next.js 15.5 production build clean with all 13 routes.
- `csc.exe Setup.cs`: Clean .NET Windows GUI installer compilation (`FleetHUD-Setup.exe`, 5.42 MB).
- `Compress-Archive`: Updated `FleetHUD-v1.1.1-windows-x64.zip` (5.08 MB).

## 2026-10-27 — Session 9: Phase 1 Master Plan Foundation, Chat Topic Attribution & Release v1.1.3

### 1. Goals & User Requests
- Execute Phase 1 of the FleetHUD Master Plan (`fleethud_master_plan_fixes_and_context_relay.md`):
  - Fix Web Console context saturation progress bar rendering (100% and >=85% fill rendered transparent instead of red).
  - Implement Desktop Auth Session Pairing UX & Unpair Flow: display paired email status and add 1-click unpair capability in Desktop Cockpit.
  - Implement Session Attribution: display physical machine / cloud device name under Surface column (`WORKSTATION-PC`, `local`).
  - Implement Chat Topic Display (User Request): display conversation / chat topic underneath project name on both Native Cockpit and Web Console to distinguish parallel sessions under the same repository.
  - Fix Windows Uninstaller orphaned browser windows: terminate any Edge app window instances running `--user-data-dir=...\\FleetHUD\\AppProfile` or on `127.0.0.1:41738`.
  - Add Desktop System Notifications: context saturation reserve alert (>=90%) and quota pool reset alert with single-shot debouncing.
- Execute full quality gate, update versions to `v1.1.3`, package standalone Windows installer/zip, and merge to `main`.

### 2. Implementation & Root Causes
| Area | Problem / Gap | Resolution |
| :--- | :--- | :--- |
| **Web Console Bar Color** | Tailwind configuration lacked default numeric color keys (`bg-red-500`, `bg-amber-400`) | Updated `web/app/console/page.tsx` line 624 to use custom palette keys `bg-red` (`#f85149`) and `bg-yellow` (`#d29922`). Saturated bars now render solid high-contrast red. |
| **Auth Session Pairing & Unpair** | Local daemon did not reflect pairing status and lacked an unpair flow | Updated `GET /api/v1/auth/session` to return `{ paired, email }` extracted from `~/.fleethud/session.json`. Added `#auth-paired-card` with active green indicator and `[Unpair / Disconnect]` calling `DELETE /api/v1/auth/session`. |
| **Chat Topic Attribution** | Multiple sessions under the same repository all displayed identical "fleethud" project names | Exposed `title` and `device_name` on `SessionListItem`. Enhanced `reconcile_antigravity_projects` to query and sync titles for all Antigravity sessions from `~/.gemini/antigravity/conversation_summaries.db`. Rendered chat topic stacked cleanly under project name in both Native and Web consoles. |
| **Device Attribution** | Local daemon and Web Console lacked origin machine attribution | Added `fleethud_platform::hostname()` helper and attached `device_name` to session lists in `/api/v1/sessions` and `/api/v1/live`. Displayed device name in muted monospace under the Surface column. |
| **Uninstaller Cleanup** | Running `FleetHUD-Uninstall.exe` terminated the tray process but left the Edge app window open | Updated `KillFleetHUDProcesses()` in `scripts/Setup.cs` to terminate processes matching `FleetHUD\AppProfile` or `127.0.0.1:41738`. |
| **System Notifications** | Users had no alert when reaching 90% context saturation or when rate-limited quotas reset | Added balloon notifications for >=90% context saturation (`[WAIT] Context Saturation Warning`) and quota reset (`[READY] Quota Pool Reset`) in `scripts/tray.ps1` with in-memory single-shot latching. |

### 3. Verification & Quality Gate
- `cargo fmt --all --check`: Clean formatting across entire workspace.
- `cargo clippy --workspace --all-targets -- -D warnings`: 0 warnings, 0 errors.
- `cargo test --workspace`: 47 tests passed across all 9 crates with 0 failures.
- `npm run build --prefix web`: Next.js 15.5 production build verified (14 static routes compiled cleanly).
- Release Packager: Built native release `fleethud.exe`, compiled `FleetHUD-Setup.exe` (5.42 MB), and packaged `FleetHUD-v1.1.3-windows-x64.zip` (5.08 MB).

---

## 2026-10-27 — Session 10: Phase 2 Pro Labs Context Relay Engine, Cross-Platform Alerts & Remote Cloud Handoff (Release v1.2.0)

### 1. Goals & User Requests
- Implement Phase 2 of the FleetHUD Master Plan (`fleethud_master_plan_fixes_and_context_relay.md`) using the Ponytail Ultra methodology:
  - Context Relay Engine: Detect context exhaustion (>=90% saturation or configurable threshold 80-95%), generate zero-knowledge project checkpoints (`.fleethud/HANDOFF.md`), and orchestrate seamless session rotation.
  - Invariant Enforcement: Never store prompt or tool output content (`AGENTS.md` Invariant 1); never escalate agent permissions (`AGENTS.md` Invariant 7, `--dangerously-skip-permissions` forbidden); fail open if daemon is unresponsive.
  - Cross-Platform Desktop Notifications: Centralize native alert dispatching in Rust across Windows (PowerShell balloon), macOS (`osascript` notification), and Linux (`notify-send`).
  - Remote Web Console Handoff: Allow triggering context handoff from mobile/remote web console when away from workstation via Supabase Cloud Sync command queueing, executed locally by desktop daemon.
  - Pro Labs Controls: Provide threshold configuration, handoff strategy (checkpoint + auto-rotate, or checkpoint-only), and execution ledger in both Desktop Cockpit and Web Console.
  - Documentation: Update `docs/SPEC.md` §3.14.1 detailing Pro Labs Context Relay Engine architecture and security buffers.
  - Verification & Release: Run full Rust quality gate, Next.js production build, bump version to `1.2.0` across all 8 repo locations, compile release binaries, and merge via PR.

### 2. Implementation & Root Causes
| Area | Problem / Gap | Resolution |
| :--- | :--- | :--- |
| **Context Relay Engine** | Agent sessions hitting 100% context saturation lose working context, degrade code output, and waste tokens | Built `crates/daemon/src/pro.rs` with `run_context_relay` and `execute_relay_handoff`. Evaluates session saturation on every 5s tick. When threshold is crossed, writes `.fleethud/HANDOFF.md` containing zero-knowledge metadata (repo path, session ID, saturation percentage, model quota status, instruction checklist) and records entry in SQLite. |
| **Cross-Platform Alerts** | Only Windows tray had basic alerts; macOS and Linux lacked native desktop notifications | Implemented `send_desktop_notification(title, body)` in `crates/platform/src/lib.rs` supporting Windows (WinForms balloon), macOS (`osascript -e 'display notification ...'`), and Linux (`notify-send`). Dispatches saturation warnings across all 3 platforms directly from daemon. |
| **Relay Store & State** | Daemon lacked persistent configuration and ledger for context relays | Added `ContextRelayConfig` and `ContextRelayEntry` to `crates/store/src/lib.rs`. Created SQLite tables `context_relay_config` and `context_relay` with CRUD queries (`get_context_relay_config`, `save_context_relay_config`, `list_context_relays`, `record_context_relay`, `update_context_relay_status`). |
| **Daemon Pro API** | Frontends had no endpoints to query relay state or trigger handoffs | Added `/api/v1/pro/relay` (GET config & entries), `/api/v1/pro/relay/config` (POST update), `/api/v1/pro/relay/execute` (POST execute handoff), and `/api/v1/pro/relay/dismiss` (POST dismiss) in `crates/daemon/src/api.rs`. |
| **Remote Web Console Execution** | Remote devices cannot reach loopback `127.0.0.1:41738` to trigger local desktop actions | Extended Supabase cloud sync in `crates/daemon/src/sync.rs`. Included `context_relays` and `relay_config` in cloud snapshots. In `push_snapshot`, the local daemon checks for incoming `relay_commands` in `fleethud_sync_state`, executes the handoff locally on the workstation, and records completion in cloud state. |
| **Web Console UI** | Web console lacked visibility into context saturation reserve and handoff action | Added `[PRO LABS]` Context Saturation Reserve Warning banner (>=90%) with quick action, row-level `[Handoff & Rotate]` button, and `handleRemoteRelay` dispatching to local daemon or queueing via Supabase sync state for remote execution. |
| **Desktop Cockpit UI** | Cockpit needed configuration and ledger visibility | Added Pro Labs Context Relay card to `crates/daemon/src/console.html` with terminal launch explainer, policy/strategy/threshold selectors, pending relays table, and manual `[Handoff & Rotate]` triggers. |
| **Specification** | `docs/SPEC.md` lacked official technical specification for Context Relay | Added §3.14.1 to `docs/SPEC.md` documenting Context Relay triggers, zero-knowledge checkpointing, cross-platform notifications, cloud sync command relay, and security guarantees. |

### 3. Verification & Quality Gate
- `cargo fmt --all --check`: Clean formatting across all crates.
- `cargo clippy --workspace --all-targets -- -D warnings`: 0 warnings, 0 errors.
- `cargo test --workspace`: 49 tests passed across all 9 crates with 0 failures (added `test_context_relay_crud` in `crates/store`).
- `npm run build --prefix web`: Next.js 15.5 production build verified (14 static routes compiled cleanly).
- Version Lockstep: Bumped version to `1.2.0` in `Cargo.toml`, `scripts/Setup.cs`, `web/package.json`, `npm/package.json`, `npm/fleethud/package.json`, `scripts/install.ps1`, `web/public/downloads/install.ps1`, and `web/app/downloads/page.tsx`.
- Release Packager: Compiled native release `fleethud.exe` (5.22 MB), compiled installer `FleetHUD-Setup.exe` (5.47 MB), and packaged `FleetHUD-v1.2.0-windows-x64.zip` (5.12 MB).

---

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
| **Terminal Process Spawning** | Headless `Command::new("claude").spawn()` executed invisibly in the background without a console window and failed to resolve Windows `.cmd` files. | Added `spawn_agent_in_terminal` in `crates/platform/src/lib.rs`. On Windows, launches in an interactive Command Prompt (`cmd /c start "FleetHUD Relay" cmd.exe /k ...`); on macOS, uses AppleScript Terminal tab; on Linux, uses `x-terminal-emulator`. Focuses editor for IDE agents. |
| **Relay Dismiss Latching** | Dismissing a relay set status to `"dismissed"`, but ticker check only skipped `"pending"` and `"executed"`, immediately recreating the alert. | Updated `crates/daemon/src/pro.rs` line 282 to include `r.status == "dismissed"`. |
| **Session Root Resolution** | CWD lookup checked only Antigravity SQLite DB, defaulting CLI agents to daemon root. | Updated `crates/daemon/src/pro.rs`, `sync.rs`, and `api.rs` to query `db.get_session_focus_target(&s.id)` to obtain the canonical `project.repo_root`. |

### 3. Verification & Quality Gate
- `cargo fmt --all --check`: Clean formatting across all crates.
- `cargo clippy --workspace --all-targets -- -D warnings`: 0 warnings, 0 errors.
- `cargo test --workspace`: 49 tests passed across all 9 crates with 0 failures.
- `npm run build --prefix web`: Next.js 15.5 production build verified (14 static routes compiled cleanly).
- Version Lockstep: Bumped version to `1.2.1` in `Cargo.toml`, `scripts/Setup.cs`, `web/package.json`, `npm/package.json`, `npm/fleethud/package.json`, `scripts/install.ps1`, `web/public/downloads/install.ps1`, and `web/app/downloads/page.tsx`.
- Release Packager: Compiled native release `fleethud.exe`, compiled installer `FleetHUD-Setup.exe`, and packaged `FleetHUD-v1.2.1-windows-x64.zip`.
- Live Verification: Daemon reloaded locally; live session API immediately reflected dynamic context window (`158,723 / 128,000`).







## 2026-10-27 — Cloud session collectors and advisory repair

- Goal: expose hosted sessions on all supported operating systems without inventing activity or usage.
- Implemented `cloud-session` CLI with Codex runtime-ID detection and explicit native IDs for Claude Code, Cursor, Antigravity, Gemini CLI, Cline/Roo and Aider. A collector heartbeat is connected presence, never active agent work; it expires after 90 seconds.
- Reused native usage adapters and added strict metadata-only invocation JSONL imports with durable checkpoints, stable deduplication, cache accounting, Rust pricing and unknown-cost handling. Remote payloads omit titles and repository labels/paths. API-equivalent USD is estimated, not a subscription invoice.
- Added account-isolated Supabase cloud-session rows and a security-invoker view, so desktop snapshots cannot overwrite cloud sessions. Added dashboard merging, unknown-data display and per-session cost labels. Cloud summary charts and provider-specific telemetry exporters remain outside this implementation.
- Fixed cargo-deny config by applying its supported private-workspace policy (`publish=false`) while retaining third-party license checks. Updated vulnerable ctutils 0.4.2 to 0.4.4. The patched crate requires Rust 1.87, so workspace MSRV and CI were updated; minimum-version workspace checking passed.
- Validation: full Rust quality gate passed (55 tests, formatting, Clippy, advisories, licenses, source checks and core purity). Embedded Postgres checks passed for migration repeatability, account isolation, metadata requirements, measured usage, server timestamps and dead-collector expiry. Next.js production build passed. A collector for the actual current Codex thread was started and verified through the authenticated local daemon API; usage remains unknown because this host exposes no rollout or token feed.
- Cross-platform review: shared Rust implementation uses Path/PathBuf, existing environment overrides, no new unsafe/process control or platform-specific shell dependency. JSONL handles CRLF; tests use temporary directories. Manual CI now covers Linux, Windows and macOS. Native Windows/macOS execution has not run in this Linux environment.
- Deployment remains pending: apply the v3 Supabase migration, deploy the web client and provision a paired account session securely. No account pairing/admin database credential was available here; no live account data was changed.


## 2026-10-27 — Automatic collection and web deployment

- Corrected manual startup: the daemon now starts cloud presence automatically from detected Codex metadata or explicit provider/runtime identity bindings for every supported agent. It reuses the existing paired account. Standalone collection remains available for metadata usage imports.
- Serialized in-process account refresh between desktop sync and the automatic collector. Tests isolate the pairing path to a temporary directory and verify automatic registration without a manual collector command.
- Updated the feature branch with current main (v1.2.1), preserving its latest-context-token fix, platform locking, license hardening and daemon improvements. Adjusted the newly enabled MSRV-compatible Clippy integer-multiple lint without changing license verification behavior.
- Pushed the feature branch and created draft PR #38. The connected Vercel Git integration reported a successful preview deployment. Production database migration and initial account pairing remain unconfigured here.

### Deployment follow-up

- Confirmed daemon automatic registration against the actual current cloud runtime. The local gate passed 57 tests and Rust 1.87 workspace checking passed after merging v1.2.1.
- Vercel reports the updated branch preview Ready. GitHub CI dispatch was accepted but every job stopped before runner steps because account payments/spending limits blocked execution; Windows/macOS native checks remain pending.
- The user applied the v3 Supabase migration. A read-only schema probe returned permission denied for the new view under anonymous credentials, confirming it exists and does not grant anonymous access. Authenticated account sync cannot be tested without pairing.
- Promoting the web changes through PR #38 and the existing Vercel main-branch deployment integration; production readiness is checked separately from preview status.

## 2026-10-27 — Native release v1.3.0

- Built a new Windows x64 engine and standalone installer containing the cloud collector. Verified PE/assembly version 1.3.0.0 and exact embedded engine/notices hashes. Native CLI version and collector operations passed under Wine 10; an actual Windows GUI upgrade/uninstall has not been exercised here.
- Built Linux x86_64 and ARM64 releases with static OpenSSL using updated Ubuntu 22.04 packages. Version checks passed on Linux and under ARM64 emulation; the ARM64 collector smoke check passed.
- Bundled third-party and native runtime notices, generated SHA-256 manifests and verified all archive payloads. Fixed Unix installation/extraction and tested that a corrupt manifest cannot replace an existing installation. Updated the PowerShell updater to verify the archive and native installer version before launching it.
- Synchronized release versions, public scripts, native payloads and download links. Removed claims for unavailable macOS, npm and .deb/.rpm/AppImage distributions. macOS artifacts remain blocked by the lack of a native runner and GitHub account billing limits.
- The full release Rust quality gate passed. Publication and Vercel deployment are performed through the release PR and GitHub release rather than relabeling an old binary.

## 2026-10-27 — Session 11: Fix Local Date Boundaries, Gemini/Mini Model Substring Collision & Windows App Release

### 1. Goals & User Requests
- Resolve date boundary issue: "Today" filter was dropping nighttime spend and jumping between dates upon restart because it used UTC midnight (`00:00:00+00:00` = 05:00 AM local time).
- Resolve Gemini Flash model attribution: evaluate why Gemini 3.8 Flash (Medium) in Antigravity was showing a saturated red context bar (`148.5k / 128.0k (100%)`) and inflated cost ($0.91).
- Compile fresh native release binary `fleethud.exe`, build `FleetHUD-Setup.exe` with verified installer payload, and push updates.

### 2. Implementation & Root Causes
| Area | Problem / Gap | Resolution |
| :--- | :--- | :--- |
| **Store / Timeline Date Boundaries** | "Today" calculated start-of-day using `Utc::now().date_naive().and_hms_opt(0,0,0)`, which evaluates to UTC midnight (05:00 AM in UTC+5). Work done between 00:00 and 05:00 AM local was grouped under "yesterday", and upon 05:00 AM rollover, "Today" dropped to $0.smth. Explicit `from_ts` was also ignored for "today", "7d", and "30d". | Updated `generate_usage_report_timeline` in `crates/store/src/lib.rs` to compute start-of-day using `chrono::Local::now().date_naive().and_hms_opt(0,0,0)` converted to UTC, and prioritize explicit `from_ts` across all range queries. Added unit test. |
| **Model Matching Collision** | `lookup_model_pricing` matched `lower.contains("mini")` under OpenAI before checking Google Gemini models. Since `"gemini"` contains `"mini"`, `Gemini 3.8 Flash (Medium)` was mistakenly resolved as `OPENAI_MINI` (128k context window, $0.15/$0.075/$0.60 pricing). | Reordered model catalog in `crates/core/src/pricing.rs` so Google Gemini models are resolved before OpenAI reasoning/flagship generic `"mini"` checks. Added unit test assertion. |
| **Console & Web Range Filtering** | `console.html` and `web/app/console/page.tsx` lacked precise local date span display, showing awkward spans like `Today (Oct 26 — Oct 27)`. | Formatted date spans to condense identical local start/end dates (`Today (Oct 27, 2026)`), and passed browser-local start-of-day ISO string in `from` query param. |
| **CLI Range Filter** | `fleethud report` lacked `--range` CLI argument documented in web docs. | Added `--range` option to `Report` subcommand in `crates/cli/src/main.rs` routing to `generate_usage_report_timeline`. |

### 3. Verification & Packaging
- Rust Quality Gate: 57 tests passed across all 9 crates with 0 failures, 0 clippy warnings, clean formatting.
- Native Windows Compilation: Built `target/release/fleethud.exe` (5.3 MB).
- Native Installer: Compiled `FleetHUD-Setup.exe` via `csc.exe` with embedded payload verification passing cleanly.
- Web Production Build: Next.js 15.5 production build verified (all 14 static routes compiled cleanly).
- Live verification: `fleethud report --range today` reports exact local day spend ($28.67) and Gemini Flash 1M context window.

## 2026-10-27 — Session 12: Comprehensive Model Substring Collision Audit & Hardening

### 1. Goals & User Requests
- Audit entire model pricing catalog and matching logic to ensure no other model suffers from false substring collisions like `"gemini" -> "mini"`.
- Prevent generic keywords (`o3`, `pro`, `ultra`, `mini`, `4o`, `gpt-5`) from capturing unrelated models or tools (`turbo3`, `combo3`, `prompt-eval`, `production-app`, `protocol-v1`, `minimax-01`, `ultravox-v0.5`).
- Fix `gpt-5.6-luna` version ordering collision (`"5.6-luna"` contained `"6-luna"`).
- Implement bounded word matching (`has_bounded_word`) in Rust with dot, hyphen, space, and non-alphanumeric delimiter support.
- Add regression test asserting that all 39 catalog models resolve cleanly and 14 potential collision strings evaluate strictly to `None` (Unknown Trust Level, $0 cost).
- Recompile native release binary `fleethud.exe`, rebuild `FleetHUD-Setup.exe` with verified payload, and push updates.

### 2. Implementation & Root Causes
| Area | Collision / Gap | Resolution |
| :--- | :--- | :--- |
| **OpenAI `o3` Catch-all** | `lower.contains("o3")` matched any word containing the letters `o` and `3` (e.g. `turbo3`, `combo3`, `macro3`, `echo3`) and billed them as OpenAI o3 ($30/$120 per Mtok). | Replaced with `has_bounded_word(&lower, "o3")`. |
| **Gemini `pro` Catch-all** | `lower.contains("pro")` at line 655 matched any word containing `pro` (e.g. `prompt-eval`, `production-app`, `protocol-v1`, `profile`, `program`). | Removed unbounded `contains("pro")`. Gemini Pro is matched strictly under `gemini` or `argon`. |
| **OpenAI `5.6-luna` vs `6-luna`** | `lower.contains("6-luna")` was evaluated before `5.6-luna`. Since `"5.6-luna"` contains the substring `"6-luna"`, `5.6-luna` was misattributed to `6-luna`. | Prioritized `5.6-luna` before `6-luna` and added dot-boundary checks in `has_bounded_word`. |
| **Generic `mini` / `nano`** | `lower.contains("mini")` matched `minimax`, `admin-model`, and `gemini`. | Replaced with `lower.contains("4o-mini") \|\| has_bounded_word(&lower, "mini") \|\| has_bounded_word(&lower, "nano")`. |
| **Generic `ultra`** | `lower.contains("ultra")` matched audio/vision tools like `ultravox-v0.5`. | Replaced with `(lower.contains("gemini") && lower.contains("ultra")) \|\| has_bounded_word(&lower, "ultra")`. |

### 3. Verification & Packaging
- Rust Quality Gate: 58 tests passed across all 9 crates with 0 failures, 0 clippy warnings, clean formatting.
- Comprehensive Test: `model_lookup_resolves_all_catalog_families_and_rejects_false_substring_collisions` verified all 39 catalog models and 14 negative test cases.
- Native Windows Installer: `FleetHUD-Setup.exe` compiled and verified with `check-installer-payload.cs`.
- Local Daemon: Restarted with hardened binary; live session context correctly shows `200.5k / 1,000,000 (20.0%)` with healthy green progress bar.

## 2026-10-28 — spawnt landing preview

- Goal: build and show a local landing page using the attached `spawnt_redesign_plan.md`, while preserving the Daytona reference navbar behavior.
- Outcome: replaced the Next.js home route with a spawnt landing preview, including the supported-agent strip, feature cells, interactive console tour, privacy facts, and Free/Pro/Teams teaser. Kept other routes intact.
- Decision: console figures and tour screens are labeled illustrative until the redesigned console exists. Current downloads and pricing routes still carry FleetHUD branding and need their planned redesign in a later phase. Removed a duplicate favicon asset that caused a route conflict.
- Verification: Next.js production build passed; local server and browser review performed in dark and light themes, including scroll navbar, tour tabs, selectable bars and rows, scroll-triggered graph motion, self-hosted agent logos, a tighter final CTA, a typographic footer inspired by footer.design, and device-default theme control moved to the footer.
- Added a Log in nav action (in the mobile menu on narrow screens) and routed the landing's console links through web authentication, since the web console requires a signed-in account to be useful.
- Preserved the user-provided `spawnt_redesign_plan.md` verbatim under `docs/`, rewrote `DESIGN.md` for the current red and near-black palette, and added `docs/spawnt_rollout_plan.md` to reconcile the draft with the repo's separate local and signed-in web consoles. No console implementation was changed in this documentation pass.

## 2026-10-28 — spawnt foundation and local refresh slice

- Goal: establish one token source, record the local/cloud presentation boundary, improve the illustrative landing tour, and add an accurate local console refresh/reconciliation action.
- Outcome: added ADR 0005; Next.js and daemon now consume `web/app/spawnt-tokens.css`. Updated local console palette, responsive overflow, focus treatment, and reduced-motion behavior. Added POST `/api/v1/reconcile`, secured by the existing bearer/Host/Origin validation, with duplicate reconciliation suppression. Added separate local view reload and reconcile controls with progress/result/time labels. The signed-in web console now has a view reload action that refetches existing cloud snapshot/session APIs and explicitly does not claim to trigger daemon synchronization. Improved trust labels where source values are available. Replaced landing hero copy with the requested approved text and made tour screens distinct by view.
- Decision: current native delivery is Rust tray control plus OS scripts and installer; Tauri window migration is a separate scope decision. SPEC now reflects the actual state and marks that migration pending.
- Follow-up: both consoles now expose Overview, Sessions, Usage, Limits, Projects, Automations, and Settings destinations with separate transports. Web route selection updates its URL; local selection uses the page hash. Web Project/Automations unavailable data are stated plainly. Landing hero now uses the requested WINDOWS · MACOS · LINUX eyebrow and message.
- Verification: `npm run build --prefix web` passed; full Rust quality gate passed (cargo-deny unavailable and skipped); 16 daemon tests include missing bearer and DNS-rebinding rejection for reconcile. Browser accessibility tree confirms the updated landing hero, navigation and distinct illustrative view content. Visual desktop/narrow screenshots and installer/native cross-platform verification remain pending. This is a foundation slice, not redesign completion.

## 2026-10-28 — spawnt marketing shell and web console

- Goal: bring the signed-in web console and shared marketing header/footer into the approved spawnt design while retaining the existing cloud data/auth path.
- Outcome: added seven console destinations, dedicated Usage and Limits views, a responsive console shell, updated trust badges, and a system/light/dark footer control. Rebuilt the shared marketing and account headers and footer to match the spawnt visual language. API-equivalent pricing is labeled estimated.
- Verification: `npm run build --prefix web` passed and `git diff --check` was clean. Desktop/narrow browser and keyboard/reduced-motion checks remain pending. This phase does not complete local console, native, icon, or installer redesign.

## 2026-10-28 — spawnt local console destinations

- Goal: align the daemon-served local console navigation and refresh states with the signed-in console while keeping its local API and bearer-token boundary.
- Outcome: regrouped existing report, session, quota, account, settings, and Pro guardrail/relay features into Overview, Sessions, Usage, Limits, Projects, Automations, and Settings. Project rollups are explicitly unavailable because the local console API has no rollup view. Added focus and touch-target styling, local trust badges, connection/error states, and successful-update reporting. Reload now reflects failed data requests; Resync retains its authenticated reconcile call and reports an existing operation as in progress. Fixed a duplicate `url` declaration that prevented the embedded JavaScript from parsing.
- Decision: keep the local HTML renderer and existing Rust data path; no new API or aggregation logic was needed. This PR stacks on the signed-in-console PR.
- Verification: embedded JavaScript syntax and `git diff --check` passed. Rust quality gate, browser keyboard/reduced-motion review, and desktop/narrow two-theme visual review are pending. Native tray, icon assets, and installer redesign are outside this phase and remain required before the rollout is complete.

## 2026-10-28 — canonical spawnt mark correction

- Goal: remove the incorrect outlined bracket mark and align every current web/local console and Windows icon use with the approved solid triangle-and-square geometry.
- Outcome: added `web/public/spawnt-mark.svg` as the web/local console mark source; both console headers and marketing header/footer/tour now use that asset. Removed the incorrect SVG copies, updated browser and local-console icon references, regenerated the website, tray, and download ICOs from the canonical asset, and corrected both Windows installer screen marks.
- Decision: keep `/fleethud.svg` as a daemon compatibility alias that serves the canonical mark; `/spawnt-mark.svg` is the preferred path. No data, auth, or business rules changed.
- Verification: `npm run build --prefix web`, full Rust quality gate, and `git diff --check` passed; cargo-deny is not installed. Inspected the generated mark and desktop landing/local-console headers in Chromium. The updated installer source was reviewed but its native runtime preview and macOS/Linux installer paths are outside this icon-only correction; native tray window design remains rollout work.

## 2026-10-28 — account contrast and square controls

- Goal: make the GitHub sign-in action readable, remove the cloud-sync tag, and remove pill-shaped UI treatments across current web and local console views.
- Outcome: removed the Cloud Sync badge, gave OAuth a bordered surface with theme-aware foreground color, changed marketing release/reference labels to plain mono text, made pricing billing controls square, placed tier labels in their headers, and changed local console button naming to match its existing square shape. Connection indicators are square markers without pill containers.
- Verification: `npm run build --prefix web`, embedded local-console JavaScript parsing, `git diff --check`, and a source search for pill styles passed. The full account and pricing route visual review will be recorded with the consolidated theme check.

## 2026-10-28 — app-wide theme preference

- Goal: make System, Light, and Dark preferences apply consistently to the landing page, marketing/account routes, signed-in console, and local daemon console.
- Outcome: centralized web preference read/apply/cycle behavior, applied the saved choice before paint, exposed the same control in web app navigation and the auth header, kept the landing/footer controls in sync, and added an origin-local control to the daemon console. Replaced hard-coded dark console colors and native-control color-scheme overrides with shared theme tokens.
- Decision: retain separate browser-local preferences for the signed-in site origin and loopback console origin; no preference or agent data leaves its origin.
- Verification: `npm run build --prefix web`, embedded local-console JavaScript parsing, `git diff --check`, pill-source search, and full Rust quality gate passed; cargo-deny is not installed. Chromium visual review confirmed theme cycling on the landing, auth, signed-in console, and local console in light/dark/system modes. Narrow viewport review remains pending; native tray palette remains later rollout work.

## 2026-10-28 — pricing route and plan accuracy

- Goal: bring pricing into the shared spawnt system and align plan cadence and Teams availability with SPEC.
- Outcome: removed the unsupported monthly/annual toggle and false $49 Teams price, displayed the $19 one-time Pro license and planned $12/seat/month Teams tier, replaced the legacy cards with a responsive hairline comparison, and removed literal `&check;` strings. Updated the route metadata and rollout/design status.
- Decision: keep the existing download and sign-in destinations; Teams has no active contact or purchase flow, so its planned tier is presented as unavailable.
- Verification: Next.js production build, `git diff --check`, and desktop Chromium review in dark and light passed; the accessibility tree exposes plan sections, links, and native disclosure controls. Narrow viewport and full keyboard pass remain pending.

### Visible brand copy correction

- Replaced user-facing FleetHUD product-name copy with spawnt across web metadata, pricing, auth, docs, signed-in console, and the daemon console, including Pro license prompts and exported report filenames.
- Kept the existing domain, installer filenames, CLI command, database/schema names, callback key, and `.fleethud` data paths unchanged for compatibility; those are technical identifiers rather than visible product branding.
- Verification: production web build, `git diff --check`, full Rust quality gate (cargo-deny unavailable), and app-source search passed. Desktop theme screenshots confirmed route surfaces follow Light/Dark; narrow and keyboard review remain pending.

## 2026-10-28 — downloads route and platform availability

- Goal: redesign Downloads in the shared spawnt system while keeping published URLs and distinguishing available packages from pending platforms.
- Outcome: organized the verified Windows installer/archive and Linux install/checksum paths into a responsive platform list. Marked macOS unavailable until a signed build exists; included the current release's data-preserving update notes.
- Decision: retain `/downloads/FleetHUD-Setup.exe`, the Windows archive, `/sh`, and checksum URLs; no macOS or npm package claims are shown.
- Verification: Next.js production build, `git diff --check`, and desktop Chromium review in dark and light passed; the accessibility tree exposes the platform sections and exact download links. Narrow viewport and full keyboard review remain pending.
## 2026-10-28 — downloads route and platform availability

- Goal: redesign Downloads in the shared spawnt system while keeping published URLs and distinguishing available packages from pending platforms.
- Outcome: organized the verified Windows installer/archive and Linux install/checksum paths into a responsive platform list. Marked macOS unavailable until a signed build exists; included the current release's data-preserving update notes.
- Decision: retain `/downloads/FleetHUD-Setup.exe`, the Windows archive, `/sh`, and checksum URLs; no macOS or npm package claims are shown.

## 2026-10-28 — landing tour alignment

- Goal: fix crowded KPI labels and inconsistent tour table columns reported in the landing page screenshots.
- Outcome: split each overview metric into a label and value/trust row; set shared table tracks so agent, state, usage, and trust columns line up. Narrow layouts consistently hide the usage value column.
- Verification: Next.js production build and `git diff --check` passed. Desktop landing preview screenshot and accessibility tree reviewed; the tour section still needs a fresh browser visual review after restarting the stale dev server, plus narrow viewport and keyboard review.

## 2026-10-28 — documentation visual system and pricing labels

- Goal: bring the documentation route into the marketing design system and clarify model-family and pricing-basis labels.
- Outcome: removed literal Markdown heading markers, replaced the sidebar/card-heavy monospace presentation with the shared sans-led hierarchy and open rows, and added definitions for Frontier, Tiered, and Exact. Explicitly identified Claude Fable 5/5.1 as Anthropic models, linked to the provider, and added one current provider-sourced Fable 5.1 API rate example.
- Decision: keep rate and quota calculations in Rust; the docs explain trust and catalog terms without reproducing the unverified full pricing table. Distinguish catalog rate basis from live-value trust labels.
- Verification: Next.js production build and `git diff --check` passed. Desktop dark and light renders and the accessibility tree were reviewed, including the corrected labels and Anthropic attribution. Narrow viewport, keyboard, and reduced-motion checks remain pending. `npm run lint --prefix web` could not run because the existing `next lint` script opens an interactive first-time configuration prompt; no lint config was added.

## 2026-10-28 — copyable command snippets

- Goal: make every executable command shown in the app easy to copy without selecting text manually.
- Outcome: added a shared accessible copy control to Windows/Linux install commands, verification and CLI commands in docs, and the download page. The existing standalone Daytona preview already has copy controls.
- Verification: Next.js production build and `git diff --check` passed. Desktop browser review confirmed the buttons and layout in docs/downloads, the accessibility tree names each copy action, and clicking a docs command displayed “Copied”. Narrow viewport and keyboard-only review remain pending.

## 2026-10-28 — platform-selectable downloads

- Goal: replace the crowded three-column installer instructions with the platform-tab interaction shown in the console tour and default to the visitor's platform.
- Outcome: added Windows, macOS, and Linux tabs with browser-platform detection, arrow/Home/End keyboard navigation, focus states, a single focused release panel, and reduced-motion behavior. Kept published URLs and marked macOS unavailable until a verified package exists.
- Verification: production Next.js build and `git diff --check` passed. Chromium accessibility tree confirmed tab roles, selected state, Windows release links, and the macOS unavailable state after selection; desktop light and dark styling were inspected. Narrow viewport and non-Windows platform default require additional review.

## 2026-10-28 — settings route presentation

- Goal: bring Settings into the shared spawnt layout and remove the remaining bracketed cloud-sync navbar control.
- Outcome: replaced nested cards with three open, divided sections; aligned account state, password, subscription costs, and custom rates to square controls and shared typography. Missing price values now show `UNKNOWN` rather than `$0.00`; configured rates are identified as user-entered. The pricing table scrolls horizontally and can receive keyboard focus. Removed the `[Connect Cloud Sync]` header link; account connection remains available in Settings and Auth.
- Follow-up: replaced the filled Connect account button with a restrained text link after visual review.
- Verification: Next.js production build and `git diff --check` passed. Chromium review confirmed the route renders, sections and form controls are exposed accessibly, and the theme is applied. The local daemon is offline in this browser so live settings saves/password updates were not exercised; narrow viewport and keyboard-only review remain pending.

## 2026-10-28 — account route presentation

- Goal: align Auth with the spawnt visual language while retaining GitHub and email sign-in, account creation, password reset, and local-console access.
- Outcome: added keyboard-friendly native mode toggles, clear field labels and focus states, responsive square forms, readable GitHub sign-in, and accessible status/error messaging. OAuth now prevents duplicate clicks and reports a provider-start failure. Existing Supabase calls and callback handling remain in place.
- Verification: Next.js production build and `git diff --check` passed. Chromium review covered light/dark themes, the sign-in/create/reset content states, accessible control names, and the visible GitHub action. No credentials were entered and no external sign-in was submitted; narrow viewport and end-to-end Supabase flows remain pending.

## 2026-10-28 — web console refresh and trust states

- Goal: make manual cloud-view reload status truthful and prevent missing metrics from appearing as exact zero.
- Outcome: reload now reports success only when both cloud reads succeed, prevents duplicate manual refreshes, waits safely for an in-flight poll, and keeps the last successful refresh time on failures. Signed-out users see the control disabled with a sign-in explanation. Missing session token/cost/context values render UNKNOWN; relative times and rounded context percentages show estimated trust. Added keyboard support to expandable provider spend rows and focusable, named table scroll regions.
- Verification: Next.js production build and `git diff --check` passed. Chromium review confirmed the disconnected/signed-out state, disabled reload, unknown KPI state, table region names, and accessible session filters. A signed-in cloud fixture, narrow viewport, and keyboard-only interaction check remain pending.

## 2026-10-28 — web console visual alignment

- Goal: make the signed-in console feel like part of the spawnt marketing product rather than a separate card-heavy interface.
- Outcome: aligned the app navbar to the shared brand mark and token system, replaced boxed data widgets with open divided sections, used the marketing sans-led page hierarchy, and unified theme-aware button/link treatments. Removed square-bracket wrappers from status labels in the console and landing tour.
- Decision: preserve the separate local and cloud data/auth paths and the console's existing view navigation, filters, and refresh semantics.
- Follow-up: enlarged and top-aligned the mark with the title, then combined the disconnected-account and missing-desktop notices into one contextual connection row beneath the heading.
- Follow-up: the connection row is now signed-out only. Shared action buttons across marketing, downloads, Auth, Settings, and console use a consistent square 44 px treatment.
- Verification: `npm run build --prefix web` passed, `git diff --check` was clean, and Chromium desktop renders were reviewed in dark and light themes with the accessibility tree. Narrow viewport, keyboard-only, and reduced-motion checks remain pending; signed-in data was unavailable for this visual review.

## 2026-10-28 — native/local console alignment

- Goal: carry the spawnt console hierarchy into the daemon-served local console and current Windows tray without changing local data or authentication paths.
- Outcome: added the aligned “Your agent fleet” heading, canonical mark, connection status line, local reload/resync actions, arrow-key accessible tab navigation, divided metric sections, and square controls to the daemon console. Removed visible bracket wrappers from local status copy and made the local console’s remaining bordered surfaces square. Updated the Windows tray menu palette, labels, estimated-cost trust marker, and square red session badge; retained the existing data profile and compatibility window detection. Rust tray status and notification strings now use plain labels, and CLI reports use spawnt headings and plain trust labels.
- Decision: do not create a Tauri window during this redesign. The current implementation is daemon HTML plus platform scripts; only the Windows tray script exists here. macOS/Linux tray UI remains future work.
- Verification: full Rust quality gate, PowerShell parser validation, embedded JavaScript syntax check, and `git diff --check` passed; `cargo-deny` is not installed. A temporary loopback-only preview confirmed the desktop layout in dark and light themes, the disconnected `UNKNOWN` state, and keyboard arrow navigation. Narrow visual review remains pending.
## 2026-10-27 — Browser cloud pairing and v1.4.0 recovery

- Goal: remove manual session-file copying, repair the Windows console loading failure, expose cloud presence on native clients and publish working update downloads.
- Root cause reproduced: duplicate `url` declaration in embedded export JavaScript prevented the entire dashboard script from executing. `check-console.mjs` failed on the shipped script and passes with the corrected variable.
- Added isolated browser GitHub PKCE/email login, explicit code comparison and encrypted one-time approval RPCs. User applied the v4 migration; a live anonymous consume probe returned HTTP 200/null, without accessing account data.
- PostgreSQL checks cover repeated migrations, denied anonymous approvals/direct reads, capability verification, one-time consumption and replay rejection. Browser checks exercise the actual console, unauthorized states, pairing initiation/cancel and encrypted independent login transfer; external auth transport is stubbed.
- Native consoles merge remote cloud presence with paging/dedup. Tray separates cloud connectivity from local active work and reports daemon failure. Cloud daemons no longer overwrite desktop summaries. Atomic private credential writes serialize with refresh/disconnect; OS-held daemon locks protect metadata.
- New dependencies: RustCrypto aes-gcm for authenticated WebCrypto-compatible encryption, existing base64 crate as a direct dependency, and existing libc dependency for the Unix flock system call confined to platform code.
- Scope follows Ponytail Ultra: account pairing and reported failures only; no provider billing guesses, hidden-log discovery or generic device-management framework.
- Native Windows GUI and macOS hardware are unavailable in this cloud environment. Windows executable behavior is checked under Wine; macOS packages remain unavailable rather than relabeling old artifacts.

- Verification: 65 Rust tests, Clippy, cargo-deny and MSRV 1.87 passed. Both embedded PostgreSQL migration suites and real-browser email/PKCE approval checks passed. Windows PE/installer payload and PowerShell updater checks passed; Linux/ARM64 versions and Wine Windows collector smoke checks passed. Unix updater rejects corrupt manifests without replacing the existing install. Final Linux x86_64 packaging explicitly selects static OpenSSL to avoid an accidental host shared-library dependency.

## 2026-10-27 — Synthetic browser-auth fixture clarification

GitGuardian flagged the hardcoded JWT header/signature in the browser integration
fixture in PR #44 as a Generic High Entropy Secret. The token is synthetic, uses
an all-zero test account and an invalid signature, and is accepted only by mocked
auth transport. Generate its header and signature from clearly named test values
instead. No real account credential was committed or needs rotating. Production
code and packaged binaries are unchanged. Rerun the browser flow with this fixture.

## 2026-10-27 — Withdraw hosted cloud integration, v1.4.1

- User requested removal of the cloud feature. The latest clarification explicitly preserves older desktop account sync and sign-in.
- Removed cloud runtime identity detection, collector command/startup, metadata usage-file ingestion, cloud session types, remote presence reads and native/web/tray presence presentation. Removed the new browser device pairing command, page and RPC client.
- Retained unrelated security/advisory, pricing/date, embedded-console, credential-permission, daemon-lock and update-integrity fixes. Restored existing manual desktop account pairing while retaining server-side login validation and credential protection; unsafe auth callback URLs stay removed.
- Stopped this workspace’s daemon and standalone collectors, deleted its remote presence row, revoked its independent Supabase login and deleted its credential file. Other desktop accounts were untouched.
- Retired SQL migration creates no new feature schema; the repeatable cleanup drops only hosted presence/pairing objects. Owner must run it in the SQL editor because no admin connection is available here.
- Added checks for retired local pairing endpoints returning 404 and inert historical rows being hidden without deleting data. Disposable PostgreSQL cleanup checks verify desktop sync and Auth survive.
- Updated the environment startup draft to remove cloud collection/pairing instructions; saving the draft does not publish it.
- Replacement native packages are v1.4.1, keeping Windows update ordering correct. Native Windows GUI and macOS hardware remain unavailable in this cloud environment.

- Verification: Rust gate (56 tests, Clippy, cargo-deny), MSRV, production web build and disposable PostgreSQL cleanup checks passed. Browser checks against the packaged daemon verify working local data/auth UI, preserved desktop snapshot reads, ignored legacy cloud rows, no presence fetches and retired pairing API/page returning 404. CLI rejects retired commands and a daemon in this actual cloud runtime creates no cloud-presence row. Live Supabase pairing/view/table probes now return 404; the older desktop sync endpoint still exists.
