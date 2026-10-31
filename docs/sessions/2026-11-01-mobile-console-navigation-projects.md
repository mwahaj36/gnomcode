# Mobile console navigation and project activity

Date: 2026-11-01

## Problem and root causes

Session history's Remote link opened `/remote`, whose separate page rendered only the pairing companion and omitted the console navigation. Project activity always called the unavailable local-rollup renderer even when the account snapshot contained project names in session history.

## Changes and decisions

Render Remote inside the same console shell, with Workspace initially selected. Browser back/forward resolves the current path so returning to Dashboard does not select Workspace accidentally. Show existing synced project labels as escaped React buttons opening their filtered session history. Provide connect/reload actions when data is absent, distinguish a missing snapshot from an empty snapshot, and explain that detailed totals stay local. Remove the timeline control from this snapshot-derived project list. No additional cloud fields, client-side quota math or project rollup calculation are introduced.

## Verification

The headless Edge mobile regression reproduced all three failures before implementation: absent pairing rail, missing synced-project action and missing connection action. It now passes with a 390px touch viewport and controlled account snapshot. TypeScript, all 18 web tests, production build and the full Rust quality gate passed. The existing encrypted relay acceptance also passed after the shell change, including pairing, saved trust, touch sheets, keyboard-height composer geometry and stalled-open-socket recovery. Physical phone testing is unavailable.
