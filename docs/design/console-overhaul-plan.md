# Console overhaul: implementation plan

Status: proposed (2026-11-12). Owner: Wahaj. Supersedes the console parts of `docs/gnomcode_redesign_plan.md`.

Audience: an implementing agent that must follow instructions literally. Do every step in order. Do not skip a step because it looks unnecessary. Do not add anything that is not in this plan. When this plan and the code disagree about a fact (a file name, a field name, a line number), trust the code, adjust the step to match, and write the difference in the session record (section 12).

---

## 0. Rules for the implementing agent

Read these before touching any file. Breaking one of them means the PR is rejected.

1. Read `AGENTS.md` fully. Product invariants 6 (trust levels), 8 (business logic in Rust only) and 9 (no emojis) apply on every line of this plan.
2. **No business logic in JavaScript.** JavaScript may only format values it receives: join strings, pick a CSS class from an enum string, format dates and numbers. It may NOT compute thresholds, percentages, pace, levels, streaks, groupings or "is this low" decisions. Those come from Rust as fields. Where a step says "Rust computes X", do not also compute X in JS "as a fallback" unless the step explicitly says so.
3. **No emojis** anywhere: code, CSS comments, copy, commit messages, docs.
4. **No new npm or Cargo dependencies.** Everything in this plan is possible with what is already in the repo.
5. **One branch and one PR per phase** (section 9). Branch names are given. Never commit to `main`.
6. Each PR updates `docs/SPEC.md` in the same change (section 12 gives the exact text to change).
7. Each PR adds a dated session record under `docs/sessions/` and a row in `docs/sessions/README.md`.
8. Before every commit, run the full gate (section 10.1). Do not commit if it fails. Fix the cause, do not silence lints, do not add `#[allow]`.
9. Do not delete existing comments or docs unrelated to the change.
10. Do not change behavior that this plan does not mention (Remote crypto, pairing, PTY, automations logic, sync). If a step seems to require it, stop and write it down as a blocker.
11. Copy rule: every label you write must be 6 words or fewer. Every helper sentence must be 12 words or fewer. If you need more, it goes in a `title` tooltip or a `<details>` disclosure.

---

## 1. Goal and scope

### 1.1 Goal

A modern, calm local console that answers three questions in the first second, the way CodexBar does for limits:

1. How much do I have left on each provider, and when does it come back?
2. What are my agents doing right now, and does any need me?
3. How much have I been using (tokens and estimated spend) over time?

### 1.2 In scope

- New visual system (tokens, type, spacing, elevation, motion).
- Provider limit cards (CodexBar style) with pace.
- GitHub-style activity grid for tokens, replacing the bar chart.
- Home board (replaces Dashboard).
- Navigation reduced from 7 places to 4.
- Restyle of Workspace list, Usage and Settings to the new system.
- Rust API additions needed to keep logic out of JS.

### 1.3 Out of scope (do NOT do these)

- Porting the local console to React or any framework.
- Tray popover, notifications, Remote crypto or pairing changes.
- New limit sources or provider adapters.
- A command palette, per-agent heatmap filters, period highlighting on the heatmap. These are listed in section 14 as later work.
- Changing chat transcript rendering (`renderChatCards`, `reconcileChat`, `updateChat`).

---

## 2. Ground truth: where things live today

| Thing | Location | Notes |
| :--- | :--- | :--- |
| Local console page | `crates/daemon/src/console.html` (~825 lines) | Lines 9-475 are an inline `<style>` block. Body starts with `<body class="sp-native-console">`. Embedded with `include_str!` in `crates/daemon/src/api.rs` (`CONSOLE_HTML`). |
| Local console script | `crates/daemon/src/console-app.js` (~1745 lines) | `setView(view)` at ~line 109 switches views by toggling `[data-local-view]` sections. 2-second `setInterval` poll at ~line 1733. Loaders: `loadReport`, `loadQuota`, `loadSessions`, `loadOngoing`, `loadProjects`, `loadSettingsData`. |
| Shared presenters | `web/lib/console-presentation.js` (+ `.d.ts`) | Used by BOTH the local console (served at `/console-presentation.js`) and the web console (`web/app/console/Client.tsx`) and Remote. Key exports: `renderQuota`, `updateQuota`, `renderActivity`, `updateActivity`, `renderSummary`, `renderOngoing`, `renderSessionList`, `renderConversationLimits`. |
| Shared stylesheet | `web/app/gnomcode-console.css` | Served by the daemon at `/gnomcode-console.css` via `include_str!`. Also imported by the web app. |
| Tokens | `web/app/gnomcode-tokens.css` | Served at `/gnomcode-tokens.css`. `--sp-*` variables, dark default, light via `prefers-color-scheme` and `[data-theme]`. |
| Quota API | `GET /api/v1/quota` -> `quota_views()` in `crates/daemon/src/api.rs` (~line 2172) | Returns `QuotaPoolView { ...QuotaSnapshot, remaining_pct, reset_passed }`. |
| Quota types | `crates/core/src/lib.rs` | `QuotaSnapshot { provider, limit_id, window: QuotaWindow, used_pct, resets_at, plan, basis: TrustLevel, observed_at }`. `QuotaWindow = FiveHour, Weekly, Monthly`. `TrustLevel = Exact, Estimated, Api, Unknown`. |
| Usage report | `GET /api/v1/report` -> `TimelineReport` in `crates/store/src/lib.rs` (~line 360) | Has `daily: Vec<DailyUsage>`, `activity_interval`, `providers: Vec<ProviderReport>`, totals. `DailyUsage.chart_pct` is computed in store (~line 2945). Daily rows get regrouped into weeks/months by `group_activity` (~line 280). |
| Presence | `crates/core/src/presence.rs` | `SessionPresence = Running, AwaitingApproval, RecentlyIdle, Inactive, Ended`. |
| Vendor limit toggles | `crates/daemon/src/vendor_limits.rs` | `enabled(provider) -> bool`. Enabled from the CLI: `gnomcode limits enable claude` / `antigravity`. There is no HTTP endpoint to toggle; do not add one. |
| JS tests | `web/test/*.test.mjs`, run by `npm test` in `web/` | `calm-presentation.test.mjs` asserts presentation rules (no repeated EXACT badges, disclosures). Keep those passing. |
| Design notes | `DESIGN.md`, `docs/design/console-workspace.md` | Update `DESIGN.md` in Phase 7. |

Before Phase 0, confirm each row with `grep`. If a line number moved, use the new one.

---

## 3. Design direction

### 3.1 Thesis

Quiet surfaces, one strong number per card, color only for state. Every view has exactly one primary action. Explanations live in tooltips and disclosures, never in default paragraphs.

### 3.2 Anti-clutter rules (enforced in review)

1. **One primary number per card.** A provider card's number is "% left" of its most constrained window. A stat tile has one number.
2. **No nested borders.** A card never sits inside another bordered box. Sections are separated by spacing and a heading, not by boxes.
3. **Three text colors only**: `--sp-text` (primary), `--sp-text-2` (secondary), `--sp-text-3` (meta). No other grays.
4. **Color means state.** Green, amber, red and the heat ramp are only for limit state and activity intensity. Brand red (`--sp-primary`) is only for the single primary button per view and the user's chat bubbles.
5. **No explanatory paragraphs in default views.** Delete sentences like "Latest received snapshot. Providers update limits when they emit usage." Replace with a status chip plus `title` tooltip, or a `<details>` disclosure (SPEC 3.16 already requires this).
6. **No repeated trust badges.** Trust (`exact`, `api`, `estimated`, `unknown`) is shown once per card in the card footer tooltip, and inline only when it is `estimated` money (write "est."). This matches the existing `calm-presentation.test.mjs` rule.
7. **Empty states are one line plus one action.** Example: "No Claude limits yet" + a copy-command button.
8. **Max content width 1200px**, centered, with 32px side padding on desktop and 16px on phones.
9. **Tabular numbers** (`font-variant-numeric: tabular-nums`) for every number.
10. **No inline `style=""` attributes** in new markup. Use classes.

### 3.3 What "modern" means here (concrete)

- Cards with a 1px hairline border plus a very soft inner highlight on dark (`box-shadow: inset 0 1px 0 rgb(255 255 255 / .04)`) and a soft drop shadow on light.
- 12px card radius, 6px control radius, 3px meter radius.
- Thin 6px meters with rounded ends.
- Large, light-weight numbers (28px, weight 600, letter-spacing -0.02em) next to small meta text (12px).
- Motion: 160ms ease-out on hover and state changes; meters animate their width once on first render; everything disabled under `prefers-reduced-motion: reduce`.
- Generous whitespace: 24px between cards, 48px between page sections.

---

## 4. Visual system

### 4.1 Tokens

Replace the contents of `web/app/gnomcode-tokens.css` with the block below. Keep the `@font-face` lines at the top exactly as they are today. Keep every existing `--sp-*` variable name (other code uses them); new names are additions.

```css
:root {
  color-scheme: dark;
  /* existing, unchanged values */
  --sp-control-radius: 6px;
  --sp-control-height: 44px;
  --sp-bg: #070708;
  --sp-surface: #0d0d0f;
  --sp-surface-2: #141416;
  --sp-border: #242428;
  --sp-border-strong: #38383e;
  --sp-text: #f6f7f8;
  --sp-text-2: #a9b4c2;
  --sp-text-3: #8d99aa;
  --sp-primary: #ff2544;
  --sp-primary-text: #ff6177;
  --sp-accent: #becbd8;
  --sp-on-primary: #0c1119;
  --sp-font-sans: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  --sp-font-mono: "JetBrains Mono", Consolas, monospace;

  /* new: layout */
  --sp-radius-card: 12px;
  --sp-radius-meter: 3px;
  --sp-content-max: 1200px;
  --sp-gutter: 32px;

  /* new: spacing scale (use only these) */
  --sp-1: 4px; --sp-2: 8px; --sp-3: 12px; --sp-4: 16px;
  --sp-5: 24px; --sp-6: 32px; --sp-7: 48px;

  /* new: type scale (use only these) */
  --sp-fs-meta: 12px;
  --sp-fs-body: 14px;
  --sp-fs-strong: 15px;
  --sp-fs-title: 20px;
  --sp-fs-number: 28px;

  /* new: elevation */
  --sp-card-shadow: inset 0 1px 0 rgb(255 255 255 / .04);
  --sp-card-hover-border: #2e2e33;

  /* new: state colors */
  --sp-ok: #3fcf8e;
  --sp-low: #f5a524;
  --sp-exhausted: #ff4d5e;
  --sp-unknown: #5b6472;
  --sp-ok-soft: rgb(63 207 142 / .14);
  --sp-low-soft: rgb(245 165 36 / .14);
  --sp-exhausted-soft: rgb(255 77 94 / .14);

  /* new: activity heat ramp, level 0..4 */
  --sp-heat-0: #17171a;
  --sp-heat-1: #1f3a4d;
  --sp-heat-2: #2b5f80;
  --sp-heat-3: #3f8fbf;
  --sp-heat-4: #7cc4ef;

  /* new: motion */
  --sp-ease: cubic-bezier(.2, .8, .2, 1);
  --sp-dur: 160ms;
}
```

Light values. Add these new variables to BOTH existing light blocks (`@media (prefers-color-scheme: light) { :root:not([data-theme="dark"]) {...} }` and `:root[data-theme="light"] {...}`), next to the existing light overrides:

