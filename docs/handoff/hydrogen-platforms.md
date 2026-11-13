# Hydrogen Platforms handoff

## Completed items

1. **CI matrix for macOS/Linux desktop packaging** — PR #254 (`ci/macos-linux-packaging`): https://github.com/mwahaj36/spawnt/pull/254
   - What changed: appended `package-desktop` matrix job to `.github/workflows/ci.yml` (`ubuntu-22.04` x64 `.deb`/`.zip`, `macos-15-intel` x64 `.zip`, `macos-15` arm64 `.zip`, gated on `inputs.package`), updated `docs/SPEC.md` §6.2, added `docs/sessions/2026-11-13-ci-macos-linux-packaging.md`.
   - Gate: `CARGO_INCREMENTAL=0` `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` passed locally.
   - Verified live: CI run https://github.com/mwahaj36/spawnt/actions/runs/43464307883 (`desktop package` passed on all 3 targets; `npm shim (macos-latest)` was cancelled by GitHub Actions).
   - Not verified: runtime smoke of packaged macOS/Linux builds (Item 3). No artifacts published.
   - Risks: none.

2. **macOS/Linux correctness & CLI/data-path audit** — PR #256 (`fix/macos-linux-correctness`): https://github.com/mwahaj36/spawnt/pull/256
   - What changed: `crates/platform/src/executables.rs` (`claude_cli()` fallback to `standard_cli("claude")`, `antigravity_cli()` Unix checks for `~/.local/bin/agy`, `~/.gemini/bin/agy`, `standard_cli("agy")`, `antigravity_ide_available()` macOS `Antigravity IDE.app`), `crates/platform/src/keepawake.rs` (`unix_helper_command_uses_expected_backend` test), `docs/SPEC.md` §3.5 (marked macOS/Linux Antigravity IDE/2.0 data paths unverified on-device), `docs/sessions/2026-11-13-macos-linux-correctness.md`.
   - Gate: `CARGO_INCREMENTAL=0` `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` passed on Windows.
   - Verified live: Windows workspace tests passed.
   - Not verified: macOS/Linux CI run https://github.com/mwahaj36/spawnt/actions/runs/43475756520 was blocked from starting by GitHub Actions billing/spending limit (`The job was not started because recent account payments have failed or your spending limit needs to be increased`).
   - Risks: macOS/Linux `#[cfg(unix)]` tests (`agy_gemini_bin_install_works_without_path_entry`, `unix_helper_command_uses_expected_backend`) have not yet run on macOS/Linux runners due to the GitHub Actions billing block.

3. **Automated packaged-build smoke script** — PR #257 (`feat/packaged-smoke-script`): https://github.com/mwahaj36/spawnt/pull/257
   - What changed: added `scripts/smoke/run-packaged-smoke.mjs` (starts daemon with temp `GNOMCODE_HOME` on free port, checks `/api/v1/doctor`, `/api/v1/providers/overview`, `/api/v1/sessions`, console HTML `/`, and `formatTrayMenu` template), appended `package-smoke` job to `.github/workflows/ci.yml`, updated `docs/SPEC.md` §6.2, added `docs/sessions/2026-11-13-packaged-smoke-script.md`.
   - Gate: `CARGO_INCREMENTAL=0` `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `apps/desktop` `lint`/`typecheck`/`test` (40 passed) all passed.
   - Verified live: `node scripts/smoke/run-packaged-smoke.mjs` passed locally on Windows against built `gnomcode.exe` and `apps/desktop/dist/tray-menu.js`.
   - Not verified: CI `package-smoke` on macOS/Linux (blocked by GitHub Actions billing/spending limit).
   - Risks: none.

4. **Platform support doc (`docs/PLATFORMS.md`)** — PR #258 (`docs/platforms-support-matrix`): https://github.com/mwahaj36/spawnt/pull/258
   - What changed: added `docs/PLATFORMS.md` (exact verified status and known gaps per OS/arch for 1.0), referenced it in `docs/SPEC.md` §6.2, added `docs/sessions/2026-11-13-platforms-support-doc.md`.
   - Gate: encoding check (`'Â'` count = 0).
   - Verified live: N/A (documentation).
   - Not verified: live macOS/Linux hardware acceptance remains open and is documented in `docs/PLATFORMS.md`.
   - Risks: none.

## Status at stop

- **All 4 items completed and PRs opened.**
- **Blocking question / action for Wahaj**:
  - GitHub Actions on `mwahaj36/spawnt` hit a billing/spending limit starting at run `43475756520` (`The job was not started because recent account payments have failed or your spending limit needs to be increased`). Once billing/spending limit is restored, re-dispatch `ci.yml` with `package=true` on `main` after merging PRs #254, #256, #257, #258 to verify the Unix-only unit tests in PR #256 and `package-smoke` in PR #257 on macOS and Linux runners.
- **Resume instructions**:
  1. Restore GitHub Actions billing/spending limit.
  2. Merge PRs in order below.
  3. Run `gh workflow run ci.yml --ref main -f package=true` and `gh run watch`.
- **PRs in merge order** (all `OPEN`):
  1. #254 (`ci/macos-linux-packaging`) — https://github.com/mwahaj36/spawnt/pull/254
  2. #256 (`fix/macos-linux-correctness`) — https://github.com/mwahaj36/spawnt/pull/256
  3. #257 (`feat/packaged-smoke-script`) — https://github.com/mwahaj36/spawnt/pull/257
  4. #258 (`docs/platforms-support-matrix`) — https://github.com/mwahaj36/spawnt/pull/258
- **Handoff log PR (do not merge)**:
  - #255 (`docs/handoff-hydrogen-platforms`) — https://github.com/mwahaj36/spawnt/pull/255
