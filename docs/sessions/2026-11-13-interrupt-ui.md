# 2026-11-13 - G5 Interrupt Button UI (Console and Phone)

## Goals

Add the Section G5 `Interrupt` button to both the local daemon console chat header and the E2EE Remote phone/web companion composer, consuming Worker A's interrupt API contract from PR #187 (`feat/remote-interrupt`).

## Outcomes

- Added shared pure presentation helpers in `web/lib/console-presentation.js` and `web/lib/console-presentation.d.ts`:
  - `isSessionRunningState(state)` returning true only for `thinking`, `tool`, and `running`.
  - `createInterruptRequestId()` producing an ASCII `req_<hex>` token matching `[A-Za-z0-9_-]{8,64}`.
  - `formatInterruptAckMessage(sent)` returning deterministic UI status copy for `sent: true` vs `sent: false`.
- Wired `#chat-interrupt-btn` into `crates/daemon/src/console.html` (`.conversation-heading`) and `crates/daemon/src/console-app.js` (`updateInterruptButton`, `interruptChat`) calling `POST /api/v1/sessions/{id}/interrupt`.
- Wired `.sp-interrupt-btn` into `web/components/RemoteCompanion.tsx` (`.remote-composer-toolbar`) sending encrypted `{ type: "interrupt", session_id, request_id }` and handling `interrupt_ack` and `Unsupported remote command` errors.
- Added design-token styling in `web/app/gnomcode-console.css` with a 44 px minimum mobile touch target under `@media (max-width: 760px)`.
- Added `web/test/interrupt-ui.test.mjs` covering state gating, request ID generation/retry reuse discipline, and ack formatting.

## Decisions

- Kept the same `request_id` across explicit user retries for the same turn and cleared pending interrupt state on ack, session switch, or disconnect so an interrupt command is never replayed automatically across reconnects.
- Documented the dependency on Worker A PR #187 (`feat/remote-interrupt`).

## Verification

- `web/`: `npm ci`, `npm test`, `npm run typecheck`, `npm run build`
- `scripts/check-local-console.mjs` and `scripts/check-console-presentation.mjs`
- Chromium headless visual verification at `390x844` and `1440x900` (dark and light)
- Rust workspace gate (`CARGO_INCREMENTAL=0`): `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
