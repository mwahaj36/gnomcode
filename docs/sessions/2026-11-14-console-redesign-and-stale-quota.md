# 2026-11-14 — Console Redesign & Stale Quota Honesty

## Goals
1. Fix stale and past-reset quota reporting in Rust (`crates/core`, `crates/daemon`) and UI (local console, web/PWA, desktop tray) so past-reset windows report `reset_unconfirmed` (`0%` used, `EST`) and stale pace projections withhold `runs_out_at`.
2. Execute the full Gnomcode console redesign across desktop, web console, and mobile PWA in focused PRs while keeping every owner UI decision intact.

## Outcomes
- **Stale Quota Fix (PR #269 + PR #270):**
  - Updated `docs/SPEC.md` §3.9, `crates/core/src/quota.rs` (`QuotaState::ResetUnconfirmed`, `QuotaPace.stale` + `stale_reason`, `effective_basis`, `effective_used_pct`), and `crates/daemon/src/api/quota.rs`.
  - Updated `web/lib/console-presentation.js`, `HomeView.tsx`, `UsageView.tsx`, `apps/desktop/src/tray-menu.ts` with 30s live countdown ticks, immediate reset-boundary re-fetch, muted `Reset, not confirmed EST`, muted `Based on data from X` staleness note, and `Last seen X` near numbers.
- **Console Redesign Stack (PR #268 -> PR #271 -> PR #272 -> PR #273 -> PR #274 -> PR 6):**
  - PR #268: Canonical tokens (`gnomcode-tokens.css`), brand-red 53-week heatmap, themed 6px scrollbars, non-wrapping pills, design audit/spec/mockup/`owner-ui-decisions.md`.
  - PR #271: Unified `268px` collapsible left sidebar on desktop (`>= 768px`), bottom 4-tab bar on mobile (`< 768px`), no selection outline/stripe, zero horizontal page scroll.
  - PR #272: `72ch` transcript, collapsed subagents capped at 5 inline, dedicated `All chats` pane, IDE provider quota badges + Usage popover (`View Usage ›`), single-line padded mobile quick prompts.
  - PR #273: Full-width `Limits` -> `2fr/1fr` `Activity` (53-week grid with months & mobile right-first scroll) + `Sessions` (with token readout) left, `Today` right; multi-pool Antigravity `Low` vs `Out` card state in Rust + `SPEC.md`.
  - PR #274: Single-column grouped Settings & Account cards with `44px` mobile touch targets.
  - PR #275: Engineering session record only (no CSS changes landed in this PR).

## Decisions
- Scoped the multi-pool `Low` vs `Exhausted` card rule to providers with `> 1` distinct `pool` / `limit_id` so single-pool providers (Claude Code, Codex) still report `Exhausted` when their `Session` window is 100% used.
- Guarded reset-boundary immediate re-fetches with a 1s floor and 10s boundary deduplication window to avoid tight loops while awaiting fresh logs.

## Verification
- Rust: `cargo fmt --all -- --check`, `cargo clippy -p gnomcode-core -p gnomcode-daemon --all-targets -- -D warnings`, `cargo test -p gnomcode-core -p gnomcode-daemon`.
- Web: `node scripts/build-local-console.mjs`, `cd web && npm run typecheck && npm test && npm run lint`.
- Desktop: `cd apps/desktop && npm run build && npm test`.
- Playwright E2E: `cd web && npx playwright test e2e/shell.spec.ts e2e/views.spec.ts e2e/phone.spec.ts`.

## Release review notes
- #272 was auto-closed by GitHub when its stacked base branch was deleted after #271 merged; its branch was re-submitted as #279 and merged.
- Reviewer fixes on top of the authored PRs: nav tab keyboard focus ring restored (a stray `outline: none` outranked the focus rule), undefined `--sp-r-sm/md/lg` radius tokens defined or replaced, subagent summary wording matches the owner decision (`N running · M subagents`), `EST` label on All chats cost, Escape closes the Usage popover, and e2e guards added for the brand-red heatmap ramp, no horizontal page scroll at 390px, nav focus ring and 44px Settings controls.
- `docs/design/owner-ui-decisions.md` is the binding list; the e2e guards above keep the heatmap and layout decisions from regressing.