```css
  --sp-card-shadow: 0 1px 2px rgb(12 17 25 / .06), 0 1px 1px rgb(12 17 25 / .04);
  --sp-card-hover-border: #c9d1da;
  --sp-ok: #12955c;
  --sp-low: #b86e00;
  --sp-exhausted: #d4102e;
  --sp-unknown: #8a94a3;
  --sp-ok-soft: rgb(18 149 92 / .12);
  --sp-low-soft: rgb(184 110 0 / .12);
  --sp-exhausted-soft: rgb(212 16 46 / .10);
  --sp-heat-0: #edf0f3;
  --sp-heat-1: #c6dcef;
  --sp-heat-2: #8dbde3;
  --sp-heat-3: #4a93cc;
  --sp-heat-4: #1d5f99;
```

Why blue for heat and not red: red already means "exhausted" on limit cards. A red grid would read as an alarm.

Check contrast after editing: `--sp-text-3` on `--sp-surface` must be at least 4.5:1 in both themes. Use any contrast checker; if it fails, darken (light) or lighten (dark) `--sp-text-3` until it passes and note the new value in the session record.

### 4.2 Typography

- Family: `var(--sp-font-sans)` everywhere except code/paths (`var(--sp-font-mono)`).
- Page title (`h1#console-title`): `--sp-fs-title`, weight 600, letter-spacing -0.01em.
- Section heading (`h2`): `--sp-fs-strong`, weight 600, color `--sp-text`.
- Body: `--sp-fs-body`, line-height 1.5, color `--sp-text-2`.
- Meta (timestamps, "Resets in", legends): `--sp-fs-meta`, color `--sp-text-3`.
- Big numbers: `--sp-fs-number`, weight 600, letter-spacing -0.02em, tabular-nums, color `--sp-text`.
- Never use font sizes outside the scale. Never use 11px.

### 4.3 Icons

Keep the existing inline 24x24 stroke SVGs (`stroke: currentColor; fill: none; stroke-width: 1.75`). New icons needed: home (house outline), copy (two overlapping squares), chevron-right. Draw them as simple paths. No icon fonts, no emojis.

---

## 5. Components

All component CSS goes into `web/app/gnomcode-console.css` under a new section header comment `/* ---- Console v2 components ---- */` at the END of the file. Before adding any class, run `grep -rn "<classname>" web crates/daemon/src` to make sure the name is unused. Every component below gives: markup (the exact HTML the presenter must output), CSS, and states.

### 5.1 Card

```html
<article class="sp-card">...</article>
```

```css
.sp-card { background: var(--sp-surface); border: 1px solid var(--sp-border); border-radius: var(--sp-radius-card); box-shadow: var(--sp-card-shadow); padding: var(--sp-5); transition: border-color var(--sp-dur) var(--sp-ease); }
.sp-card:hover { border-color: var(--sp-card-hover-border); }
.sp-card-grid { display: grid; gap: var(--sp-5); grid-template-columns: repeat(auto-fill, minmax(320px, 1fr)); }
@media (max-width: 480px) { .sp-card { padding: var(--sp-4); } .sp-card-grid { grid-template-columns: 1fr; gap: var(--sp-4); } }
```

### 5.2 Section header

```html
<header class="sp-section-head"><h2>Limits</h2><span class="sp-section-meta">Updated 2m ago</span></header>
```

```css
.sp-section-head { display: flex; align-items: baseline; justify-content: space-between; gap: var(--sp-3); margin: 0 0 var(--sp-4); }
.sp-section-head h2 { margin: 0; font-size: var(--sp-fs-strong); font-weight: 600; color: var(--sp-text); }
.sp-section-meta { font-size: var(--sp-fs-meta); color: var(--sp-text-3); }
.sp-section + .sp-section { margin-top: var(--sp-7); }
```

### 5.3 Chip (status label)

```html
<span class="sp-chip sp-chip--low">Low</span>
```

Variants: `--ok`, `--low`, `--exhausted`, `--unknown`, `--neutral`. Text is plain words, never icons alone.

```css
.sp-chip { display: inline-flex; align-items: center; gap: var(--sp-1); height: 22px; padding: 0 var(--sp-2); border-radius: 999px; font-size: var(--sp-fs-meta); font-weight: 500; color: var(--sp-text-2); background: var(--sp-surface-2); white-space: nowrap; }
.sp-chip::before { content: ""; width: 6px; height: 6px; border-radius: 50%; background: currentColor; }
.sp-chip--ok { color: var(--sp-ok); background: var(--sp-ok-soft); }
.sp-chip--low { color: var(--sp-low); background: var(--sp-low-soft); }
.sp-chip--exhausted { color: var(--sp-exhausted); background: var(--sp-exhausted-soft); }
.sp-chip--unknown { color: var(--sp-text-3); }
.sp-chip--neutral::before { display: none; }
```

### 5.4 Meter row (one limit window)

This is the core CodexBar element. One row per window.

```html
<div class="sp-meter-row sp-meter-row--ok" title="Provider measured. Seen 2m ago.">
  <div class="sp-meter-top">
    <span class="sp-meter-label">Session</span>
    <span class="sp-meter-value">42% left</span>
  </div>
  <div class="sp-meter" role="meter" aria-valuemin="0" aria-valuemax="100" aria-valuenow="42" aria-label="Session: 42% left">
    <i class="sp-meter-fill" style="--fill:42%"></i>
    <i class="sp-meter-pace" style="--pace:55%"></i>
  </div>
  <div class="sp-meter-bottom">
    <span>Resets in 2h 14m</span>
    <span>On pace</span>
  </div>
</div>
```

Exception to rule 3.2.10: the two custom properties `--fill` and `--pace` are the only allowed inline styles, because the value is data.

- The fill shows **% left** (remaining), not % used, so a full bar means "plenty left". This matches CodexBar and the "% left" convention everywhere else.
- `sp-meter-pace` is a 2px vertical tick at the position where remaining *should* be if usage were even across the window (`pace_remaining_pct` from Rust). Omit the `<i class="sp-meter-pace">` element entirely when Rust sends no pace.
- Row modifier from Rust `state`: `--ok`, `--low`, `--exhausted`, `--reset_awaiting`, `--unknown`.

```css
.sp-meter-row { display: grid; gap: var(--sp-2); }
.sp-meter-row + .sp-meter-row { margin-top: var(--sp-4); }
.sp-meter-top, .sp-meter-bottom { display: flex; justify-content: space-between; gap: var(--sp-3); }
.sp-meter-label { font-size: var(--sp-fs-body); color: var(--sp-text-2); }
.sp-meter-value { font-size: var(--sp-fs-body); font-weight: 600; color: var(--sp-text); font-variant-numeric: tabular-nums; }
.sp-meter-bottom { font-size: var(--sp-fs-meta); color: var(--sp-text-3); font-variant-numeric: tabular-nums; }
.sp-meter { position: relative; height: 6px; border-radius: var(--sp-radius-meter); background: var(--sp-surface-2); overflow: hidden; }
.sp-meter-fill { position: absolute; inset: 0 auto 0 0; width: var(--fill); border-radius: inherit; background: var(--sp-ok); animation: sp-meter-in 600ms var(--sp-ease) both; }
.sp-meter-pace { position: absolute; top: -2px; bottom: -2px; left: var(--pace); width: 2px; background: var(--sp-text); opacity: .55; }
.sp-meter-row--low .sp-meter-fill { background: var(--sp-low); }
.sp-meter-row--exhausted .sp-meter-fill { background: var(--sp-exhausted); }
.sp-meter-row--exhausted .sp-meter-value { color: var(--sp-exhausted); }
.sp-meter-row--reset_awaiting .sp-meter-fill, .sp-meter-row--unknown .sp-meter-fill { background: var(--sp-unknown); }
@keyframes sp-meter-in { from { width: 0; } }
@media (prefers-reduced-motion: reduce) { .sp-meter-fill { animation: none; } .sp-card { transition: none; } }
```

Value text per state (strings come from section 8.1 `headline` field, JS does not invent them):

| state | value text | bottom-left | bottom-right |
| :--- | :--- | :--- | :--- |
| ok / low | `42% left` | `Resets in 2h 14m` or `Resets Mon 08:20` | pace text or empty |
| exhausted | `Exhausted` | `Back in 1h 05m` | empty |
| reset_awaiting | `Reset` | `Awaiting next reading` | empty |
| unknown | `Unknown` | `Reset time unknown` | empty |

Countdown formatting is formatting, so JS may do it: from `resets_at` and `Date.now()`; under 60 min `Resets in 14m`; under 48 h `Resets in 2h 14m`; otherwise `Resets Mon 08:20` (`toLocaleString(undefined, { weekday: 'short', hour: '2-digit', minute: '2-digit' })`). Keep the existing `limitDetail` helper and adapt it.

### 5.5 Provider card

```html
<article class="sp-card sp-provider-card" data-provider="claude-code">
  <header class="sp-provider-head">
    <div class="sp-provider-id">
      <span class="sp-provider-mark" aria-hidden="true">C</span>
      <div><h3>Claude Code</h3><span class="sp-provider-plan">Max</span></div>
    </div>
    <span class="sp-chip sp-chip--ok">OK</span>
  </header>
  <div class="sp-provider-windows"> ...one .sp-meter-row per window... </div>
  <footer class="sp-provider-foot" title="Provider measured (api). Seen 2m ago.">
    <span>Today $1.20 est. · 30 days $48 est.</span>
    <span>Seen 2m ago</span>
  </footer>
</article>
```

- `sp-provider-mark` is a 28px rounded square with the first letter of the display name, `--sp-surface-2` background, `--sp-text-2` color. No vendor logos (trademark risk, and no emojis).
- The header chip is the card's overall `state` from Rust (worst window).
- Window order: `five_hour` (label "Session"), `weekly` ("Weekly"), `monthly` ("Monthly"). If a provider has more than one `limit_id` for the same window (Antigravity pools), the label is `"<Pool name> · Weekly"` and Rust supplies the pool name (section 8.1 `label`).
- Footer cost line: omit the whole `<span>` when Rust sends `cost` as null. Write "est." after every dollar figure unless `cost_basis` is `exact`.
- **No-reading variant** (provider known but no windows):

```html
<article class="sp-card sp-provider-card sp-provider-card--empty" data-provider="claude-code">
  <header class="sp-provider-head">...same id block...<span class="sp-chip sp-chip--unknown">No data</span></header>
  <p class="sp-provider-empty">Limits are off for Claude Code.</p>
  <button type="button" class="sp-copy" data-copy="gnomcode limits enable claude"><code>gnomcode limits enable claude</code><span>Copy</span></button>
</article>
```

The sentence and command come from Rust (`enable_hint`, `enable_command`). If `enable_command` is null (Codex), render only the hint ("Appears after your next Codex request.").

