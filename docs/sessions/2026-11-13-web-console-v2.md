# 2026-11-13 - Web console v2 and PWA polish

## Goals

Phase 9 of `docs/design/console-overhaul-plan.md` (addendum 15.3 and 15.4): bring the account web console and the Remote/PWA to parity with the local console, carry the needed Rust fields in sync and Remote payloads, and close the Phase 7 leftovers.

## Outcomes

- `web/app/console/Client.tsx`: four places (Home, Workspace, Usage, Settings) with a toggle-only rail (hover expansion removed); `limits`, `projects`, `automations` alias to Usage or Settings; `sessions` stays as a sub-page of Home (View all) with Home highlighted. Home = provider cards, grouped session rows, Today tiles. Usage = cards, period totals tiles, spend by agent, subscription value, activity breakdown, projects. SummaryPanel, OngoingPanel and the old Dashboard layout are gone. No heatmap on the web (synced data lacks it).
- Under 720px the rail is a safe-area-aware top bar with four icons; `viewport-fit=cover` is set through the Next `viewport` export; manifest colors were already `#070708` and display `standalone`; `touch-action: manipulation` applies globally.
- Sync: sessions now include `home_group` (key list test updated). Quota already carried state, pace, label, pool, window_label and stale. Remote conversation limits gained optional `window`, `window_label`, `pool`, `resets_at`, `reset_passed`, `pace`. No chat content added.
- Remote: header limit chips open a limits sheet with one collapsed provider card (`renderConversationLimitCard`).
- Leftovers: the local Usage "Recorded activity" block no longer repeats the token total next to the grid (`renderActivity(report, { totals: false })`); Today hours say "hours in your local time" and the grid says "Days in UTC"; Pricing, Offline license and relay/guardrail table inline styles became classes (`sp-tone-*`, `sp-row-action`, ...).

## Problems and root causes

- Old rail CSS stacked hover rules, `!important` overrides and `first-of-type` assumptions for the toggle position. The toggle moved to the bottom, so those rules were deleted and replaced by one block keyed on `data-place` and `.web-rail-toggle`.
- `/console` needs a Supabase sign-in. Verification used a synthetic localStorage session and a page-level fetch stub for `spawnt_desktops` (nothing committed, no real credentials).

## Decisions

- Desktops older than this change have no `home_group`; their sessions all show under Recent rather than guessing a group in TypeScript.
- Remote limits sheet drops the short chip label so the card shows the Rust `window_label`.

## Verification

- `cargo fmt`, `clippy -D warnings`, `cargo test --workspace`; web `npm ci`, `npm test` (80 pass), `typecheck`, `build`.
- Browser at 390x844, dark and light, with synthetic data: Home and Usage render, top bar shows four icons.

## Open items

- Remote limits sheet and standalone (installed) mode were not exercised visually; Remote needs a paired desktop.
- Desktop 1440 expanded-rail and Settings pages were only spot-checked.
