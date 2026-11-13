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