```css
.sp-provider-card { display: grid; gap: var(--sp-5); align-content: start; }
.sp-provider-head { display: flex; align-items: center; justify-content: space-between; gap: var(--sp-3); }
.sp-provider-id { display: flex; align-items: center; gap: var(--sp-3); min-width: 0; }
.sp-provider-id h3 { margin: 0; font-size: var(--sp-fs-strong); font-weight: 600; color: var(--sp-text); }
.sp-provider-plan { font-size: var(--sp-fs-meta); color: var(--sp-text-3); text-transform: capitalize; }
.sp-provider-mark { display: grid; place-items: center; width: 28px; height: 28px; border-radius: 8px; background: var(--sp-surface-2); color: var(--sp-text-2); font-weight: 600; font-size: var(--sp-fs-body); flex: none; }
.sp-provider-foot { display: flex; justify-content: space-between; gap: var(--sp-3); padding-top: var(--sp-4); border-top: 1px solid var(--sp-border); font-size: var(--sp-fs-meta); color: var(--sp-text-3); font-variant-numeric: tabular-nums; }
.sp-provider-empty { margin: 0; font-size: var(--sp-fs-body); color: var(--sp-text-2); }
.sp-copy { display: flex; align-items: center; justify-content: space-between; gap: var(--sp-3); width: 100%; min-height: 44px; padding: 0 var(--sp-3); border: 1px solid var(--sp-border); border-radius: var(--sp-control-radius); background: var(--sp-surface-2); color: var(--sp-text); cursor: pointer; }
.sp-copy code { font-family: var(--sp-font-mono); font-size: var(--sp-fs-meta); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.sp-copy span { font-size: var(--sp-fs-meta); color: var(--sp-text-3); }
```

Copy behavior: one delegated click handler in `console-app.js` for `[data-copy]`: `navigator.clipboard.writeText(value)`, then set the inner `<span>` text to `Copied` for 1500 ms, then back to `Copy`. Wrap in try/catch; on failure set it to `Select and copy`.

**Compact variant** (conversation header and Home mini): `renderProviderCard(p, { compact: true })` outputs only the header row and the meter rows without the bottom line and without the footer, and uses class `sp-provider-card--compact` with `padding: var(--sp-4); gap: var(--sp-3);`.

### 5.6 Stat tile

```html
<div class="sp-tile"><span class="sp-tile-label">Tokens today</span><strong class="sp-tile-value">2.4M</strong><span class="sp-tile-meta">vs 1.9M yesterday</span></div>
```

```css
.sp-tiles { display: grid; gap: var(--sp-4); grid-template-columns: repeat(3, minmax(0, 1fr)); }
@media (max-width: 720px) { .sp-tiles { grid-template-columns: 1fr; } }
.sp-tile { display: grid; gap: var(--sp-1); padding: var(--sp-4) var(--sp-5); border: 1px solid var(--sp-border); border-radius: var(--sp-radius-card); background: var(--sp-surface); box-shadow: var(--sp-card-shadow); }
.sp-tile-label { font-size: var(--sp-fs-meta); color: var(--sp-text-3); }
.sp-tile-value { font-size: var(--sp-fs-number); font-weight: 600; letter-spacing: -.02em; color: var(--sp-text); font-variant-numeric: tabular-nums; line-height: 1.1; }
.sp-tile-meta { font-size: var(--sp-fs-meta); color: var(--sp-text-3); }
```

Max three tiles per row. Omit `sp-tile-meta` when there is nothing to compare.

### 5.7 Session row (Home board and Workspace list)

```html
<button type="button" class="sp-session-row" data-action="open-session:ID">
  <span class="sp-session-dot sp-session-dot--running" aria-hidden="true"></span>
  <span class="sp-session-main">
    <strong>Fix quota parser</strong>
    <span>fleethud · Claude Code · opus-4</span>
  </span>
  <span class="sp-session-context" title="Context 62% full"><i style="--fill:62%"></i></span>
  <span class="sp-session-time">12m</span>
  <svg class="sp-session-chevron" ...chevron-right... aria-hidden="true"></svg>
</button>
```

Use the existing click action the old session list uses to open a conversation (grep `renderSessionList` and the `data-action` it emits; reuse that exact action string instead of inventing `open-session`).

```css
.sp-session-list { display: grid; border: 1px solid var(--sp-border); border-radius: var(--sp-radius-card); background: var(--sp-surface); overflow: hidden; }
.sp-session-row { display: grid; grid-template-columns: 8px minmax(0, 1fr) 64px 48px 16px; align-items: center; gap: var(--sp-3); min-height: 56px; padding: var(--sp-2) var(--sp-4); border: 0; border-top: 1px solid var(--sp-border); background: transparent; color: inherit; text-align: left; cursor: pointer; transition: background var(--sp-dur) var(--sp-ease); }
.sp-session-row:first-child { border-top: 0; }
.sp-session-row:hover, .sp-session-row:focus-visible { background: var(--sp-surface-2); }
.sp-session-main { display: grid; min-width: 0; }
.sp-session-main strong { font-size: var(--sp-fs-body); font-weight: 500; color: var(--sp-text); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.sp-session-main span { font-size: var(--sp-fs-meta); color: var(--sp-text-3); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.sp-session-dot { width: 8px; height: 8px; border-radius: 50%; background: var(--sp-unknown); }
.sp-session-dot--running { background: var(--sp-ok); box-shadow: 0 0 0 3px var(--sp-ok-soft); }
.sp-session-dot--needs_you { background: var(--sp-low); box-shadow: 0 0 0 3px var(--sp-low-soft); }
.sp-session-context { height: 4px; border-radius: 2px; background: var(--sp-surface-2); position: relative; overflow: hidden; }
.sp-session-context i { position: absolute; inset: 0 auto 0 0; width: var(--fill); background: var(--sp-text-3); }
.sp-session-time { font-size: var(--sp-fs-meta); color: var(--sp-text-3); text-align: right; font-variant-numeric: tabular-nums; }
.sp-session-chevron { width: 16px; height: 16px; color: var(--sp-text-3); }
@media (max-width: 480px) { .sp-session-row { grid-template-columns: 8px minmax(0, 1fr) 40px; } .sp-session-context, .sp-session-chevron { display: none; } }
```

- Omit the context `<span>` when the session has no `context_tokens`/`context_window`. Context % comes from Rust if a field exists; if the session payload only has the two raw numbers, dividing them for a bar width is formatting and is allowed.
- Time: relative to `last_event_at` using the existing `relativeActivity` helper, shortened (`12m`, `3h`, `2d`).
- Dot class comes from Rust `home_group` (section 8.3).

### 5.8 Activity grid (GitHub style)

```html
<section class="sp-section sp-heatmap" aria-label="Token activity, last year">
  <header class="sp-section-head"><h2>Activity</h2><span class="sp-section-meta">UTC days</span></header>
  <div class="sp-heatmap-stats">
    <div><strong>412M</strong><span>tokens this year</span></div>
    <div><strong>143</strong><span>active days</span></div>
    <div><strong>9</strong><span>day streak</span></div>
    <div><strong>Oct 3</strong><span>busiest day</span></div>
  </div>
  <div class="sp-heatmap-scroll" tabindex="0">
    <div class="sp-heatmap-months" style="--weeks:53"><span style="--col:1">Nov</span>...</div>
    <div class="sp-heatmap-body">
      <div class="sp-heatmap-days"><span>Mon</span><span></span><span>Wed</span><span></span><span>Fri</span><span></span><span></span></div>
      <div class="sp-heatmap-grid" role="img" aria-label="412M tokens over 143 active days. Details below.">
        <i class="sp-heat" data-level="0" data-date="2025-11-10" title="Nov 10: no recorded activity"></i>
        <i class="sp-heat" data-level="3" data-date="2025-11-11" title="Nov 11: 2.4M tokens · $3.10 est. · 41 turns"></i>
        <i class="sp-heat sp-heat--before"></i>
        ...
      </div>
    </div>
  </div>
  <div class="sp-heatmap-legend"><span>Less</span><i class="sp-heat" data-level="0"></i><i class="sp-heat" data-level="1"></i><i class="sp-heat" data-level="2"></i><i class="sp-heat" data-level="3"></i><i class="sp-heat" data-level="4"></i><span>More</span></div>
</section>
```

Rules:

- 7 rows (Monday at top, Sunday at bottom; ISO weeks match the existing weekly grouping). Cells flow top to bottom, then left to right: `grid-auto-flow: column`.
- The first column starts on the Monday given by Rust `from`. Rust always returns a dense array starting on a Monday, so the presenter just emits cells in order.
- Days after today in the last column: do not emit (the grid simply ends).
- Days before `first_recorded`: emit `<i class="sp-heat sp-heat--before"></i>` (invisible, keeps alignment). If `first_recorded` is null (no history at all), do not render the grid; render the empty state "No activity recorded yet."
- Days with zero tokens on or after `first_recorded`: `data-level="0"`, title `"<Mon d>: no recorded activity"`. Never "0 tokens".
- Month labels: a label at the column of the first Monday of each month. Rust does not need to send them; computing "which column holds the first week of a month" from the dense date list is formatting.
- Tooltip text: `"<Mon d>: <tokens> tokens · $<cost> est. · <turns> turns"`. Drop the cost part when `cost_usd` is 0 and `cost_basis` is `unknown`.
- Click on a cell: if the Usage period details list (`.sp-activity-breakdown`) contains `[data-date="<date>"]`, open that `<details>` and scroll it into view. Otherwise do nothing. One delegated handler in `console-app.js`.
- Keyboard: the scroll container is focusable (`tabindex="0"`) so it can be scrolled with arrow keys. Per-cell arrow navigation is out of scope.
- On load, scroll `.sp-heatmap-scroll` fully right (`el.scrollLeft = el.scrollWidth`) so today is visible on narrow screens.

```css
.sp-heatmap-stats { display: flex; flex-wrap: wrap; gap: var(--sp-6); margin-bottom: var(--sp-5); }
.sp-heatmap-stats div { display: grid; }
.sp-heatmap-stats strong { font-size: var(--sp-fs-title); font-weight: 600; color: var(--sp-text); font-variant-numeric: tabular-nums; }
.sp-heatmap-stats span { font-size: var(--sp-fs-meta); color: var(--sp-text-3); }
.sp-heatmap-scroll { overflow-x: auto; padding-bottom: var(--sp-2); }
.sp-heatmap-months { display: grid; grid-template-columns: repeat(var(--weeks), 14px); margin-left: 32px; height: 16px; font-size: var(--sp-fs-meta); color: var(--sp-text-3); }
.sp-heatmap-months span { grid-column: var(--col); white-space: nowrap; }
.sp-heatmap-body { display: flex; gap: var(--sp-2); }
.sp-heatmap-days { display: grid; grid-template-rows: repeat(7, 11px); row-gap: 3px; width: 24px; font-size: 10px; line-height: 11px; color: var(--sp-text-3); }
.sp-heatmap-grid { display: grid; grid-template-rows: repeat(7, 11px); grid-auto-flow: column; grid-auto-columns: 11px; gap: 3px; }
.sp-heat { display: block; width: 11px; height: 11px; border-radius: 2px; background: var(--sp-heat-0); outline: 1px solid rgb(255 255 255 / .03); outline-offset: -1px; }
.sp-heat[data-level="1"] { background: var(--sp-heat-1); }
.sp-heat[data-level="2"] { background: var(--sp-heat-2); }
.sp-heat[data-level="3"] { background: var(--sp-heat-3); }
.sp-heat[data-level="4"] { background: var(--sp-heat-4); }
.sp-heat--before { visibility: hidden; }
.sp-heatmap-grid .sp-heat:not(.sp-heat--before) { cursor: pointer; }
.sp-heatmap-grid .sp-heat:hover { outline: 1px solid var(--sp-text-2); }
.sp-heatmap-legend { display: flex; align-items: center; justify-content: flex-end; gap: 3px; margin-top: var(--sp-2); font-size: var(--sp-fs-meta); color: var(--sp-text-3); }
.sp-heatmap-legend span { margin: 0 var(--sp-1); }
```

