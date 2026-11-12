# Session: First-Run Tour Refresh (PR 4 / A2)

- Date: 2026-11-13
- Branch: `feat/tour-refresh`
- Spec & Roadmap: [`docs/SPEC.md`](../SPEC.md) (§3.16), [`docs/ROADMAP_1.0.md`](../ROADMAP_1.0.md)

## Goals

- Rewrite the four-step first-run guide in `crates/daemon/src/console.html` and `crates/daemon/src/console-app.js` for the current console architecture:
  1. Four places (`Home`, `Workspace`, `Usage`, `Settings`).
  2. List-first `Workspace` with `New` for opening the session launcher.
  3. Provider limit cards and enabling `Claude` and `Antigravity` live limits under Settings > Providers.
  4. Tray right-click overview and Remote pairing under Settings > Devices and Remote.
- Enforce zero-session gating for auto-opening the modal (`#onboarding-dialog`) and show a non-modal dismissible tip (`#tour-tip`) when sessions already exist.
- Preserve skippable, keyboard-accessible dialog behavior, replayability from Settings (`Getting started`), zero analytics, and strict copy limits (labels <= 6 words, sentences <= 12 words, no emojis).

## Changes

- `crates/daemon/src/console.html`: Added `#tour-tip` reusing `.app-notice` + `.dialog-actions` styling and rewrote all four `#onboarding-dialog` steps cleanly within word limits.
- `crates/daemon/src/console-app.js`: Added `dismissTourTip()`, `checkOnboardingOffer()`, and session-count gating in `offerOnboarding(sessionCount)` (`0` opens `#onboarding-dialog`; `> 0` displays `#tour-tip` until dismissed or opened).
- `scripts/check-onboarding.mjs`: Added automated verification of zero-session gating, non-zero tip display and dismissal, replayability, keyboard/Escape skip, blocked storage, absence of emojis/analytics, topic coverage, and label (<= 6 words) + sentence (<= 12 words) limits.
- `docs/SPEC.md` & `docs/ROADMAP_1.0.md`: Updated §3.16 and ticked roadmap item.

## Verification

- `node scripts/check-onboarding.mjs`
- `node scripts/check-console.mjs`
- Full Rust gate (`cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `CARGO_INCREMENTAL=0 cargo test --workspace`).
