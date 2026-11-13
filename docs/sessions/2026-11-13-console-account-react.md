# 2026-11-13 - Account view in React (roadmap E1, step 4)

## Goals
- Move account sign-in, token pairing, connection status and disconnect from `console-app.js` into `web/local-console/AccountView.tsx` with identical behavior and copy.

## Result
- `AccountView.tsx` owns `#view-auth`: connection and publication status from `GET /api/v1/auth/session` and `/api/v1/sync/status`, sign-in (always behind the account-access consent dialog, `remote_access_confirmed: true`), pairing from JSON, base64 JSON or a bare token (masked until revealed), duplicate submissions ignored, disconnect behind a confirmation with a dismissible notice on failure. The bearer token still travels only in the `Authorization` header.
- The rail's "Sign in" entry is toggled by a `gnomcode:account-state` event; `console-app.js` keeps three lines for it until the shell port. The transitional `window.__gnomcodeShell` bridge from the dialogs PR is removed: the account view imports `confirmAction` and `showNotice` directly.
- `local-api.ts` gains `LocalApiError` (carries the HTTP status) so views can tell "the daemon refused" from "no answer" without parsing messages.
- `console-app.js` 371 -> 236 lines. The shell's refresh on the account view now probes the daemon like every other view; the account view re-checks itself on reload.
- Rendered review (1440 and 390, dark and light) found the sign-in fields shrunk to content width with the button beside the password field. A scoped rule now makes them fill the column with the button below; a browser test pins it.

## Tests
- 69 -> 73 browser tests: bare token and base64 JSON pairing, refused pairing keeps the token and shows red text, sign-in layout.

## Verification
- `CARGO_INCREMENTAL=0` fmt, clippy `-D warnings`, `cargo test --workspace`; web `npm ci`, `npm test`, typecheck, build, `npm run test:e2e`.
