# 2026-11-13 - Playwright browser tests for the local console (roadmap E1, E4 last item)

## Goals
- Pin the current shell behavior (navigation, redirects, polling status, tour, theme, account, approvals, phone width) with real browser tests before `console-app.js` is ported to React, so the port can prove "behavior identical".
- Start the real daemon from a temp `GNOMCODE_HOME` on a free loopback port with fixture data; no mocks for the daemon itself.

## What was added
- `@playwright/test` 1.64.1 (exact pin, released 2026-10-07) as a `web` devDependency. License is Apache-2.0 (permissive; the brief said MIT). Dev-only: it is not bundled into `console-react.js`, the site or the desktop app. Browsers are downloaded by `npx playwright install chromium`, never committed.
- `web/playwright.config.ts`, `web/e2e/daemon.ts` (spawns `gnomcode daemon --port <free>` with `GNOMCODE_HOME`, `CLAUDE_CONFIG_DIR`, `CODEX_HOME` pointed at a temp dir, seeded from `fixtures/claude-code/v2.1` and `fixtures/codex/v0.162`, waits for `daemon.json` and `/health`), `web/e2e/fixtures.ts` (one daemon per worker, `openConsole()` opens the tokened launch URL the way the tray does).
- Specs: `shell` (rail, aliases `#limits`/`#projects`/`#automations`, back/forward, roving keys, rail toggle, last view, connection status, stale state, token never in API URLs), `views` (Home, Workspace, Usage, Settings), `approvals` (real hook round trip: POST `/api/v1/approvals`, card, Approve once/Deny, exactly one decision, text-only rendering, 44px on phone), `tour` (tip, dialog, back, skip, Escape, empty-history auto open), `account` (consent dialog, sign-in, token pairing, disconnect, web console link), `phone` (390px: no sideways scroll, 44px targets, list collapsed).
- `npm run test:e2e`; CI job `console-e2e` on ubuntu and windows (builds the debug daemon, installs Chromium, runs the suite, uploads traces on failure).

## Findings
- The tour's last step ("Open session launcher") focuses `#launch-agent-select`, which only exists in the dead legacy markup, and never opens the React launcher. Captured as `test.fixme`; the shell port fixes it.
- Legacy Workspace/Settings markup stays in the DOM (hidden) and duplicates ids and labels, so tests scope to `#local-*-root`. The port removes the duplicates.

## Verification
- `npm run test:e2e` 49 passed, 1 fixme (known bug above); web `npm test`, `npm run typecheck`, `npm run lint` pass. No Rust or runtime web change in this PR, so the Rust gate is unchanged.
