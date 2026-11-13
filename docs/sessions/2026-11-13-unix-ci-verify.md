# 2026-11-13 - Unix CI verification (#202)

## Goals
- The Unix process-group stop (#202) had never compiled or run in CI. CI is manual-dispatch, so verify it on Ubuntu and macOS.

## Outcomes
- Dispatched the CI workflow on `main` (run 43384446238). `clippy + test` passed on macOS. On Ubuntu clippy passed (the cfg-gated Unix code compiles) but `platform` test `unix_stop_escalates_past_ignored_sigint` failed with `process termination failed`.
- Root cause: `process_is_alive` used `sysinfo` presence only. After SIGKILL the child stays a zombie until its parent waits, so `stop_process_group` kept seeing it as alive and reported failure although it had been killed. Reproduced locally under WSL (Ubuntu 22.04).
- Fix: `process_is_alive` returns false for a zombie (`ProcessStatus::Zombie`). SPEC section on stopping owned processes states it.

## Not in scope, reported
- MSRV job fails: let-chains in `gnomcode-daemon` need Rust 1.88 while `rust-version` is 1.87.
- `npm shim` (Ubuntu, macOS): `node scripts/sync-branding.mjs --check` reports stale `apps/desktop/src/branding.ts`.
- `desktop` job: `npm run lint` errors (`no-empty`, `no-unused-vars`, `no-control-regex`).

## Verification
- `cargo test -p gnomcode-platform` under WSL before (1 failure) and after (all pass), then CI re-dispatched on the branch; run URLs are in the PR.
