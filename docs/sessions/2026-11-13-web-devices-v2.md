# Web devices and connect page redesign (2026-11-13)

## Goals

Redesign the web console "Connect your desktop" page and the Devices section with the console v2 system. No backend or business logic changes.

## Outcomes

- `web/components/DevicesView.tsx` (new, presentation only) renders the list. `DesktopDevices.tsx` keeps all data, polling and actions and passes props down.
- Each desktop is a card: device icon, name, status chip (Online when the last heartbeat is under 3 minutes old, otherwise Offline, or Access off), relative last seen, one brand-red Open desktop button (first enabled desktop only), and a quiet destructive Disable account access text button that still opens the existing confirmation dialog.
- Connected browsers use the session-list style, grouped This device, Approved, Pending and Revoked, with phone or browser icons, status chips (hidden under 480px where the group label carries the state), full device ID in a tooltip, and a quiet Remove with inline Cancel/Remove confirmation.
- "This device" comes from the request id stored in the local device record (`savedDevice`), read only.
- Page header gains a purpose line; the notification and connection icon buttons show text labels from 640px and keep tooltips and aria-labels below that.
- Empty and signed-out states are one line plus one action.
- `web/lib/devices-presentation.mjs` holds the pure helpers (relative time, online check, device kind, grouping) with tests.
- CSS lives in a labeled section at the end of `web/app/gnomcode-console.css`.

## Problems and decisions

- Approve and Deny were not added. Approval is automatic and account-wide (`scripts/check-remote-companion.mjs` asserts there is no per-desktop approval step), and the web has no approval API. Pending rows read "Waiting for your desktop to accept".
- The desktop heartbeat interval is not exposed to the web, so the 3 minute online window is a display heuristic.
- `.sp-remote button:not([type="submit"])` outranked single-class button styles, so device buttons use `:is(button,a)` selectors with higher specificity.

## Verification

Rendered with a local synthetic harness (not committed) at 390 and 1440, dark and light, including empty, signed-out and inline remove confirmation. `npm ci`, `npm test`, `npm run typecheck` and `npm run build` pass in `web/`. No Rust touched.
