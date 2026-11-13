# 2026-11-13 - Tool approvals card in React (roadmap E1, step 2)

## Goals
- Move the pending-approvals card out of `console-app.js` into `web/local-console/ApprovalsCard.tsx`, with identical behavior: poll `GET /api/v1/approvals` every 2 s (this also tells the daemon someone is watching), show one card per request, send `allow` or `deny` for that exact id, hide the region when nothing is pending.

## Result
- `ApprovalsCard.tsx` renders every field as text (React escaping replaces the hand-written `escapeHtml` template), disables only the buttons of the request being decided and reloads after the decision. Nothing is inferred or auto-approved; the daemon keeps the rules (ADR 0032).
- `console.html` has `#approvals-root` where the static section was; `console-app.js` lost the loader, the decision call, the `approval-decide` action, `escapeHtml` and the 2 s approvals timer (496 -> 461 lines). The unused `refreshShell` bridge entry is gone.
- `web/test/approvals.test.mjs` dropped its console-source assertions; the behavior (hook round trip, exactly one decision, text-only rendering, 44px targets on a phone) is covered by `web/e2e/approvals.spec.ts` from the Playwright PR.

## Verification
- `CARGO_INCREMENTAL=0` fmt, clippy `-D warnings`, `cargo test --workspace`; web `npm ci`, `npm test`, typecheck, build, `npm run test:e2e` (68 pass).
