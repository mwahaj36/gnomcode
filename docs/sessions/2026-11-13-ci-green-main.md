# 2026-11-13 CI green on main

## Goals
Fix CI jobs that fail on main independent of recent PRs: MSRV, npm shim branding check, desktop lint.

## Outcomes
- MSRV: `gnomcode-daemon` uses let-chains (Rust 1.88). Bumped `rust-version` and the MSRV job to 1.88; `cargo +1.88 check --workspace --all-targets` passes.
- npm shim: `apps/desktop/src/branding.ts` was stale against `web/branding.json`; regenerated with `node scripts/sync-branding.mjs`.
- Desktop lint: empty `catch (_) {}` blocks in `main.ts` became `catch { /* reason */ }`; the intentional control-character regex in `security.ts` has a targeted `eslint-disable-next-line` with reason.

## Verification
`cargo fmt`, `cargo test --workspace`, desktop `npm ci`, lint, test, typecheck and build passed locally. A local clippy 1.99 reports extra `collapsible_if` lints that the CI stable toolchain does not; left out of scope.
