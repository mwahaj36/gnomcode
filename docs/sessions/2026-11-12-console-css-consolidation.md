# 2026-11-12 - Console CSS consolidation (overhaul Phase 1)

## Goals
- Move the inline `<style>` block of `crates/daemon/src/console.html` into the shared `web/app/gnomcode-console.css`.
- Add the v2 tokens (layout, spacing, type scale, elevation, state colors, heat ramp, motion) to `web/app/gnomcode-tokens.css`.
- No visual change in the local console; no change in the web console.

## Outcomes
- 467 lines of inline CSS now live at the top of `gnomcode-console.css` under `/* ---- Local console (moved from console.html) ---- */`. `console.html` still links tokens, console and xterm stylesheets.
- New tokens added to `:root` and to both light blocks. No existing variable or `@font-face` line changed.

## Decisions and deviations from the plan
- **Scoping uses `:where(.sp-native-console)`, not a plain `.sp-native-console ` prefix.** A plain prefix adds one class of specificity (0,1,0) and would flip winners against existing rules in `gnomcode-console.css` that previously won at equal specificity. `:where()` adds none, so every moved rule keeps its original specificity. `body` became `body:where(.sp-native-console)`; `*` became `:where(.sp-native-console), :where(.sp-native-console) *`; the `:root` alias block (`--bg`, `--card-bg`, ...) now sits on `:where(.sp-native-console)` so the web app does not receive those aliases. ID selectors that exist only in `console.html` stay unscoped.
- **The moved block is placed at the START of `gnomcode-console.css`, not the end.** In `console.html` the inline style preceded the linked sheets, so equal-specificity rules in `gnomcode-console.css` won. Appending the block at the end would let the moved rules win those ties. Placing it first preserves the original cascade exactly. A comment in the file explains this.
- Dropped the dead `@import url("/gnomcode-tokens.css")` (it was mid-stylesheet and ignored by browsers anyway).
- `scripts/check-console-presentation.mjs` forbids marketing aliases `var(--text)`, `var(--border)`, `var(--bg)` in the shared stylesheet. The moved block used them, so those 24 uses were replaced with `var(--sp-text)`, `var(--sp-border)`, `var(--sp-bg)` (the aliases were 1:1 mappings, so no value changes). The other aliases (`--text-muted`, `--card-bg`, ...) are kept.

## Problems encountered
- Baseline is red on `main`: `scripts/check-console-presentation.mjs` (first step of `npm test`) asserts old quota markup ("Last received reading", "25.0% remaining", "50.0%") that `renderQuota` no longer produces since the compact-limits change. This is unrelated to CSS and fails identically without this branch's changes. It is not fixed here; Phase 3 rewrites `renderQuota` and its assertions. The node test files (`node --test test/*.test.mjs`) pass: 30/30.
- Direct before/after comparison needed care: pointer hover, in-flight transitions and live data made naive snapshots noisy.

## Verification
- `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`: pass.
- `npm run typecheck`, `npm run build` (web): pass. `node --test test/*.test.mjs`: 30 pass. `npm test`: fails at the pre-existing quota assertion described above (the CSS-alias assertion later in the same script passes with this change).
- Computed-style A/B in the built local console (temp `GNOMCODE_HOME`, port 41999): the old stylesheet (inline block first, then old console sheet) was rebuilt in-page from the new file and toggled against the new one in the same page. Every element in `html, body, body *` (about 1500) was compared across 7 views x dark/light x 1200px/390px. The only differences: `html { box-sizing }` (content-box now, border-box before; html has no padding or border so nothing renders differently) and the live `#sync-state` text. Screenshots were not compared.
- Web console `/console` was not opened (requires sign-in). Its safety rests on every moved rule being scoped to `.sp-native-console`, which the web app never sets, and on none of the unscoped ID selectors existing in `web/`.
