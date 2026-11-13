# 2026-11-13 - E1 Local Console React Usage View

## Goals

Replace the legacy Usage (`#usage`) view in `crates/daemon/src/console-app.js` with `web/local-console/UsageView.tsx` inside the embedded zero-sourcemap `/console-react.js` bundle, keeping legacy static HTML targets in `crates/daemon/src/console.html` intact for existing static checks until the final E1 PR retires `console-app.js`.

## Outcomes

- Added `web/local-console/UsageView.tsx` and mounted it on `#local-usage-root` (`window.__gnomcodeReactViews.usage = true`), hiding `[data-legacy-usage="true"]` elements at runtime.
- Reused `web/lib/console-presentation.js` presenters (`updateProviderCards`, `updateHeatmap`, `updateActivity`, `updateProviders`, `updateValue`, `mountProjectList`) with zero business logic in React.
- Wired reporting period selection (`today`, `month`, `7d`, `30d`, `all`), heatmap cell-to-day scroll interaction, and `/api/v1/export` CSV/JSON download.
- Added `web/test/local-console-usage.test.mjs` and rebuilt `crates/daemon/src/console-react.js` (`397,875` bytes, zero `sourceMappingURL`).

## Verification

- `node scripts/build-local-console.mjs --check`
- `node scripts/check-console.mjs`
- `node scripts/check-local-console.mjs`
- `web/` (`npm test` `103/103`, `npm run typecheck`, `npm run lint`, `npm run build`)
- Rust (`CARGO_INCREMENTAL=0`: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`)
- Headless Chromium visual check at `1440x900` and `390x844` (dark & light) confirming unobscured render, zero horizontal overflow, clean spacing, quota cards, heatmap, and project list.
