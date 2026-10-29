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
| `irm` and `curl` 404 on `raw.githubusercontent.com` | `spawnt` is a private repository; raw GitHub URLs require private authentication | Moved install scripts and release packages directly to Next.js public routes (`/win` -> `/downloads/install.ps1`, `/sh` -> `/downloads/install.sh`), removing all private GitHub URLs from web headers, footers, and docs. |
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