Note: the 10px font in `.sp-heatmap-days` is the single allowed exception to the type scale, because the rows are 11px tall.

**Today hour strip** (when the period selector is "Today"): 24 cells in one row, same `.sp-heat` cells, labels `00`, `06`, `12`, `18` under columns 1, 7, 13, 19. Uses `report.daily[*].level` (section 8.2). Markup: `<div class="sp-heatstrip">` with `display: grid; grid-template-columns: repeat(24, 1fr); gap: 3px;` and cells `height: 28px; width: auto;`.

**Mini variant** for Home: `renderHeatmap(data, { weeks: 12 })` renders only the last 12 columns, no stats row, no month labels, no legend.

### 5.9 Empty state

```html
<div class="sp-empty"><p>No sessions yet.</p><button type="button" class="btn-primary" data-action="...">New session</button></div>
```

`.sp-empty { display: grid; justify-items: start; gap: var(--sp-3); padding: var(--sp-5); border: 1px dashed var(--sp-border); border-radius: var(--sp-radius-card); color: var(--sp-text-2); }`

---

## 6. Information architecture

### 6.1 Seven places become four

| New place | Internal view key | Rail label | Contains | Replaces |
| :--- | :--- | :--- | :--- | :--- |
| Home | `overview` (keep the key) | Home | Provider cards, Needs you / Running / Recent sessions, 3 tiles, 12-week mini grid | Dashboard |
| Workspace | `sessions` (keep) | Workspace | Conversation list, chat, terminal, launcher | Workspace |
| Usage | `usage` (keep) | Usage | Period selector, full provider cards, activity grid, totals, spend by agent, subscription value, projects table | Usage, Limits, Projects |
| Settings | `settings` (keep) | Settings | Providers (limit sources), Appearance, Devices and Remote, Automations, Pricing, Data and export, Account, Guide | Profile, Automations |

Keep the internal keys so stored URLs and `sessionStorage` values keep working. Only the labels and contents change.

### 6.2 Redirects

In `console-app.js`, before the existing `if ([...].includes(initialView))` check (~line 1725), and at the top of `setView`, map old keys:

```js
const VIEW_ALIASES = { limits: 'usage', projects: 'usage', automations: 'settings' };
// in setView(view, ...): view = VIEW_ALIASES[view] || view;
```

Also map the hash on load the same way. Remove `limits`, `projects`, `automations` from the rail. Keep `auth` as is (it is reached from the account entry).

### 6.3 Rail

Fixes audit D8 (rail overlays content). New behavior:

- Collapsed by default: 64px wide, icons only, label in `title` and `aria-label`.
- Expanded: 208px, toggled ONLY by the existing `#navigation-toggle` button (no hover expansion). State stored in `localStorage` key `gnomcode-rail-expanded` inside try/catch.
- The rail is a grid column, not an overlay: `.sp-console-workspace { display: grid; grid-template-columns: var(--rail-w, 64px) minmax(0, 1fr); }` and `body[data-rail="expanded"] { --rail-w: 208px; }`. Content moves when the rail expands; nothing is ever covered.
- Delete every CSS rule that expands the rail on `:hover` or `:focus-within` (grep `sp-console-nav` and `rail-label` in `console.html` and `gnomcode-console.css`).
- Rail order top to bottom: brand mark, Home, Workspace, Usage, (flex spacer), Settings, account entry, toggle.
- Active item: `--sp-surface-2` background, `--sp-text` color, 3px left indicator in `--sp-text`. Inactive: `--sp-text-3`, hover `--sp-text-2`.
- Under 720px width: keep the current touch toggle behavior (the rail becomes an off-canvas drawer). Do not redesign mobile navigation.

### 6.4 Page frame

- `h1#console-title` shows the place name ("Home", "Workspace", "Usage", "Settings"). In `setView` change `titles` to `{ overview: 'Home', sessions: 'Workspace', usage: 'Usage', settings: 'Settings', auth: 'Account' }`.
- Page content wrapper: `.sp-console-content > *:not(.ide-layout) { max-width: var(--sp-content-max); margin-inline: auto; }` with `padding: var(--sp-6) var(--sp-gutter)`; on phones `padding: var(--sp-4)`.
- The Workspace view (`.ide-layout`) stays full width.
- The period selector (`.timeline-bar`) shows only on Usage. Change its `data-local-view` from `overview,usage,projects` to `usage`. Home always uses "Today" for tiles (it fetches `range=today` separately, section 7.1).

---

## 7. View specifications

Wireframes use plain ASCII. Widths are approximate desktop (1440px window).

### 7.1 Home

```
+------+-------------------------------------------------------------------+
| rail |  Home                                                             |
|      |                                                                   |
|      |  Limits                                          Updated 1m ago   |
|      |  +-------------------+ +-------------------+ +-------------------+|
|      |  | C Claude Code  OK | | C Codex       LOW | | A Antigravity  -- ||
|      |  | Session  42% left | | Session   8% left | | Limits are off.   ||
|      |  | [=====|-----]     | | [=--------]       | | [gnomcode limits  ||
|      |  | Resets in 2h 14m  | | Resets in 41m     | |  enable antigr..] ||
|      |  | Weekly   71% left | | Weekly   55% left | |                   ||
|      |  | [=======|--]      | | [=====|----]      | |                   ||
|      |  +-------------------+ +-------------------+ +-------------------+|
|      |                                                                   |
|      |  Needs you (1)                                                    |
|      |  +---------------------------------------------------------------+|
|      |  | o Approve file write   fleethud . Claude Code    [==]  2m   > ||
|      |  +---------------------------------------------------------------+|
|      |  Running (2)                                                      |
|      |  | o ...                                                         ||
|      |  Recent                                              View all >   |
|      |  | o ... (max 6 rows)                                            ||
|      |                                                                   |
|      |  Today                                                            |
|      |  [ Tokens 2.4M ] [ Est. spend $3.10 ] [ Active sessions 3 ]       |
|      |  Last 12 weeks  [mini heat grid]                                  |
+------+-------------------------------------------------------------------+
```

Order is fixed: Limits, Needs you, Running, Recent, Today. Rules:

- "Needs you" and "Running" sections are omitted entirely when empty. "Recent" shows at most 6 rows and a `View all` link that calls `setView('sessions')`.
- If there are zero sessions ever: show `sp-empty` "No sessions yet." with a primary button "New session" that opens the launcher (reuse the existing launcher action).
- Provider cards use the full (non-compact) variant but without the footer cost line (`{ cost: false }` option) to keep Home light.
- Tiles: "Tokens today", "Est. spend today", "Active sessions". Values from `GET /api/v1/report?range=today` totals and the count of sessions with `home_group` in `needs_you` or `running`.
- Delete from Home: `#overview-summary` paragraph-style summary, the onboarding sign-in button block (`#dashboard-signin` moves to Settings > Account), `#daily-activity` bar chart, ROI cards.
- HTML: replace the overview blocks in `console.html` (`#overview-summary`, `.sp-overview-live`, the `overview` entry of `#daily-activity`) with:

```html
<div data-local-view="overview" class="sp-home">
  <section class="sp-section" id="home-limits"></section>
  <section class="sp-section" id="home-sessions"></section>
  <section class="sp-section" id="home-today"></section>
</div>
```

### 7.2 Workspace

Behavior changes (audit D7):

- Opening Workspace with no active chat shows the conversation list on the left and, in the main area, the most recent conversation if it was opened in the last 24 hours, otherwise an `sp-empty` "Pick a conversation or start a new one." with a primary "New session" button.
- The launcher (`#ide-launcher`) is hidden by default and opened by the "New" button at the top of the sidebar. It is no longer the landing screen.
- The conversation list is expanded by default on desktop. Remember collapsed state in `localStorage` key `gnomcode-chat-list-collapsed` (try/catch).

Visual changes:

- Sidebar: 300px, `--sp-surface` background, no border box around the whole layout (remove `border:1px solid var(--sp-border)` from `.ide-layout`, keep only the divider between sidebar and chat).
- Sidebar header: search input (full width, 36px tall here is fine since it is a filter, but keep a 44px hit area via padding) and a "New" primary button.
- List rows use `.sp-session-row` (section 5.7) without the context bar, grouped under small headings "Needs you", "Running", "Today", "Earlier" (headings: `--sp-fs-meta`, uppercase is NOT allowed; use sentence case, `--sp-text-3`, 600 weight).
- Chat header: title on the left, compact provider card (`compact: true`) for the current provider on the right, collapsed into a single line: `Session 42% left · Weekly 71% left`. Use `renderConversationLimits` restyled with `sp-chip` per window (state class from Rust).
- Do not touch transcript bubbles, composer, terminal, model picker logic. Only adjust their spacing to the new scale if they use non-scale values.

### 7.3 Usage

```
Usage                                         [ Period: Last 30 days v ] [Export v]
Limits
  (full provider cards with cost footer, same grid as Home)
Activity
  (stats row) (53-week grid) (legend)
  (Today period only: 24-hour strip instead of the grid)
Totals for <period>
  [ Tokens ] [ Est. spend ] [ Cache saved ]
By agent
  (existing providers-container content, restyled as a simple table: agent, tokens, est. spend, share bar)
Subscription value
  (existing ROI cards restyled as .sp-card, "Edit" link to Settings > Pricing)
Projects
  (existing projects table restyled; moved here from the Projects view)
Details
  <details> existing per-interval breakdown list (.sp-activity-breakdown)
```

- Move `#quota-table` (was `data-local-view="limits"`) into Usage as the "Limits" section, rendered with the new provider cards.
- Move `#view-projects` content into a Usage section `#usage-projects`. Keep the existing `loadProjects()` and call it when Usage is shown.
- Activity grid is independent of the period selector (always the last year). The totals, by-agent, projects and details follow the selector, as today.
- Export buttons (`export:csv`, `export:json`) move from Profile into a small menu button next to the period selector. Keep the existing `data-action` handlers.
- Remove all inline `style=""` attributes from the moved markup and replace them with classes.

### 7.4 Settings

One column, max 720px wide, sections in this order. Each section is `<section class="sp-section sp-settings-section">` with an `h2` and at most one helper sentence.

1. **Providers.** One row per provider from `GET /api/v1/providers/overview`: name, current limit source (text from Rust `source_label`, e.g. "Status line", "Provider API", "Local logs", "Off"), and when off, the `sp-copy` button with `enable_command`. No toggles (there is no HTTP API to toggle; see section 2).
2. **Appearance.** Existing theme control.
3. **Devices and Remote.** Existing `.profile-devices` content unchanged in behavior.
4. **Automations.** Entire existing `#view-automations` content moved here unchanged in behavior. Its loader (`refreshAutomation` or the code that runs when `activeView === 'automations'`) must now run when Settings is shown. Grep for `'automations'` in `console-app.js` and update every condition.
5. **Pricing.** Existing custom pricing and subscription plan settings.
6. **Data.** Reload view, Resync local data, last-updated line. (Exports moved to Usage.)
7. **Account.** Existing account entry and `#dashboard-signin`.
8. **Guide.** Existing "Reopen guide" control.

Remove `#view-automations` and `#view-projects` `<main>` elements after their content has moved, and remove their `aria-controls` references.

---

## 8. Data contracts (Rust)

All logic below lives in Rust. Add doc comments to every `pub` item. Use `chrono` (already used). No `unwrap`/`expect`.

