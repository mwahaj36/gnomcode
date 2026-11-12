# 2026-11-13 - Console polish and design docs

## Goals

Phase 7 of `docs/design/console-overhaul-plan.md`: font-size sweep, focus styles, hit targets, full visual check (10.3), DESIGN.md and console-workspace.md updates. Also close leftovers from the Phase 3 to 6 reviews: light heat contrast, uneven provider card heights, chat header limits styling, and the checks Phase 6 left unverified.

## Outcomes

- Light heat ramp raised: level 0 `#e4e9ee` with a `--sp-border-strong` outline (1.65:1 against the white card), level 1 `#aecde9` (1.65:1), levels 2 to 4 `#74a9d9`, `#3f86c4`, `#1d5f99` (2.49, 3.88, 6.67). Steps are distinct. Dark ramp unchanged.
- Provider card grid uses `align-items: stretch`; measured at 1440 the three Home cards are all 389x178.
- Chat header limits are `sp-chip` per window (22px high, state colored, tooltip kept). The old 10px/11px rules and per-limit flex markup CSS are gone.
- Font sweep: 105 declarations of 9, 10, 11 and 13px in `gnomcode-console.css`, `console.html`, `console-app.js` and `console-presentation.js` now use `--sp-fs-meta` (12px) or `--sp-fs-body` (14px). The heatmap day labels stay at 10px. Sizes 15 to 26px in legacy headings and the 16px phone inputs were left (see open items).
- Focus: global 2px `--sp-accent` ring for buttons, links, summaries, `.sp-session-row`, `.sp-copy` and `.sp-heatmap-scroll`. The older red rings (`--sp-primary`) were replaced, since brand red is reserved for the primary button and user bubbles.
- Brand red: the selected conversation row no longer has a red border. Submit buttons marked `btn-control` are no longer red; "Add rule" and "Activate license" are neutral so Settings has one primary (Save Preferences).
- Tabular figures now apply to the whole native console. The `font:` shorthand resets `font-variant-numeric`, so the rule is `!important`.
- Hit targets: copy buttons, "View all", pagination buttons, list toggle, folder browse button, settings links and `summary` rows are at least 44px.
- Phone Workspace: choosing a conversation closes the list drawer (it used to cover the transcript); the fixed list toggle no longer overlaps the drawer heading; Terminal and Actions sit on the title row; the redundant in-page "Conversations" button is hidden on all widths because the toolbar toggle does the same job.
- Usage Today: hour labels (00, 06, 12, 18) no longer wrap to two lines.
- `DESIGN.md` has a new "Console overhaul system" section (IA, tokens, components, anti-clutter rules) and the focus-ring statement is corrected. `docs/design/console-workspace.md` FIRST VIEWPORT describes the toggle-only rail and list-first Workspace.

## Problems and root causes

- Screenshots from the browser pane are a crop of the viewport until the window is resized again, so most 1440 checks used DOM metrics (overflow, font size, hit target, red count, numeric variant) via a script plus spot screenshots.
- Tabular figures: first fix (inherit on `.sp-native-console`) did nothing because any `font:` shorthand resets the property. Root fix is the `!important` universal rule with a comment.
- The `button[type="submit"]` rule made every submit button red regardless of class. Now `:not(.btn-control)`.

## Decisions and deviations

- Desktop dense form controls stay 40px (DESIGN.md buttons and fields). The 44px rule covers rail, copy, session rows, chips, pagination, list toggle and everything on phones.
- No JS or Rust logic was added. No ADR or SPEC change: nothing here alters scope or architecture.
- Stretching card heights makes a collapsed card in a row with an open or empty card taller. Accepted for consistent alignment.

## Section 13 checklist (status at this PR)

- [x] Rail has exactly Home, Workspace, Usage, Settings (checked in the DOM).
- [x] `#limits` and `#projects` land on `#usage`, `#automations` on `#settings` (checked).
- [x] Home order: provider cards, grouped sessions, tiles, 12-week grid (section tops 114, 377, 1221, 1411 at 1440).
- [x] Provider cards: % left, reset line, pace, seen time, enable command when empty (Phase 3 tests; empty and exhausted states seen live). Cost footer on Usage not seen with real quota data.
- [x] Usage: 53-week grid with stats and legend, Today 24-cell strip (24 cells counted), day click covered by `heatmap.test.mjs`.
- [x] No bar chart in the local console (no canvas or bar elements).
- [x] Workspace opens on the list; launcher behind New.
- [x] Rust tests of section 8 exist and pass (cargo test --workspace).
- [x] JS tests of phases 3 to 6 exist and pass (`npm test`, 73 tests).
- [x] No quota threshold, level or streak math in `web/lib` (searched).
- [x] SPEC 3.9 and 3.16, ADR, session records: done in earlier phases; DESIGN.md updated here.
- [~] Visual check 10.3 in all 4 modes for all views: see below.

## Visual checklist 10.3 results

Local console from this worktree, temp `GNOMCODE_HOME`, port 41990, real local agent logs read-only. Home, Workspace (list, chat open, launcher open), Usage (All time, Today) and Settings were scripted at 390 and 1440 in dark and light.

- No horizontal page scroll: pass in all views and modes (only the heatmap scroller scrolls; on phone it opens at today on the right edge).
- No text under 12px (except heatmap day labels): pass.
- Brand red: Home, Usage, Settings have at most one primary; Workspace has the Focus Window primary plus user bubbles (allowed).
- No card inside a card: pass (zero nested `.sp-card`).
- Tabular figures: pass after the fix.
- Hit targets: pass on phone in both themes; desktop form controls at 40px by decision.
- Rail expansion pushes content: unchanged from Phase 6.
- Focus: Tab walks rail, view, copy buttons, summary, link, session rows; ring is 2px accent, `:focus-visible` true.
- Reduced motion: the global `prefers-reduced-motion` rule and the meter, chevron and row rules are present; the pane cannot emulate the media query, so this was read from the CSS, not observed.
- No emojis: pass (scripted scan).
- Accessibility: provider meters expose `role="meter"` with min, max and now; the heatmap has `aria-label` "Token activity, last year".
- Not run: web console `/console` (needs sign-in) and the Remote surfaces; the 1440 dark Workspace launcher form was only measured, not viewed.

Before and after (descriptions): light heatmap went from near-invisible cells on a white card to a clearly stepped blue ramp with an outlined empty cell. Provider cards went from ragged heights to a flush row. The phone conversation list covered the open transcript and overlapped its own toggle; it now closes on selection and the toggle hides while the drawer is open.

## Open items

- Legacy headings still use 15 to 26px sizes outside the scale (for example 17, 18, 21, 22, 26px in `gnomcode-console.css`); a token-based pass needs per-heading review.
- Usage shows both the Activity grid card and the older "Recorded activity" summary (same 6.3B figure); consider merging.
- The heatmap hides weeks before the first recorded day, so a young database shows a mostly empty card.
- "Today, by hour" subtitle says local time while the card meta says UTC days; wording needs a Rust-side decision.
- The inline-styled "execute-relay" button in `console-app.js` and the Pricing and Offline license details still use inline styles.
- Web console and PWA visual pass belongs to Phase 9.

## Verification

- Gate: `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` with `CARGO_INCREMENTAL=0`; web `npm ci`, `npm test` (73 pass), `npm run typecheck`, `npm run build`.
