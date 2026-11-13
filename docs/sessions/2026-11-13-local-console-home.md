# 2026-11-13 — E1 Local Console React Home View (Phase 1)

## Goals

Begin Section E1 ("Local console built from `web/` React; delete `console-app.js`") incrementally with the **Home** view (`overview`) built from `web/local-console/`, bundled deterministically without sourcemaps, served by `crates/daemon`, and keeping Electron release DevTools and sourcemaps disabled. Keep existing views and `scripts/check-*.mjs` checks green until the remaining Workspace, Usage, and Settings PRs complete the cutover.

## Outcomes

- Added `scripts/build-local-console.mjs` and `scripts/local-console-loader.cjs` compiling `web/local-console/main.tsx` into `crates/daemon/src/console-react.js` using `next`'s bundled `webpack` (`mode: 'production'`, `devtool: false`, `nodeEnv: 'production'`) and `terser` (`sourceMap: false`, `comments: false`), with `--check` enforced in `web/package.json` `prebuild`.
- Added `crates/daemon/src/api/console_react.rs` and append-only `mod console_react;` + `.route("/console-react.js", ...)` in `crates/daemon/src/api.rs`.
- Implemented `web/local-console/local-api.ts`, `web/local-console/HomeView.tsx`, and `web/local-console/main.tsx`, mounting React onto `#local-home-root` and delegating all card, session-group, Today tile, and mini-heatmap rendering to `web/lib/console-presentation.js` (zero business logic in React).
- Kept fallback IDs (`#home-limits`, `#home-sessions`, `#home-today`, `#home-activity`) and deferred Home DOM writes in `console-app.js` when `window.__gnomcodeReactViews.overview` is active.
- Explicitly disabled `sourceMap`, `inlineSourceMap`, and `declarationMap` in `apps/desktop/tsconfig.json` and added `apps/desktop/test/release-hygiene.test.cjs` verifying `dist/` contains no `.map` / `sourceMappingURL` and `src/main.ts` retains `devTools: !app.isPackaged`.

## Verification

- `node scripts/build-local-console.mjs --check`, `scripts/check-console.mjs`, `scripts/check-local-console.mjs`, `scripts/check-console-presentation.mjs`
- `web/`: `npm test`, `npm run typecheck`, `npm run lint`, `npm run build`
- `apps/desktop/`: `npm test` (`40/40`), `npm run typecheck`, `npm run build`
- Chromium headless verification at `390x844` and `1440x900` (dark & light)
- Rust workspace gate (`CARGO_INCREMENTAL=0`): `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
