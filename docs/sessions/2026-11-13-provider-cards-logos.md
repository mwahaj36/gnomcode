# 2026-11-13 - Provider card height, missing-state fallback and logos

## Goals

Fix four bugs seen on the Limits provider cards: stretched collapsed cards, a "No data" chip on every card, lost Antigravity pool grouping, and letter tiles instead of provider marks.

## Outcomes

- `.sp-card-grid` uses `align-items: start`; each card keeps its own height.
- A window with no Rust `state` but a reading renders as `plain`: no chip, accent-colored meter fill and strip bar. "No data" remains only for a window with no reading or a provider with no windows. No thresholds are computed in JS.
- Windows without `pool` are grouped by `limit_id` (title-cased) when the provider has two or more distinct `limit_id` values. Formatting only.
- `web/lib/provider-logos.js` holds monochrome marks (`currentColor`, 18px in the 32px tile, `aria-hidden`): Claude (claude-code), OpenAI (codex), Google Gemini (gemini-cli, and antigravity). Unknown providers keep the letter tile. The tray menu stays text-only.
- Shared presenter, so console, Home, web console and Remote all get the change.

## Marks source and license

Paths are from simple-icons (CC0-1.0, https://github.com/simple-icons/simple-icons): `icons/claude.svg`, `icons/openai.svg`, `icons/googlegemini.svg`, fetched 2026-11-13. simple-icons has no Antigravity icon, so Antigravity uses the Gemini mark. Marks are trademarks of their owners and only identify the provider.

## Problems and root causes

- The Phase 7 grid used `align-items: stretch`, so short collapsed cards grew to the tallest card in the row.
- `stateOf` mapped a missing `state` to `unknown`, which rendered "No data" and gray bars for payloads from older desktops or Remote sync.

## Verification

- `web/test/provider-cards.test.mjs`: no chip without state, No data kept for no reading, grid alignment, pool fallback, svg per known provider, letter fallback.
- Rendered a synthetic payload with and without `state` fields in dark and light at desktop and 390px width.
- Full gate results are in the PR description.
