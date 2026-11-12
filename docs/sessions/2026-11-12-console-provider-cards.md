# 2026-11-12 - Collapsible provider limit cards (console overhaul Phase 3, frontend)

## Goals

- Render provider limits as CodexBar-style collapsible cards from `GET /api/v1/providers/overview` in the local console, and keep the web console and Remote working through the shared presenters.
- Follow `docs/design/console-overhaul-plan.md` sections 5.1-5.5 and addendum 15.1 (collapsed by default); `docs/design/console-mockup.html` wins where they differ.

## Outcomes

- `web/lib/console-presentation.js`: new `renderMeterRow`, `renderPoolGroup`, `renderProviderCard`, `renderProviderCards`, `updateProviderCards`. `renderQuota` and `updateQuota` are kept (web console, Remote, tour) and now group raw pools per provider and call the same card code. `renderConversationLimits` emits one state chip per window. The old `PROVIDERS` table, tone and headline helpers and the compact and full quota markup were deleted.
- Cards are native `<details>`, collapsed by default. The summary shows the provider mark, name, plan, the `headline_window` line, a strip of one bar per window, a state chip and a chevron. Open state survives the 2-second poll (existing `updateMarkup` key logic) and page reloads (`localStorage` key `gnomcode-provider-open`, try/catch, synced across the Home and Usage containers).
- Multi-pool providers group windows under Rust-supplied pool names with compact `.sp-mini` rows. `source_issue` replaces the reset line in the summary and the affected meter, and shows as a note in multi-pool bodies.
- `console-app.js`: `loadQuota` reads `/api/v1/providers/overview`, renders `#quota-table` (with cost footer) and `#overview-limits` (without), re-renders once a minute so relative times stay honest, and a delegated `[data-copy]` handler copies enable commands.
- CSS: v2 section appended to `web/app/gnomcode-console.css`; dead `.sp-limit-*` and `.sp-limits-compact` rules removed. SPEC 3.9 updated.
- Tests: `web/test/provider-cards.test.mjs` (14 tests); `scripts/check-console-presentation.mjs` and `check-native-navigation.mjs` updated to the new markup.

## Problems and root causes

- Meter rows and pools were siblings in a grid body that already had a 24px gap, so margins stacked to 40px. Fix: wrap windows in `.sp-windows`.
- Light-theme chip text (`--sp-ok` 3.8:1, `--sp-low` 4.0:1 on white, less on the tint) failed 4.5:1. Fix: chip text uses `--sp-text`; the state color stays on the dot and tint. Amber issue lines use `--sp-text-2`.
- The meter fill animation replayed on every re-render because the markup is replaced. Fix: it only runs under `.sp-limits[data-enter]`, set on first render.
- Home's legacy two-column grid squeezed cards into 260px. Interim fix: single column until the Home board (Phase 5).

## Decisions

- Chip text for exhausted is "Out" (mockup) rather than "Exhausted" (plan 5.5 step 2).
- The empty (no readings) variant is a plain article, not a `<details>`: there is nothing to reveal and the copy button must be visible.
- Raw pools from older desktops have no `headline_window`; the headline is the first window with the worst Rust-supplied state. Windows without `state` render as unknown with plain percent left.
- The copyable command keeps the monospace stack; all other limit text is sans.
- Old `.sp-limit-row` rules in `web/app/globals.css` belong to the marketing site and were left alone.

## Verification

- Visual: local console from this worktree on port 41790 with a temporary `GNOMCODE_HOME`; real readings (Codex weekly exhausted) plus a scratch injection of realistic data (Claude weekly 98% left with session reset, Codex weekly exhausted, Antigravity pools "Claude and GPT models", "Gemini Models", "Gemini 4 Argon", an empty provider). Dark and light at 1440 and 390: no horizontal scroll, 64px summary rows, container query hides the strip under 400px.
- Review with apple-design (disclosure-controls, gauges, accessibility contrast and targets) and impeccable craft floor: contrast and animation findings fixed above; state is always worded, never color alone.
- Gate: cargo fmt, clippy -D warnings, test --workspace; web `npm ci`, `npm test`, `npm run typecheck`, `npm run build`.