### 8.1 Quota state and pace (`crates/core`)

Create `crates/core/src/quota.rs` and add `pub mod quota;` in `crates/core/src/lib.rs`. Core is pure: no I/O, `now` is a parameter.

```rust
//! Quota state and pace derived from a snapshot (SPEC 3.9).

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;

use crate::{QuotaSnapshot, QuotaWindow, TrustLevel};

/// Remaining percentage at or below which a window is reported as low.
pub const LOW_REMAINING_PCT: f64 = 20.0;
/// Readings older than this are shown as last seen (SPEC 3.9).
pub const STALE_AFTER: Duration = Duration::minutes(2);
/// Pace is not reported before this fraction of a window has elapsed.
const MIN_PACE_ELAPSED: f64 = 0.05;
/// Usage within this many percentage points of even pace counts as on pace.
const ON_PACE_BAND_PCT: f64 = 5.0;

/// Display state of one quota window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuotaState { Ok, Low, Exhausted, ResetAwaiting, Unknown }

/// Whether usage is ahead of, behind, or at even pace for the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PaceVerdict { OnPace, Ahead, Behind }

/// Pace of usage across a window.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct QuotaPace {
    /// Remaining percentage expected at `now` if usage were even across the window.
    pub pace_remaining_pct: f64,
    /// Actual remaining minus expected remaining; negative means using faster than pace.
    pub delta_pct: f64,
    /// Verdict derived from `delta_pct`.
    pub verdict: PaceVerdict,
    /// When the window would run out at the current rate, if before the reset.
    pub runs_out_at: Option<DateTime<Utc>>,
}

/// Length of a quota window.
#[must_use]
pub fn window_length(window: QuotaWindow) -> Duration { ... } // FiveHour 5h, Weekly 7d, Monthly 30d

/// Remaining percentage, or None when the reading cannot describe capacity.
#[must_use]
pub fn remaining_pct(s: &QuotaSnapshot, now: DateTime<Utc>) -> Option<f64> { ... }

/// Display state for a snapshot.
#[must_use]
pub fn quota_state(s: &QuotaSnapshot, now: DateTime<Utc>) -> QuotaState { ... }

/// Pace for a snapshot, when it can be computed honestly.
#[must_use]
pub fn quota_pace(s: &QuotaSnapshot, now: DateTime<Utc>) -> Option<QuotaPace> { ... }

/// Whether the reading is older than `STALE_AFTER`.
#[must_use]
pub fn is_stale(s: &QuotaSnapshot, now: DateTime<Utc>) -> bool { ... }
```

Exact algorithms:

- `reset_passed = resets_at.is_some_and(|r| r <= now)`.
- `remaining_pct`: if `reset_passed` or `basis == Unknown` or `!used_pct.is_finite()` return None; else `Some((100.0 - used_pct).clamp(0.0, 100.0))`. This is the logic currently in `quota_views`; move it here and make `quota_views` call it.
- `quota_state`, checked in this order:
  1. `reset_passed` -> `ResetAwaiting`
  2. `remaining_pct` is None -> `Unknown`
  3. remaining `<= 0.0` -> `Exhausted`
  4. remaining `< LOW_REMAINING_PCT` -> `Low`
  5. otherwise `Ok`
- `quota_pace` returns None unless ALL hold: state is `Ok` or `Low`; `basis` is `Exact` or `Api`; `resets_at` is Some.
  - `len = window_length(window)`; `start = resets_at - len`; `elapsed = (now - start).num_seconds() as f64 / len.num_seconds() as f64`, clamped to `0.0..=1.0`.
  - If `elapsed < MIN_PACE_ELAPSED` return None.
  - `pace_remaining_pct = 100.0 * (1.0 - elapsed)`.
  - `delta_pct = remaining - pace_remaining_pct`.
  - verdict: `delta_pct < -ON_PACE_BAND_PCT` -> `Ahead` (using faster than pace); `delta_pct > ON_PACE_BAND_PCT` -> `Behind` (in reserve); else `OnPace`.
  - `runs_out_at`: only when verdict is `Ahead` and `used_pct > 0`: `rate = used_pct / (now - start).num_seconds() as f64` (percent per second); `secs = (remaining / rate)`; `eta = now + Duration::seconds(secs as i64)`; return `Some(eta)` only if `eta < resets_at`, else None.
- `is_stale`: `now - observed_at > STALE_AFTER`.

Card-level state (worst window) lives in the daemon view (8.1.1), ordered by severity: `Exhausted > Low > Unknown > ResetAwaiting > Ok`. Add a `pub fn severity(self) -> u8` on `QuotaState` returning 4,3,2,1,0 in that order.

Unit tests in `crates/core/src/quota.rs` (`#[cfg(test)] mod tests`), each a separate `#[test]`:

1. `state_ok_when_plenty_left` (used 30 -> Ok).
2. `state_low_below_twenty_left` (used 85 -> Low).
3. `state_exhausted_at_full_use` (used 100 -> Exhausted).
4. `state_reset_awaiting_after_reset_time` (resets_at = now - 1m -> ResetAwaiting, remaining None).
5. `state_unknown_for_unknown_basis`.
6. `state_unknown_for_nan_used`.
7. `pace_none_for_estimated_basis`.
8. `pace_none_in_first_five_percent_of_window` (five_hour, 10 minutes elapsed).
9. `pace_on_pace_when_usage_even` (weekly, 50% elapsed, used 50 -> OnPace, pace_remaining 50).
10. `pace_ahead_with_run_out_before_reset` (five_hour, 50% elapsed, used 80 -> Ahead, runs_out_at Some and before resets_at).
11. `pace_behind_when_in_reserve` (weekly, 50% elapsed, used 20 -> Behind).
12. `pace_ahead_without_run_out_when_reset_comes_first` (weekly, 90% elapsed, used 96 -> Ahead; compute and assert runs_out_at is None if eta >= resets_at; pick numbers so it is).
13. `stale_after_two_minutes`.
14. `severity_orders_exhausted_first`.

Build snapshots with a small test helper `fn snap(window, used, resets_in: Duration, basis) -> QuotaSnapshot` using a fixed `now` (`Utc.with_ymd_and_hms(2026, 11, 12, 12, 0, 0)`), never `Utc::now()`.

#### 8.1.1 `QuotaPoolView` additions (`crates/daemon/src/api.rs`)

Extend the existing struct (do not rename existing fields; Remote and the web console read them):

```rust
pub(crate) struct QuotaPoolView {
    #[serde(flatten)]
    snapshot: gnomcode_core::QuotaSnapshot,
    remaining_pct: Option<f64>,
    reset_passed: bool,
    // new
    state: gnomcode_core::quota::QuotaState,
    pace: Option<gnomcode_core::quota::QuotaPace>,
    stale: bool,
    /// Display label, e.g. "Session", "Weekly", "Gemini Pro · Weekly".
    label: String,
}
```

`label`: `five_hour -> "Session"`, `weekly -> "Weekly"`, `monthly -> "Monthly"`. If the provider has more than one distinct `limit_id` among its views, prefix with a title-cased pool name: replace `_` and `-` with spaces, capitalize each word (`gemini_pro` -> `Gemini Pro`), then `" · "`. Compute this in `quota_views` after filtering. `now` is computed once at the top of `quota_views` and passed to every core function (today the function calls `Utc::now()` twice; fix that).

Add daemon tests next to the existing quota tests in `api.rs` (grep `fn quota_views` tests or add `#[cfg(test)]` cases): `label_prefixes_pool_when_provider_has_several`, `label_plain_for_single_pool`, `view_carries_state_and_pace`.

### 8.2 Activity: levels and heatmap

#### 8.2.1 `level` on `DailyUsage` (`crates/store/src/lib.rs`)

Add to `DailyUsage`:

```rust
/// Activity intensity 0..=4 for grid cells, computed in Rust.
#[serde(default)]
pub level: u8,
```

Add a pure function in store (near `group_activity`):

```rust
/// Intensity level 0..=4 for `value` relative to `max`, square-root scaled so outliers do not flatten the grid.
pub(crate) fn activity_level(value: u64, max: u64) -> u8 {
    if value == 0 || max == 0 { return 0; }
    let ratio = (value as f64 / max as f64).sqrt();
    ((ratio * 4.0).ceil() as u8).clamp(1, 4)
}
```

Set `level` wherever `chart_pct` is set today (~line 2945): `day.level = activity_level(day.total_tokens, maximum)`. Set `level: 0` in every other `DailyUsage { .. }` literal (grep `chart_pct: 0.0` and add `level: 0` next to each).

Tests: `activity_level_zero_for_no_tokens`, `activity_level_four_for_max`, `activity_level_one_for_tiny_values` (1 of 1_000_000 -> 1), `activity_level_sqrt_scaled` (max/4 -> 2, max/16 -> 1).

Update `web/lib/console-presentation.d.ts` type for daily rows to include `level?: number`.

#### 8.2.2 Heatmap query (`crates/store/src/lib.rs`)

Add a struct and a method on the database type that already serves the timeline report (find the function that builds `TimelineReport`, ~line 2654, and the SQL it uses for daily rows; reuse that SQL fragment, do not write a second definition of "tokens including cache exactly once").

```rust
/// One UTC day in the activity grid.
#[derive(Debug, Clone, Serialize)]
pub struct HeatmapDay {
    /// UTC date `YYYY-MM-DD`.
    pub date: String,
    /// Tokens including cache exactly once.
    pub total_tokens: u64,
    /// Recorded API-equivalent cost.
    pub cost_usd: f64,
    /// Pricing provenance.
    pub cost_basis: TrustLevel,
    /// Measured invocations.
    pub turns: u64,
    /// Intensity 0..=4.
    pub level: u8,
}

/// Summary figures for the activity grid.
#[derive(Debug, Clone, Serialize)]
pub struct HeatmapStats {
    pub total_tokens: u64,
    pub active_days: u32,
    pub busiest: Option<HeatmapBusiest>,
    pub current_streak: u32,
    pub longest_streak: u32,
}

/// Busiest recorded day.
#[derive(Debug, Clone, Serialize)]
pub struct HeatmapBusiest { pub date: String, pub total_tokens: u64 }

/// Dense daily activity for the trailing year, starting on a Monday.
#[derive(Debug, Clone, Serialize)]
pub struct ActivityHeatmap {
    pub from: String,
    pub to: String,
    pub first_recorded: Option<String>,
    pub days: Vec<HeatmapDay>,
    pub stats: HeatmapStats,
}

/// Build the activity grid for the 53 ISO weeks ending on `today` (UTC).
pub fn activity_heatmap(&self, today: chrono::NaiveDate) -> Result<ActivityHeatmap, Error>
```

Algorithm:

1. `this_monday = today - Duration::days(today.weekday().num_days_from_monday() as i64)`; `from = this_monday - Duration::weeks(52)`; `to = today`.
2. Query daily rows (UTC dates) between `from` and `to` inclusive with the shared SQL. Put rows in a `BTreeMap<NaiveDate, row>`.
3. Query `first_recorded`: the minimum UTC date of any recorded usage ever (a separate `SELECT MIN(...)`). None if nothing recorded.
4. Build `days` densely: for each date from `from` to `to`, take the row or a zero row (`cost_basis: TrustLevel::Unknown`).
5. `max = days.iter().map(total_tokens).max()`; `level = activity_level(total_tokens, max)` for each.
6. Stats over `days` with `date >= first_recorded` only:
   - `total_tokens` = sum; `active_days` = count of `total_tokens > 0`.
   - `busiest` = the max day (earliest date wins ties); None if all zero.
   - `longest_streak` = longest run of consecutive active days.
   - `current_streak` = run of consecutive active days ending at `today`; if `today` is not active, the run ending at `today - 1`; 0 if neither.
