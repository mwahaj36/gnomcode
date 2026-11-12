# 2026-11-13 — Artifacts viewer alongside changes (UI)

## Goals

Add read-only inspection of session artifacts (images, Markdown, HTML) alongside workspace changes in both the local console chat view and the encrypted Remote mobile companion, keeping HTML completely sandboxed from the host app.

## Outcomes

- Added `classifyArtifactKind`, `isSafeArtifactImageSrc`, `renderArtifactsList`, and `renderArtifactPreview` (plus TypeScript definitions) in `web/lib/console-presentation.js` and `.d.ts`.
- HTML artifacts render strictly inside `<iframe sandbox="" referrerpolicy="no-referrer" srcdoc="...">` with zero script, same-origin, form, or popup tokens.
- Markdown artifacts reuse the existing sanitized `MarkdownIt({ html: false, breaks: true, linkify: false })` chat pipeline; image sources reject `javascript:`, `data:text/html`, and arbitrary external origins.
- Wired `#chat-artifacts-btn` and `#chat-artifacts-panel` into `crates/daemon/src/console.html` and `console-app.js` using `GET /api/v1/sessions/{id}/artifacts` + workspace change count / fallback from `GET /api/v1/repos/diff` and `/api/v1/repos/file`.
- Wired `Artifacts` toolbar button and `.sp-artifacts-sheet` dialog into `web/components/RemoteCompanion.tsx` using E2EE `{ type: 'artifacts', session_id }` on demand with calm `Unsupported remote command` handling when the daemon backend handler is not yet present.
- Added token-based styling (`var(--sp-*)`) in `web/app/gnomcode-console.css`, responsive layout at 390px and 1440px (dark and light), 44px mobile touch targets, and hidden secondary toolbar under `max-height: 450px`.

## Decisions & Verification

- Used empty `sandbox=""` instead of allowing scripts inside HTML previews so agent-generated HTML cannot access cookies, DOM, E2EE keys, or parent windows.
- Kept backend ownership untouched (Worker B scope): local fallback reads changed markdown/HTML files via existing `/api/v1/repos/file`, while dedicated session artifact capture and Remote E2EE parity await Worker A's `C last-turn artifacts` PR.
- Verified with `web/test/artifacts-view.test.mjs`, `web/` tests, `tsc`, Next build, `check-local-console.mjs`, headless Chromium visual checks (`390x844` and `1440x900` dark/light), and full `CARGO_INCREMENTAL=0` Rust workspace checks.
