# 2026-11-13 - Notice, tour and confirmation dialogs in React (roadmap E1, step 3)

## Goals
- Move the app notice, first-run tip, four-step tour and confirmation dialog from `console-app.js` into `web/local-console/ShellDialogs.tsx`, keeping the copy, step order, focus handling and storage behavior.

## Result
- `ShellDialogs.tsx` renders the same ids and classes (`#app-notice`, `#tour-tip`, `#onboarding-dialog`, `#confirm-dialog`) with native `<dialog>` + `showModal()`, so focus trapping and Escape work as before. Escape and Skip finish the tour and return focus to the opener; the last step opens the Workspace launcher and focuses its Agent field. An empty history opens the tour by itself, otherwise a dismissible tip is shown once; blocked storage never throws.
- `shell-bus.ts` gives the React code `confirmAction`, `showNotice` and `startOnboarding` (Settings "Getting started" calls it directly instead of through `window.__gnomcodeLocalActions`). `window.__gnomcodeShell` is a transitional bridge so the account code still in `console-app.js` can confirm and notify; it goes away with the account port.
- `console-app.js` 461 -> 371 lines: onboarding, notice, confirm, the `onboarding-*` and `dismiss-*` actions and the offer check are gone. `console.html` carries one `#shell-dialogs-root`.

## Tests
- All 68 browser tests pass unchanged; added a 69th for Escape cancelling a confirmation and the dismissible notice after a failed disconnect.

## Verification
- `CARGO_INCREMENTAL=0` fmt, clippy `-D warnings`, `cargo test --workspace`; web `npm ci`, `npm test`, typecheck, build, `npm run test:e2e`. Rendered the tour at 1440 and 390, dark and light.