7. Wrap any SQL strings in `obfstr!` the same way the neighboring queries do (follow the existing pattern in this file).

Make the stats a separate pure function `fn heatmap_stats(days: &[HeatmapDay], first_recorded: Option<NaiveDate>, today: NaiveDate) -> HeatmapStats` so it is unit-testable without a DB.

Tests (store crate, use an in-memory/temp DB the way neighboring tests do):

1. `heatmap_starts_on_monday_and_ends_today` (assert `days.len()` between 365 and 371, first date is a Monday, last is today).
2. `heatmap_is_dense_with_zero_days`.
3. `heatmap_first_recorded_none_when_empty`.
4. `heatmap_stats_streak_across_gap` (active Mon, Tue, gap Wed, active Thu, Fri; today Fri -> current 2, longest 2).
5. `heatmap_current_streak_counts_from_yesterday_when_today_empty`.
6. `heatmap_busiest_prefers_earliest_on_tie`.
7. `heatmap_ignores_days_before_first_recorded_in_stats`.
8. `heatmap_uses_utc_day_boundaries` (an event at 23:30 UTC and one at 00:30 UTC next day land on different days).

#### 8.2.3 Endpoint

In `crates/daemon/src/api.rs` add `.route("/api/v1/activity/heatmap", get(activity_heatmap_handler))` next to `/api/v1/report`. Handler: `validate_auth` first (copy the pattern from `quota_handler`), lock db, call `activity_heatmap(Utc::now().date_naive())`, map errors to `StatusCode::INTERNAL_SERVER_ERROR` exactly like neighbors, return `Json`. Add a handler test that asserts 401 without a token and 200 with one (copy an existing handler test pattern).

### 8.3 Home grouping (`crates/core/src/presence.rs` and session views)

Add to `presence.rs`:

```rust
/// Where a session appears on the Home board.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HomeGroup { NeedsYou, Running, Recent }

impl SessionPresence {
    /// Home board group for this presence.
    #[must_use]
    pub const fn home_group(self) -> HomeGroup {
        match self {
            Self::AwaitingApproval => HomeGroup::NeedsYou,
            Self::Running | Self::RecentlyIdle => HomeGroup::Running,
            Self::Inactive | Self::Ended => HomeGroup::Recent,
        }
    }
}
```

Find where the sessions/conversations API builds its per-session JSON (`list_sessions_handler` and `list_conversations_handler` in `api.rs`; follow the code to the view struct that has `is_ongoing`). Wherever `is_ongoing` is derived from a `SessionPresence`, add a sibling field `home_group` from `presence.home_group()`. Test: `home_group_maps_every_presence` in `presence.rs` covering all five variants.

### 8.4 Provider overview endpoint

`GET /api/v1/providers/overview`. Leave `/api/v1/quota` in place for Remote and the web console.

```rust
#[derive(Serialize)]
pub(crate) struct ProviderOverview {
    provider: String,             // slug, e.g. "claude-code"
    display_name: String,         // "Claude Code"
    plan: Option<String>,         // first non-None plan among its snapshots
    state: QuotaState,            // worst window by severity; Unknown when no windows
    windows: Vec<QuotaPoolView>,  // sorted: five_hour, weekly, monthly, then label
    updated_at: Option<DateTime<Utc>>, // max observed_at
    source_label: String,         // "Local logs" | "Status line" | "Provider API" | "Language server" | "Off"
    enable_hint: Option<String>,  // shown when windows is empty
    enable_command: Option<String>,
    cost: Option<ProviderCost>,   // None when no recorded spend in 30 days
}

#[derive(Serialize)]
pub(crate) struct ProviderCost { today_usd: f64, last_30_days_usd: f64, basis: TrustLevel }
```

Rules:

- Providers listed: always `claude-code`, `codex`, `antigravity` (in that order), then any other provider that has quota windows, sorted by slug.
- Display names and hints move from the JS `PROVIDERS` table in `console-presentation.js` into a Rust `match` in this handler module:
  - `claude-code`: "Claude Code"; hint "Limits are off for Claude Code."; command `gnomcode limits enable claude`.
  - `codex`: "Codex"; hint "Appears after your next Codex request."; command None.
  - `antigravity`: "Antigravity"; hint "Limits are off for Antigravity."; command `gnomcode limits enable antigravity`. If `vendor_limits::enabled("antigravity")` is true but there are no windows, hint becomes "Start an Antigravity agent to read limits." and command None.
  - For Claude, if `vendor_limits::enabled` (check the exact provider key used by `vendor_limits.rs`) is true and there are no windows, hint "Waiting for the first Claude reading." and command None.
  - Unknown slug: title-case the slug, hint "No reading yet.", command None.
- `source_label`: `codex` -> "Local logs"; `claude-code` -> "Provider API" if vendor limits enabled, else "Status line" if any window has `basis == Exact`, else "Off"; `antigravity` -> "Language server" if enabled, else "Off"; others -> "Local logs".
- Cost: from the store, per provider, recorded cost since local midnight (`today_usd`) and since now minus 30 days (`last_30_days_usd`). Reuse the provider breakdown the report already computes (`ProviderReport`) by calling the same store function with `range=today` and a 30-day range; do not write new SQL if that works. Verify that `ProviderReport.provider` uses the same slug as `QuotaSnapshot.provider` with a test; if they differ, add a mapping function with a test. `basis` = the report's `cost_basis` (Unknown if any unpriced turns).

Tests: `overview_lists_core_providers_without_data`, `overview_state_is_worst_window`, `overview_hint_changes_when_source_enabled` (if `vendor_limits::enabled` reads a file under `GNOMCODE_HOME`, set it to a temp dir in the test), `overview_windows_sorted_session_first`.

---

## 9. Phases and PRs

Each phase is one branch and one PR. Do them in order; each starts from an up-to-date `main` after the previous PR merged.

### Phase 0: preparation (no PR)

1. Make sure branch `fix/core-loop` is merged into `main` (it touches `gnomcode-console.css` and `console-presentation.js`). If not merged, stop and ask Wahaj.
2. `git checkout main && git pull`.
3. Run the full gate (10.1). It must pass on clean `main`. If it does not, stop and report; do not start on a red baseline.
4. Capture baseline screenshots (10.3) of Dashboard, Workspace, Usage, Limits, Projects, Automations, Profile, at 1440px and 390px, dark and light. Save them in the scratchpad, not in the repo. You will compare against them at the end.

### Phase 1: tokens and CSS consolidation. Branch `refactor/console-css`

Goal: no visual change; all console CSS in one file; new tokens present.

1. Add the new tokens from 4.1 to `web/app/gnomcode-tokens.css` (both dark and both light blocks).
2. Cut the entire inline `<style>...</style>` block from `crates/daemon/src/console.html` (from `<style>` after the favicon link to `</style>` before `<link rel="stylesheet" href="/gnomcode-tokens.css">`).
3. Paste it at the end of `web/app/gnomcode-console.css` under the header comment `/* ---- Local console (moved from console.html) ---- */`. Remove the `@import url("/gnomcode-tokens.css");` line from the moved block (the page already links the tokens file).
4. Scope check: the web console also loads `gnomcode-console.css`. Every moved selector that is generic (`.card`, `.btn-primary`, `.tab-btn`, `.nav-tabs`, `main`, `body`, `h1`, etc.) must be prefixed with `.sp-native-console ` (the body class of the local console) so it cannot affect the web console. Selectors starting with `#` ids that only exist in `console.html` can stay as they are.
5. Keep the order: the moved block goes after the existing rules so the cascade stays identical.
6. Run the gate, start the console (10.2), and compare every view against the baseline. Any difference is a bug in steps 2-5; fix it.
7. Run `npm run build` in `web/` and open `/console` in the web app (10.2) to confirm it looks unchanged.
8. Commit: `refactor(console): move local console styles into the shared stylesheet`.
9. Commit tokens separately: `feat(console): add state, spacing and heat tokens`.
10. SPEC: no change needed (no behavior change). Session record: yes.
11. PR title: `refactor(console): consolidate console CSS and add v2 tokens`.

### Phase 2: quota state and pace in Rust. Branch `feat/quota-state-pace`

1. Create `crates/core/src/quota.rs` exactly as in 8.1, with all 14 tests. Write the tests first, run `cargo test -p gnomcode-core quota` (check the real crate name in `crates/core/Cargo.toml`), see them fail, then implement.
2. Refactor `quota_views` in `api.rs` to use `quota::remaining_pct`, `quota_state`, `quota_pace`, `is_stale` with a single `now`. Add `state`, `pace`, `stale`, `label` fields (8.1.1) and the 3 daemon tests.
3. Make sure `conversation_quota` (right below `quota_views`) also returns `state` and `label` per window if it builds its own structs; follow it and add the fields so the chat header can use them.
4. Update `web/lib/console-presentation.d.ts` types for the new optional fields (`state?`, `pace?`, `stale?`, `label?`).
5. No UI change in this PR. Existing JS still works because fields are only added.
6. SPEC 3.9: apply text change 12.1.
7. Commits: `feat(core): quota state and pace from snapshots`, `feat(daemon): expose quota state, pace and labels`.

### Phase 3: provider cards. Branch `feat/console-provider-cards`

1. Add `/api/v1/providers/overview` (8.4) with its tests. Commit `feat(daemon): provider overview endpoint`.
2. In `web/lib/console-presentation.js` add, and export:
   - `renderMeterRow(win)` -> markup of 5.4. Inputs: one `QuotaPoolView`. Uses `win.state`, `win.label`, `win.remaining_pct`, `win.resets_at`, `win.pace`, `win.basis`, `win.observed_at`, `win.stale`. Escape every string with the existing `escape` helper. Value text table in 5.4. Pace text: `on_pace` -> `On pace`; `behind` -> `${Math.round(delta)}% in reserve`; `ahead` with `runs_out_at` -> `Runs out ${format(runs_out_at)}` where format is the same as the reset format; `ahead` without -> `${Math.round(-delta)}% ahead of pace`. `title` attribute: `${trust(win.basis)}. ${win.stale ? 'Last seen' : 'Seen'} ${relativeActivity(win.observed_at)}.`
   - `renderProviderCard(p, { compact = false, cost = true } = {})` -> markup of 5.5. Chip text: `ok` -> `OK`, `low` -> `Low`, `exhausted` -> `Exhausted`, `reset_awaiting` -> `Resetting`, `unknown` -> `No data`.
   - `renderProviderCards(list, options)` -> `<div class="sp-card-grid">` + cards, wrapped in a section with `sp-section-head` "Limits" and meta `Updated ${relativeActivity(maxUpdatedAt)}`.
   - `updateProviderCards(element, list, options)` using the existing `updateMarkup`.
