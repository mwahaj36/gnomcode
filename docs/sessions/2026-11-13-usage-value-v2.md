# 2026-11-13 - Usage subscription value tiles and activity cleanup

## Goals

The subscription value (ROI) card on Usage looked out of place: nested bordered sub-cards, uppercase title, red and green used for money, legacy Tailwind styles. The "Recorded activity" block above the grid had become an almost empty header.

## Outcomes

- New "Subscription value" section (sentence case) with the helper "API-equivalent estimate, not an invoice", a quiet "Manage plans" link, and one `sp-tile` per provider: name, big multiple with an `est.` label, meta "$1,423.94 value · $53.33 plan". Thousands separators and tabular figures.
- A neutral highlighted "All providers" tile is shown only when more than one provider has usage (with one provider it would repeat that tile).
- Shared renderer `renderValue`/`updateValue` in `web/lib/console-presentation.js` serves both the local console (`#usage-value`) and the web console (`ValuePanel`). Rust fields `roi_by_provider` and `combined_roi` are only formatted, not computed.
- `renderActivity(report, { totals: false })` no longer prints the "Recorded activity" heading block. It prints one note line (period, interval, date window, days in UTC) and keeps the "Activity details" disclosure under the grid. Home and tour renderers (`totals: true`, hourly) are unchanged.
- Phone: tiles are two columns up to 720px and one column at 420px and below. Red and green are no longer used for money.
- CSS is in a new labeled section at the end of `web/app/gnomcode-console.css`.

## Problems and root causes

- `money` was already declared further down `console-presentation.js`; the new formatter is named `usd`.
- The local console needs a bearer token from the app window, so the browser pane showed "Local authorization is missing". The new markup was verified by injecting synthetic rows through `updateValue` on the live local console page plus a static harness with synthetic data in light and dark at 1440 and phone widths. The web console needs a Supabase session, so it shares the same renderer and CSS and was checked through the harness.

## Decisions

- One quiet text link instead of a button for "Manage plans"; it routes to Settings through the existing `view:settings` action (local) or `changeConsoleView` (web).
- No SPEC or ADR change: presentation only.

## Verification

- `npm test` (81 pass, includes a new `renderValue` test), `npm run typecheck`, `npm run build` in `web/`.
- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` with `CARGO_INCREMENTAL=0`.
