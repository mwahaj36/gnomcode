# 2026-11-13 - E1 Local Console React Workspace View

## Goals

Replace the legacy Workspace (`#sessions`) UI in `crates/daemon/src/console-app.js` with a React component (`web/local-console/WorkspaceView.tsx`) compiled into the daemon's embedded zero-sourcemap `/console-react.js` bundle, while keeping legacy static markup in `crates/daemon/src/console.html` intact for existing static HTML checks until the final E1 PR retires `console-app.js`.

## Outcomes

- Extended `web/local-console/local-api.ts` with typed mutation helper `fetchLocalMutation`, desktop bridge types (`chooseWorkspace`, `restartToUpdate`), and `xterm.js` / `FitAddon` globals.
- Implemented `web/local-console/WorkspaceView.tsx` covering sidebar search/source/project/agent/status filters, pagination, agent launcher + folder browser dialog, chat transcript with optimistic reconciliation and permission actions, imported session banner, model switcher, inline agent/workspace PTY WebSockets, and schedule/reset automation.
- Delegates pure rendering and reconciliation to `web/lib/console-presentation.js` (`renderSessionList`, `reconcileChat`, `updateChat`, `renderConversationLimits`) with zero business logic in React.
- Mounted `WorkspaceView` onto `#local-workspace-root` (`window.__gnomcodeReactViews.sessions = true`) in `web/local-console/main.tsx`, hiding `#legacy-ide-layout` at runtime without breaking static `check-local-console.mjs` assertions.
- Rebuilt `crates/daemon/src/console-react.js` deterministically (`381,404` bytes, zero `sourceMappingURL`).

## Problems Encountered & Root-Cause Fixes

1. **CSS hex color gate failure:** `.sp-workspace-terminal-screen` initially used literal `#0d0d0f`, violating `web/test/` color token discipline (`no literal hex colors after line 800`). Fixed by replacing it with `var(--sp-surface)`.
2. **Mobile 390px conversation heading wrapping:** At `390x844`, `div:first-child` in `.conversation-heading` inherited `flex: 1 1 0` beside `Resume conversation`, `Terminal`, and `Actions`, shrinking the title to 2 characters per line (`overflow-wrap: anywhere`). Fixed by scoping `.sp-workspace-chat-panel .conversation-heading > div:first-child { flex: 1 1 100%; min-width: 0; }` under `@media (max-width: 700px)`.

## Verification

- `node scripts/build-local-console.mjs --check`
- `node scripts/check-console.mjs`
- `node scripts/check-local-console.mjs`
- `web/` (`npm test`, `npm run typecheck`, `npm run lint`, `npm run build`) including `web/test/local-console-workspace.test.mjs`
- Rust gate (`CARGO_INCREMENTAL=0`: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`)
- Headless Chromium visual check at `1440x900` and `390x844` (dark & light) confirming React Workspace mount, `#legacy-ide-layout` hidden, zero horizontal overflow, clean mobile title wrapping, and chat message rendering.