3. Keep `renderQuota(pools, publishedAt, opts)` exported because the web console (`web/app/console/Client.tsx`, `QuotaPanel`) and Remote call it with raw pools. Re-implement it as: group pools by `provider` (formatting only), build a minimal overview object per provider `{ provider, display_name: provider, windows, state: worst-of(windows[].state) }`, and call `renderProviderCards`. For worst-of, use a fixed JS order array `['exhausted','low','unknown','reset_awaiting','ok']` on the Rust-supplied strings; this is selecting from Rust output, not computing state. If a pool has no `state` field (payload synced from an older desktop), treat it as `'unknown'` and render remaining % plainly with no color. Delete the old long non-compact branch and the old `PROVIDERS`, `limitTone`, `limitHeadline` code.
4. Rewrite `renderConversationLimits(limits)` to output one `sp-chip` per window: `<span class="sp-chip sp-chip--${state}" title="...">${label} ${headline}</span>`. Keep the accessible "Provider measured" text in the `title`, so `calm-presentation.test.mjs` still passes; if that test asserts on exact markup that changed, update the test to the new markup but keep its intent (provenance present, no EXACT badge).
5. `console-app.js`: `loadQuota()` now fetches `/api/v1/providers/overview` and calls `updateProviderCards(document.getElementById('quota-table'), data, { cost: true })`. Add the delegated `[data-copy]` handler (5.5).
6. Add CSS from 5.1-5.5 to the v2 section of `gnomcode-console.css`.
7. JS tests in a new file `web/test/provider-cards.test.mjs`:
   - renders `% left` and `Resets in` for an ok window;
   - exhausted window shows `Exhausted` and class `sp-meter-row--exhausted`;
   - reset_awaiting shows `Awaiting next reading`;
   - no pace element when `pace` is null; pace element with `--pace:` when present;
   - empty provider renders `data-copy` with the command, and no button when `enable_command` is null;
   - no emoji code points in output (`/\p{Extended_Pictographic}/u` must not match);
   - every interpolated string is escaped (pass `display_name: '<b>x</b>'`, assert `&lt;b&gt;`).
8. SPEC 3.9 and 3.16: apply 12.2.
9. Commits: `feat(console): provider limit cards`, `test(console): provider card presentation`.

### Phase 4: activity grid. Branch `feat/console-activity-grid`

1. Store: `activity_level` + `level` on `DailyUsage` (8.2.1) with tests. Commit `feat(store): activity intensity levels`.
2. Store: `activity_heatmap` + `heatmap_stats` (8.2.2) with tests. Commit `feat(store): trailing-year activity heatmap`.
3. Daemon: endpoint (8.2.3) with tests. Commit `feat(daemon): activity heatmap endpoint`.
4. Presenters in `console-presentation.js`, exported:
   - `renderHeatmap(data, { weeks = 53, stats = true, legend = true, months = true } = {})` -> 5.8 markup. If `weeks < 53`, slice `days` to the last `weeks * 7` entries counted from the end of the first column alignment: compute `start = days.length - ((weeks - 1) * 7 + todayIndex + 1)` where `todayIndex` = weekday of the last date (Mon=0); clamp to 0. Number formatting: reuse the existing `tokens()` compact formatter. Busiest date as `Oct 3` via `toLocaleDateString('en-US', { month: 'short', day: 'numeric', timeZone: 'UTC' })`.
   - `renderHourStrip(report)` -> 5.8 Today strip from `report.daily` (uses `level`).
   - `updateHeatmap(element, data, options)`.
5. Change `renderActivity(report)`: remove the `.sp-activity-bars` block entirely. For `range_label === 'Today'` render `renderHourStrip(report)`; otherwise render nothing in place of the chart (the grid is a separate section). Keep the `sp-activity-breakdown` details list and totals. Make sure each detail item has `data-date` (it already does).
6. `console-app.js`: add `loadHeatmap()` fetching `/api/v1/activity/heatmap`; call it when Usage or Home is shown and then at most every 60 seconds (keep a `lastHeatmapAt` timestamp; do NOT fetch it on every 2-second poll). Render into a new `<div id="usage-heatmap" data-local-view="usage">` placed right after the Usage limits section. After rendering, scroll `.sp-heatmap-scroll` to the end. Add the delegated cell-click handler (5.8).
7. CSS from 5.8.
8. Web console: if `web/app/console/Client.tsx` renders `ActivityPanel`, leave it on `renderActivity` (it will lose the bars and keep totals). Do not add the heatmap to the web console in this PR (its data comes through sync, which does not carry heatmap data). Note it in section 14.
9. Tests `web/test/heatmap.test.mjs`:
   - 7-row grid: count of `.sp-heat` cells equals `days.length` (pre-history included);
   - pre-history cells have `sp-heat--before`;
   - zero day title says `no recorded activity` and never `0 tokens`;
   - `first_recorded: null` renders the empty state text;
   - `weeks: 12` renders at most 84 cells and no stats/legend;
   - month labels appear for each month boundary;
   - update `calm-presentation.test.mjs` case "activity breakdown is optional while the chart is available immediately" so it no longer expects bars but still expects the `sp-activity-breakdown` disclosure.
10. SPEC 3.16: apply 12.3.

### Phase 5: Home board. Branch `feat/console-home`

1. Core `HomeGroup` + `home_group()` + test (8.3). Daemon adds `home_group` to session views; add/extend a handler test asserting the field exists.
2. Presenters: `renderHomeSessions(sessions)` -> three groups (5.7 rows), using `home_group` from each session; Recent limited to 6 rows; `View all` link. `renderTodayTiles({ report, activeCount })` -> 5.6 tiles. Both exported with `update*` twins.
3. `console.html`: replace the overview blocks as specified in 7.1. Move `#dashboard-signin` into the Settings Account section.
4. `console-app.js`: when `activeView === 'overview'` in the poll:
   - limits: reuse the overview data fetched for cards (`updateProviderCards('#home-limits', data, { cost: false })`), one fetch per poll shared with Usage;
   - sessions: reuse the data `loadSessions`/`loadOngoing` already fetch; do not add a second sessions fetch;
   - today tiles: fetch `/api/v1/report?range=today` at most every 10 seconds;
   - mini grid: reuse the cached heatmap data (Phase 4) with `{ weeks: 12, stats: false, legend: false, months: false }`.
5. Remove dead code: functions that only rendered the removed blocks (`updateSummary` usage for `#overview-summary`, the overview part of the bar chart). Keep exported presenter functions that `Client.tsx` still imports (grep `web/app` for each name before deleting anything).
6. Rename the rail label "Dashboard" to "Home" and the title mapping (6.4). Replace the Dashboard icon with a simple house path: `M3 11l9-8 9 8v10H3z M9 21v-6h6v6`.
7. First-run guide (onboarding) text mentions "Dashboard"; update those words to "Home" and "Usage" accordingly. Change only words, not the guide structure.
8. Tests `web/test/home.test.mjs`: needs_you rows come first; empty groups omitted; recent capped at 6; zero sessions renders the `sp-empty` with "New session".
9. SPEC 3.16: apply 12.4.

### Phase 6: navigation and page merges. Branch `feat/console-nav`

1. Implement the rail (6.3): remove hover expansion CSS, add grid column layout, `data-rail` attribute on `body`, toggle via `#navigation-toggle`, persisted in `localStorage` (try/catch).
2. Remove the Limits, Projects and Automations rail buttons. Add `VIEW_ALIASES` redirects (6.2). Rename "Profile" to "Settings".
3. Move content per 7.3 (Limits section and Projects into Usage, export buttons next to the period selector) and 7.4 (Automations, exports removed, sign-in into Settings). Update every `data-local-view` attribute accordingly. Grep `data-local-view=` and check each one.
4. Change `.timeline-bar` to `data-local-view="usage"`.
5. Update every `activeView === 'automations'` / `'projects'` / `'limits'` condition in `console-app.js` to the new home of that content.
6. Settings Providers section (7.4 item 1) using `/api/v1/providers/overview` (already fetched; reuse the cache).
7. Workspace behavior changes (7.2): list expanded by default, launcher behind "New", empty state in main area. This touches `showLauncher`, `renderIdeSessions`, and the view setup in `setView`; change only what is needed.
8. Remove inline `style=""` from all markup you moved.
9. Test: extend `web/test/audit-regressions.test.mjs` (or create `nav.test.mjs` reading `console.html` as text) to assert: exactly 4 `tab-btn` with `data-view` in the rail; no `:hover` rule that changes rail width in `gnomcode-console.css`; `VIEW_ALIASES` present in `console-app.js`.
10. ADR: create `docs/adr/NNNN-console-four-places.md` (next free number; copy `0000-template.md`) recording the 7-to-4 decision and the no-hover rail; add its row to SPEC section 9.
11. SPEC 3.16: apply 12.5.

### Phase 7: polish and docs. Branch `feat/console-polish`

1. Sweep `gnomcode-console.css` and `console.html` for font sizes outside the scale (`grep -nE "font-size: ?(9|10|11|13|16|17|18)px"`), replace with the nearest scale variable (except the documented 10px heatmap day labels and the 16px phone form inputs required by SPEC 3.16).
2. Sweep for paragraphs of explanatory copy in default views (grep for `<p>` inside the moved sections and for presenter strings longer than 12 words). Shorten or move into `title`/`<details>`.
3. Focus styles: `:focus-visible { outline: 2px solid var(--sp-accent); outline-offset: 2px; }` for buttons, links, `.sp-session-row`, `.sp-copy`, `.sp-heatmap-scroll` (add once, globally scoped under `.sp-native-console`).
4. Verify all hit targets are at least 44px (rail buttons, copy buttons, session rows, chips that are clickable).
5. Run the full visual verification (10.3) and fix everything found.
6. Update `DESIGN.md` with sections: tokens (4.1 summary), components list (5), anti-clutter rules (3.2), IA (6.1). Update `docs/design/console-workspace.md` "FIRST VIEWPORT" paragraph to mention the toggle-only rail and list-first Workspace.
7. Final session record with before/after screenshot descriptions (no images in the repo).

---

## 10. Verification

### 10.1 Gate (every commit)

From the repo root:

