# 2026-11-13 - Shell in React, console-app.js deleted (roadmap E1, final step)

## Goals
- Port what was left of `console-app.js` (view routing, rail, connection status, delegated clicks, bridge) to React/TS in `web/`, then delete the script, its route and its `include_str!`, and reduce `console.html` to one root element. No business logic in TypeScript; bearer token handling unchanged (header only, never in an API URL).

## Result
- `ConsoleShell.tsx` renders the rail, title, connection status, the four view roots, the approvals card, the account view and the dialogs in one tree. It keeps the contracts the views already use: `body[data-console-view]`, `body[data-rail]`, `gnomcode:view-change` and `gnomcode:reload-view` events, `#local-*-root` ids and the old-address redirects (`#limits`, `#projects` -> Usage, `#automations` -> Settings, rewritten in place). Back and forward, arrow/Home/End roving focus, the Explorer deep link and the daemon probe (`sessions?limit=1` and `providers/overview` every 2 s, status "Local daemon connected", "Local data stale", "Local data unavailable") behave as before.
- `shell-bus.ts` replaces `window.__gnomcodeLocalActions`: `navigateTo`, `workspaceAction` (queued until the Workspace view reports ready) and the dialog helpers. `chat-list.ts` is the single owner of the conversation-list collapsed state (Workspace and the header toggle both used to write the body class separately). `local-api.ts` gets `getConsoleBaseUrl()`.
- Deleted `crates/daemon/src/console-app.js` (was 2,586 lines at the start of E1) and the `/console-app.js` route and `include_str!` in `api.rs`; `console.html` is 16 lines (head, one `#gnomcode-console-root` carrying the injected base URL, the vendored xterm scripts). The embedded-console Rust tests now assert the root element and the absence of the old script.
- Behavior change on purpose: the rail's Sign in entry was given `tabindex="-1"` by the old script after the first navigation, so keyboard users could not reach it; it is a normal focusable button now.
- Release packaging runs the browser suite against the release binary (previous PR), so no script check remains tied to the deleted file.

## Tests
- 73 -> 76 browser tests (keyboard reachability and hash navigation; single React root, `/console-app.js` returns 404; Home activity grid spacing). The rendered review found the Home "Last 12 weeks" heading touching the Today tiles; it now has the standard section gap.

## Verification
- `CARGO_INCREMENTAL=0` fmt, clippy `-D warnings`, `cargo test --workspace`; web `npm ci`, `npm test`, typecheck, build, `npm run test:e2e`. Rendered 1440 and 390, dark and light.
