# 2026-11-12 - Activity heatmap API

Phase 4 backend of `docs/design/console-overhaul-plan.md` (sections 8.2.1-8.2.3).

## Goals

- Rust-computed activity intensity (`level`, 0..=4) on daily report rows.
- A trailing-year dense activity grid with stats served at `GET /api/v1/activity/heatmap`.

## Outcomes

- `DailyUsage.level` set wherever `chart_pct` is set; `activity_level` is square-root scaled.
- The daily usage SQL was factored into `Database::daily_usage_rows`, used by both the timeline report and `activity_heatmap`, so "tokens including cache exactly once" has one definition.
- `activity_heatmap(today)` builds Monday-aligned UTC days (53 ISO weeks); `heatmap_stats` is pure and counts only days on or after the first recording.
- Handler `activity_heatmap_handler` authenticates first and maps errors to 500 like its neighbours.

## Problems and decisions

- `web/lib/console-presentation.d.ts` has no daily row type, so no `.d.ts` change was needed.
- The heatmap passes an inclusive end bound of `T23:59:59.999Z` on today; `first_recorded` ignores cloud-surface rows like the report does.
- JavaScript, CSS and HTML are unchanged; presenters follow in a later PR.

## Verification

- `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.
- New tests: four `activity_level_*`, eight `heatmap_*` in the store crate, and `activity_heatmap_requires_auth_and_returns_dense_days` in the daemon.
