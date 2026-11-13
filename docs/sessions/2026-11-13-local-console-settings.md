# 2026-11-13 - E1 Local console React Settings view and shell bridge

## Goals
- Replace the Settings view (`#view-settings`) in the local console with React (`web/local-console/SettingsView.tsx`) mounted on `#local-settings-root` and compiled into `crates/daemon/src/console-react.js`.
- Keep all business logic out of React (delegate provider list formatting to `web/lib/console-presentation.js` and read/write via local daemon endpoints).
- Expose `window.__gnomcodeLocalActions` in `crates/daemon/src/console-app.js` so React views can trigger navigation, session deep-links, and onboarding cleanly while keeping `console-app.js` guarded as a shell bridge.

## Outcomes
- Added `web/local-console/SettingsView.tsx` covering Providers, Appearance, Devices & Remote, Automations (resume queue, guardrail rules, alerts, rotation relays), Notifications, Pricing & DB stats, Offline License management, Data Reload/Resync, Account link, Desktop updates, and Guide/Getting started.
- Updated `web/local-console/main.tsx` and `web/local-console/local-api.ts` to mount `<SettingsView />`, set `window.__gnomcodeReactViews.settings = true`, and hide `#legacy-settings-content`.
- Updated `crates/daemon/src/console.html` to add `#local-settings-root` and wrap legacy settings DOM inside `#legacy-settings-content` (preserving static HTML IDs expected by existing test scripts until `console-app.js` / legacy markup retirement).
- Guarded legacy `console-app.js` (`setView`, `refreshCurrentView`, `loadProData`, `loadSettingsData`, `loadRemoteStatus`, `loadQuota` `#settings-provider-list`) and exposed `window.__gnomcodeLocalActions` + `gnomcode:theme-change` event dispatch in `cycleTheme`.
- Added `.sp-settings-root` layout/form rules to `web/app/gnomcode-console.css`.

## Assumption & Deviation
- Physical deletion of `crates/daemon/src/console-app.js` and legacy markup would break compilation because `crates/daemon/src/api.rs:121`, `api.rs:680-687`, and `api.rs:3636-3638` compile-time embed `include_str!("console-app.js")` and serve `/console-app.js` (and `api.rs` only allows append-style edits from Worker B). Therefore, as a clearly marked assumption, `console-app.js` is retained as a minimal guarded shell/navigation shim while all 4 views (Home, Workspace, Usage, Settings) now run in React, and final deletion awaits owner cleanup of those `api.rs` route lines.

## Verification
- `node scripts/build-local-console.mjs` & `--check`
- `node scripts/check-console.mjs` & `node scripts/check-local-console.mjs`
- `node --test web/test/local-console-settings.test.mjs`
- `web/` (`npm test`, `npm run typecheck`, `npm run lint`, `npm run build`)
- Chromium CDP verification against `/#settings` at `1440x900` and `390x844` (dark & light) with `overflowX: false`
- Rust workspace gate (`CARGO_INCREMENTAL=0`: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`)
