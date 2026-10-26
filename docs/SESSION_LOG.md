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

### 4. Outcomes & Verification
- **Commit `61f3320`:** Initialized Cargo workspace, `docs/SPEC.md`, `AGENTS.md`, ADRs `0001`–`0003`, 16 engineering skills, `SECURITY.md`, `CHANGELOG.md`, and issue templates.
- **Commit `fc60078`:** Removed all emojis across repo and GitHub metadata; added Invariant 9 to `AGENTS.md`.
- **Commit `5d66efe`:** Updated `docs/SPEC.md` with all user decisions on metrics, retention, backfill, accounts, and settings.
- **Commit 4 (in progress):** Switch CI workflow to manual `workflow_dispatch`, add `docs/SESSION_LOG.md`, and generate anonymized golden fixtures (`fixtures/`) for M0 completion.
