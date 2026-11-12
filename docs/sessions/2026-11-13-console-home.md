# 2026-11-13 - Console Home board

Phase 5 of `docs/design/console-overhaul-plan.md` (sections 5.6, 5.7, 7.1, 12.4, addendum 15).

## Goals

- Replace the Dashboard overview with a Home board: limit cards, sessions grouped Needs you / Running / Recent, Today tiles and a 12-week activity grid.
- Keep every grouping decision in Rust; JavaScript renders the `home_group` it receives.

## Outcomes

- New shared presenters `renderHomeSessions`, `renderTodayTiles` and their `update*` twins (with `.d.ts`). Rows reuse the existing conversation open path; Recent is capped at 6 with a View all link; empty groups are omitted; zero sessions shows one line and a primary New session button.
- Local console: `#home-limits`, `#home-sessions`, `#home-today`, `#home-activity` replace `#overview-summary`, `#overview-limits`, `#ongoing-sessions`, the Home daily chart and the Home sign-in button (Settings already has Account). Rail label and title read Home, with a house icon; guide text says Home.
- `loadOngoing` now fetches all sessions once (limit 100) and feeds both Home and the Workspace ongoing strip; no second sessions fetch. Today totals use `/api/v1/report?range=today`, at most every 10 seconds. The mini grid reuses the cached heatmap fetch.
- Removed `summaryState`/`refreshSummary` (their only target was the removed summary block). Exported presenters used by `Client.tsx` were kept.
- CSS for tiles, session rows and Home layout from the plan; SPEC 3.16 updated.
- Tests: `web/test/home.test.mjs` (10). Updated `check-*` scripts and the console title assertion in `api.rs`.

## Problems and decisions

- The backend `home_group` was already on main; no Rust change was needed.
- The mini grid draws no cells before the first recording, so a young install shows a small block inside a wide card. Left as designed.
- Provider cards overflowed at phone width: grid tracks used `1fr` (min-content blowout). Fixed with `minmax(0, 1fr)` on `.sp-card-grid` and `.sp-provider-card`.
- Tile numbers wrapped at phone width; they now use the title size and no wrap below 720px.
- Session rows show the agent slug in title case; Rust sends no display name on session rows.
- The phone rail is still a side rail (Phase 6), so Home gets about 280px at 375px width.

## Verification

- Local console from this worktree, temp `GNOMCODE_HOME`, port 41795, real agent logs. Dark and light at 1440, light and dark at 375. No horizontal overflow outside the heatmap scroller after the fixes. Clicking a row opens Workspace with the conversation loaded.
- Gate: cargo fmt, clippy -D warnings, cargo test --workspace, npm ci, npm test, typecheck, build.

## Design review (apple-design, impeccable)

- Fixed (High): phone overflow of provider cards; wrapping tile numbers.
- Open, Medium: mini grid sparse for new installs; limits cards have unequal heights at desktop because collapsed and empty cards differ.
- Open, Low: no keyboard shortcut to jump between groups.