```
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

From `web/`:

```
npm test
npm run typecheck
npm run build
```

On Windows you may run `.agents/skills/rust-quality-gate/scripts/check.ps1` instead of the three cargo commands. All must pass. Paste the final summary lines of each into the PR description.

### 10.2 Running the console

- Local console: build and run the daemon via the CLI (`cargo run -p gnomcode -- console`; check `crates/cli/src/main.rs` for the exact `Console`/`Open` subcommand flags). It prints or opens a `127.0.0.1` URL with a token. Open it in the built-in browser.
- Use a temp `GNOMCODE_HOME` pointing at a scratch directory so you never write to the real `~/.spawnt` or agent folders. Seed synthetic data with the existing fixtures/tests helpers if available; if the console is empty, that is also a valid state to verify (empty states).
- Web console: `npm run dev` in `web/` and open `/console` (requires sign-in; if it cannot be reached without real credentials, skip it and say so in the PR; do not enter any real credentials).

### 10.3 Visual check (Phases 1, 3-7)

For each of Home, Workspace (list, chat open, launcher open), Usage (All time, Today), Settings:

1. Desktop 1440x900, dark.
2. Desktop 1440x900, light (`data-theme="light"` via the theme control).
3. Phone 390x844, dark.
4. Phone 390x844, light.

Check each screenshot against this list and fix any failure:

- [ ] No horizontal page scroll (only `.sp-heatmap-scroll` may scroll sideways).
- [ ] No text smaller than 12px except heatmap day labels.
- [ ] No more than one brand-red element per view.
- [ ] No card inside a bordered box.
- [ ] No paragraph longer than 12 words in a default (non-disclosed) area.
- [ ] Every number uses tabular figures.
- [ ] Every limit row shows "% left", a reset line, and has a tooltip with trust and seen time.
- [ ] Heatmap shows today on the right edge on phone width.
- [ ] Rail expansion pushes content, never covers it.
- [ ] Reduced motion (emulate `prefers-reduced-motion: reduce`): no meter animation.
- [ ] No emojis.
- [ ] Keyboard: Tab reaches every control in a sensible order; focus ring visible.

Also run one quick check with the browser accessibility tree (`read_page`) on Home: the meters expose `role="meter"` with values, the heatmap has its `aria-label`.

---

## 11. Pitfalls (read before each phase)

1. **The stylesheet is shared with the web console.** Any unscoped selector you add can change `web/app/console`. Prefix local-only rules with `.sp-native-console`. New `sp-` components are intentionally shared.
2. **Presenter exports are shared with `Client.tsx` and Remote.** Before renaming or deleting any export from `console-presentation.js`, run `grep -rn "<name>" web crates` and keep it if anything imports it.
3. **The daemon embeds files at compile time** (`include_str!`). After editing `console.html`, `console-app.js`, `gnomcode-console.css`, `gnomcode-tokens.css` or `console-presentation.js`, rebuild the daemon before checking the local console.
4. **`check-console-presentation.mjs`** runs before `npm test`. Read it before Phase 3; it may enforce rules about the presenter file.
5. **Do not compute state in JS.** Reviewers will grep for `< 20`, `<= 0`, `Math.sqrt`, `quantile`, `streak` in JS and reject.
6. **Never show estimates as exact.** Every dollar figure says "est." unless its basis is exact. Unknown stays "Unknown", never 0.
7. **Never call `Utc::now()` inside core.** Pass `now` in.
8. **Polling.** Do not add new 2-second fetches. The heatmap refreshes at most every 60 s and the today report every 10 s.
9. **Do not touch `remote.rs`, pairing, crypto, PTY or chat rendering.**
10. **Windows line endings.** The repo is LF. Make sure your editor does not convert files to CRLF (`git diff --stat` should not show whole-file changes).

---

## 12. Documentation changes per PR

### 12.1 Phase 2, SPEC 3.9

Append to the 3.9 paragraph:

> Rust derives a display state per window (`ok`, `low` below 20% left, `exhausted`, `reset_awaiting`, `unknown`) and, for exact or API readings after the first 5% of a window, a pace: the remaining percentage expected under even use, the difference, a verdict (`on_pace` within 5 points, `ahead`, `behind`) and a run-out time when it falls before the reset. Interfaces render these fields and never recompute them.

### 12.2 Phase 3, SPEC 3.9 and 3.16

3.9, append:

> Limits render as one card per provider (Claude Code, Codex and Antigravity always listed) with a meter per window showing percent left, reset countdown, pace marker and seen time; a provider without readings shows its enable command instead of a blank.

### 12.3 Phase 4, SPEC 3.16

Replace the last sentence of the 3.16 "Workspace is chat-first..." paragraph group's reporting rule with (keep the existing all-time grouping sentence):

> Token activity renders as a trailing-year grid of UTC days (53 ISO weeks, Monday first) with five Rust-computed intensity levels (square-root scaled to the busiest day), plus active days, current and longest streak and busiest day. Days before the first recording are not drawn; days without recordings read "no recorded activity", never zero tokens. The Today period uses a 24-hour strip in the same style.

### 12.4 Phase 5, SPEC 3.16

Replace "Dashboard groups usage/reporting" with:

> Home shows provider limit cards, sessions grouped as Needs you, Running and Recent (Rust-derived group), today's tokens, estimated spend and active sessions, and a 12-week activity grid.

### 12.5 Phase 6, SPEC 3.16

Replace "Desktop fine-pointer rail expands on hover/focus; touch uses an explicit toggle." with:

> The console has four places: Home, Workspace, Usage (limits, activity, spend, projects, exports) and Settings (providers, appearance, devices, automations, pricing, data, account, guide). The desktop rail is a grid column toggled only by its button and persisted locally; it never overlays content. Old Limits, Projects and Automations links redirect.

Also in each PR: SPEC section 5 roadmap status if there is a matching row; `docs/sessions/YYYY-MM-DD-<topic>.md` with goals, outcomes, problems, root-cause fixes, decisions, verification; a row in `docs/sessions/README.md`.

---

## 13. Final acceptance checklist (all phases merged)

- [ ] Rail has exactly Home, Workspace, Usage, Settings.
- [ ] `#limits`, `#projects`, `#automations` URLs land on the right new place.
- [ ] Home shows provider cards, grouped sessions, three tiles and a 12-week grid, in that order.
- [ ] Provider cards: "% left", reset countdown, pace marker and pace text when available, seen time, cost footer on Usage, enable command when empty.
- [ ] Usage shows the 53-week grid with stats and legend; Today shows the 24-hour strip; clicking a day opens its details.
- [ ] No bar chart remains anywhere in the local console.
- [ ] Workspace opens on the conversation list; launcher is behind "New".
- [ ] All Rust tests listed in section 8 exist and pass.
- [ ] All JS tests listed in phases 3-6 exist and pass.
- [ ] No JS file contains quota thresholds, level math or streak math.
- [ ] SPEC 3.9 and 3.16 updated; ADR added; DESIGN.md updated; session records added.
- [ ] Visual check 10.3 passes in all 4 modes for all views.

---

## 14. Later (not part of this plan)

- Per-agent filter chips on the activity grid (needs a store query per agent).
- Highlighting the selected period on the grid.
- Heatmap in the web console and Remote (needs the sync payload to carry daily levels).
- Ctrl+K command palette.
- Tray popover with the compact provider cards.
- Local-time day boundaries for the grid (needs a timezone offset parameter end to end).

---

## 15. Addendum (2026-11-12): collapsible cards, tray, mobile, web console and PWA

Visual reference for everything in this plan: [`console-mockup.html`](console-mockup.html). Open it in a browser and match it. Where the mockup and sections 5-7 differ, the mockup and this addendum win.

### 15.1 Collapsible provider cards (replaces the outer markup of 5.5)

- Every provider card is a native `<details class="sp-card sp-provider-card">`, **collapsed by default** on Home, Usage, the tray, the web console and Remote. No JS is needed to toggle.
- The `<summary>` is the one-line overview: provider mark, name, one line of the tightest window (`Weekly 98% left · resets Thu`, `Weekly exhausted · back Mon 1:20 PM`), a strip of tiny bars (one per window, colored by state), the state chip and a chevron. Rust supplies which window is tightest: add `headline_window` (index into `windows`) to `ProviderOverview` in 8.4, chosen as the window with the highest `severity`, ties broken by lowest `remaining_pct`.
- The body (`.sp-provider-body`) holds the meter rows from 5.4. Multi-pool providers (more than one distinct `pool`) group windows under a pool heading with compact one-line meters (`.sp-pool`, `.sp-mini` in the mockup).
- Remember open/closed per provider in `localStorage` key `gnomcode-provider-open` (JSON object of slug to boolean, try/catch). Default closed.
- On cards narrower than 400px (container query) hide the strip so the summary text fits.

### 15.2 Tray right-click menu (new Phase 8, branch `feat/tray-menu`)

"Tray" means the native menu shown when the user right-clicks the system tray icon (Windows notification area; also the menu bar icon on macOS and the indicator menu on Linux). It is a native `Menu`, not an HTML window, so it shows text only: no bars and no custom colors.

- `apps/desktop/src/main.ts`: replace the static `tray.setContextMenu(...)` with `buildTrayMenu(overview, sessions)`. Call it once at startup and again whenever the data changes. Poll `GET /api/v1/providers/overview` and the sessions list every 30 s from the main process using the existing `daemon-client.ts`, and rebuild only when the rendered labels changed. Rebuilding the menu while it is open is unsafe on Windows, so skip the rebuild if the menu is open (track `menu-will-show` / `menu-will-close`).
- Menu layout, top to bottom:
  1. `Needs you (1)` header (disabled) followed by up to 3 session items, `Fix quota parser · Claude Code`. Clicking one focuses the session (existing focus endpoint) or opens it in the main window. Omit the section when empty.
  2. `Running (2)`, same pattern. Omit when empty.
  3. Separator.
  4. One item per provider, collapsed into a submenu: label `Claude Code    Weekly 98% left · Thu`, `Codex    Weekly out · back Mon 1:20 PM`, `Antigravity    1 pool out`. The text after the name is the Rust `headline_window` (15.1), formatted exactly like the card summary. The submenu lists every window as a disabled item: `Session: reset, awaiting reading`, `Weekly: 98% left, resets Thu 8:00 AM`. Multi-pool providers group items under disabled pool headers with a separator between pools. The last submenu item is `Open limits` (opens Usage in the main window). When a provider has no data, the label is `Claude Code    Limits off` and the submenu has a single `Copy enable command` item that copies `enable_command` to the clipboard.
  5. Separator. `Updated 1m ago` (disabled).
  6. `New session`, `Open Gnomcode`, separator, `Quit Gnomcode` (existing behavior).
- State without color: prefix nothing; write the state in words (`out`, `low`, `reset`). No emojis or pictographs. Plain ASCII and the middle dot only.
- Tooltip (`tray.setToolTip`) adds the tightest limit: `Gnomcode · Claude weekly 98% · Codex out`, max 127 characters (Windows limit).
- Labels are built in TypeScript from Rust fields only (formatting, no thresholds), so add a pure `formatTrayMenu(overview, sessions, now)` in `apps/desktop/src/` returning a template array, and unit-test it in `apps/desktop/test` (headline text, empty provider, multi-pool grouping, tooltip truncation, no non-ASCII except the middle dot).
- The left-click behavior stays as today (open the main window).
- Fail open: if the daemon is unreachable, the menu shows `Daemon unavailable` (disabled) plus Open/Quit, as today.

### 15.3 Mobile

- Under 720px the rail becomes a bottom-safe top bar with 4 icons (mockup), cards stack in one column collapsed, and tiles become a two-column grid with the third tile spanning the full width.
- All tap targets stay at least 44px, summary rows at least 64px. No horizontal page scroll; only the heatmap scrolls inside its own box.

### 15.4 Web console and PWA (new Phase 9, branch `feat/web-console-v2`)

- `web/app/console/Client.tsx` and the Remote/PWA surfaces (`web/components/RemoteCompanion.tsx`, `web/app/remote`) must use the same presenters and CSS, so the account console and the installed PWA match the local console.
- Synced and Remote payloads must carry the Rust fields the UI needs (`state`, `pace`, `label`, `pool`, `window_label`, `stale`). Add them where the desktop publishes quota (`crates/daemon/src/sync.rs` and the encrypted Remote payload in `remote.rs`), all optional, so older desktops still render as `unknown` without colors. Do not add chat content to sync (invariant 1).
- Replace `QuotaPanel`, `SummaryPanel`, `OngoingPanel` and `ActivityPanel` in `Client.tsx` with the new provider cards, grouped session rows and tiles. The heatmap stays local-only (section 14).
- PWA polish: `manifest` `theme_color` and `background_color` set to `--sp-bg` for dark (`#070708`), `viewport-fit=cover` with `env(safe-area-inset-*)` padding, a standalone display mode, an app-like top bar, and no 300ms tap delay (`touch-action: manipulation`). Verify at 390x844 in both themes, installed (standalone) and in the browser.
- The Remote conversation header uses the compact chips from 5.5/Phase 3 step 4.
