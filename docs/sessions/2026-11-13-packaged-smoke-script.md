# Automated packaged-build smoke script (2026-11-13)

## Goal

Provide a cross-platform smoke test (`scripts/smoke/run-packaged-smoke.mjs`) for packaged `gnomcode` builds and wire it into CI packaging.

## Changes

- `scripts/smoke/run-packaged-smoke.mjs`: starts the packaged/staged daemon binary with a temporary `GNOMCODE_HOME` on a free loopback port, verifies `/api/v1/doctor`, `/api/v1/providers/overview` (3 providers: `antigravity`, `claude-code`, `codex`), `/api/v1/sessions`, console HTML at `/`, and `formatTrayMenu` from `apps/desktop/dist/tray-menu.js` when built; cleans up the daemon and temp directory and exits non-zero on failure.
- `.github/workflows/ci.yml`: appended `package-smoke` job (`ubuntu-22.04`, `macos-15-intel`, `macos-15`) gated on `inputs.package` (`needs: [package]`).
- `docs/SPEC.md`: documented `scripts/smoke/run-packaged-smoke.mjs` in §6.2.

## Verification

- Local Windows run against built `gnomcode.exe` + `apps/desktop/dist/tray-menu.js`: passed (`/api/v1/doctor` 7 checks, `/api/v1/providers/overview`, `/api/v1/sessions`, `GET /`, `formatTrayMenu` 8 items).
- `apps/desktop` gate (`npm run lint`, `npm run typecheck`, `npm test`), `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.
- CI `package-smoke` not dispatched due to the GitHub Actions account billing/spending limit observed in PR #256.
