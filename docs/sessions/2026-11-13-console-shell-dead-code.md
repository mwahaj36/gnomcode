# 2026-11-13 - Remove the dead legacy console code (roadmap E1, step 1)

## Goals
- Cut `console-app.js` and `console.html` down to the shell that is actually alive. Home, Workspace, Usage and Settings have been React views for several PRs; the legacy chat, launcher, usage, settings, remote, update and pricing code behind `window.__gnomcodeReactViews` guards never did anything visible, and the legacy markup was hidden but still in the DOM (duplicate ids and labels).
- Prove with the Playwright suite from the previous PR that nothing user-visible changed.

## Result
- `console-app.js` 2,586 -> 496 lines; `console.html` 397 -> 72 lines. What is left in the script: view routing and aliases, rail and conversation-list toggles, the connection probe, the first-run tour, account sign-in and pairing, tool approvals, delegated clicks for presenter markup, and the `window.__gnomcodeLocalActions` bridge.
- `main.tsx` mounts the four views without hiding legacy nodes; `__gnomcodeReactViews` is gone. Settings "Getting started" calls `__gnomcodeLocalActions.startOnboarding()` instead of clicking a hidden `[data-action]` element.
- The shell's connection probe asks the daemon for one session and the provider overview instead of re-running the legacy loaders (sessions x2, report, quota). The status text, error state and 2 s cadence are unchanged.

## Bugs the new tests found and this PR fixes
- The tour's last step called a legacy launcher that no longer exists and never opened the React launcher. It now opens the launcher and focuses its Agent field.
- The Explorer deep link `#sessions?folder=...` ran before the deferred React bundle mounted, so the folder was dropped. Workspace actions that arrive early are queued until the Workspace view reports ready.

## Tests
- New: `web/e2e/workspace.spec.ts` (Files, Changes, Artifacts panels; quick prompts, Attach, Interrupt on a conversation presented as managed), more shell, tour and phone cases. 68 browser tests in total.
- Replaced `scripts/check-console-presentation.mjs` with `web/test/console-presentation.test.mjs` (same assertions as `node:test`), `scripts/check-tour-density.mjs` with `web/test/console-tour-density.test.mjs`, and `scripts/check-settings-saves.mjs` (failing: stale hook indexes and a missing module mock) with `web/test/settings-saves.test.mjs`.
- Deleted with their assertions covered by e2e: `check-console.mjs` (script compile), `check-local-console.mjs`, `check-native-navigation.mjs` (failing: referenced a removed helper), `check-onboarding.mjs`, `check-app-controls.mjs`.
- Deleted `check-cloud-removal.mjs` (failing): it needs a manually installed PGlite and a historical commit blob, so it could never run in CI; the migration it exercised has shipped.
- Web tests that read `console-app.js` or `console.html` were rewritten or removed: the Remote half of each wiring test stays, the local half is now in `workspace.spec.ts`. `audit-regressions`, `local-console-*` source greps and the legacy halves of `nav` and `web-console-v2` are gone; a single `local-console-bundle.test.mjs` keeps the no-sourcemap and no-reporting-math contracts.
- `package-release.ps1` runs the browser suite against the release binary instead of the removed script checks.

## Verification
- `CARGO_INCREMENTAL=0`: cargo fmt, clippy `-D warnings`, `cargo test --workspace`; web `npm ci`, `npm test`, `npm run typecheck`, `npm run build`, `npm run test:e2e`.
