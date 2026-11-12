# 2026-11-13 - Console activity grid on Usage

Phase 4 frontend of `docs/design/console-overhaul-plan.md` (steps 4-10; section 5.8).

## Goals

- Replace the daily bar chart with the GitHub-style token activity grid on Usage.
- Keep all level, streak and statistic math in Rust; JavaScript only lays out cells and formats text.

## Outcomes

- New shared presenters `renderHeatmap`, `renderHourStrip` and `updateHeatmap` (with `.d.ts`). The `weeks: 12` mini variant is ready for Home.
- `renderActivity` no longer draws bars; the Today period draws the 24-hour strip, other periods draw nothing in their place. Totals and the `sp-activity-breakdown` disclosure are unchanged.
- Local console: `#usage-heatmap` at the top of Usage, `loadHeatmap()` with a 60 second cache (not on the 2-second poll, forced by Reload view), scroll to today, and one delegated click handler that opens the matching day detail when the period lists it.
- CSS from section 5.8 using the existing `--sp-heat-*` tokens. Dead bar-chart CSS was removed.
- Tests: `web/test/heatmap.test.mjs`, updated `calm-presentation.test.mjs`, and `scripts/check-console-presentation.mjs` now splits the hourly header at the strip instead of the removed bars.

## Problems and decisions

- Usage has no Limits section yet (Phase 6), so the grid sits first on the page rather than after limits.
- Month labels are positioned with a `--col` custom property (as in the mockup) rather than a CSS grid column, and a label is dropped when it would collide with the next, or when its column is entirely before the first recording.
- First render showed the scroller 32px wider than its content because of the month row margin; the month row no longer forces a width.
- Home is not wired in this PR (Home board is Phase 5); only Usage fetches the grid. The web console keeps `renderActivity` and gets no grid because sync carries no heatmap data (section 14).
- The disk filled during the test run; build outputs of this worktree were deleted and `cargo test` re-run with `CARGO_INCREMENTAL=0`.

## Verification

- Local console from this worktree with a temporary `GNOMCODE_HOME` on port 41790. It read real agent logs, so the grid showed six active days. A synthetic year (ramping usage, gaps, first recording in March) was rendered through the real module and CSS at 1440 dark and light, and 390 dark. No page scroll at 390; today sits at the right edge.
- Light theme contrast: grid text 4.94:1, stat numbers 18.9:1. Heat level 1 against the card is 1.41:1 (token ramp from the plan, noted below).
- Gate: `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm test` (56 pass), `npm run typecheck`, `npm run build`.

## Design review (apple-design charts, impeccable)

- Fixed: labels floating over hidden pre-history columns; sideways overflow at desktop width; stats wrapping 3+1 on phones (now 2x2).
- Open, Medium: cells are not individually focusable (plan keeps keyboard navigation out of scope); the grid summary is in `aria-label` and per-day data is in the Activity details disclosure.
- Open, Medium: light level 0/1 cells have low non-text contrast against the card (1.14 and 1.41). The ramp comes from the plan tokens; revisit in Phase 7.
- Open, Low: Mon/Wed/Fri labels scroll away on phones.
