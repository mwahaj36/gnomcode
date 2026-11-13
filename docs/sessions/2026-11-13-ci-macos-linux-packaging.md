# CI packaging matrix for macOS and Linux desktop builds (2026-11-13)

## Goal

Add `workflow_dispatch` CI jobs to build the `gnomcode` daemon and Electron desktop app for macOS (`arm64` and `x64`) and Linux (`x64` `.deb` and `.zip`) without publishing artifacts.

## Changes

- `.github/workflows/ci.yml`: appended `package-desktop` matrix job (`ubuntu-22.04` `x64`, `macos-15-intel` `x64`, `macos-15` `arm64`) gated by `inputs.package`.
- `docs/SPEC.md`: documented the `package-desktop` CI matrix in §6.2.

## Problems and fixes

1. Electron 45.1.0 downloads `node_modules/electron/dist` lazily on `require('electron')`, so `node desktop-notices.cjs` failed with `ENOENT` on `LICENSE` until `node -e "require('electron')"` was added after `npm ci`.
2. `taiki-e/install-action@cargo-about` did not provide `cargo-about` on `macos-15-intel`, and `cargo install cargo-about --locked` builds without the `cli` binary unless `--features cli` is passed. Added `if ! cargo about --version >/dev/null 2>&1; then cargo install cargo-about --locked --features cli; fi`.

## Verification

- Dispatched `ci.yml` (`package=true`) on `ci/macos-linux-packaging`:
  - Run `43464307883` (`https://github.com/mwahaj36/spawnt/actions/runs/43464307883`) — all three `package-desktop` jobs (`ubuntu-22.04 / x64`, `macos-15-intel / x64`, `macos-15 / arm64`) passed.
  - Earlier failed runs during iteration: `43461306788`, `43464307883` (`npm shim (macos-latest)` was cancelled by GitHub Actions runner; all packaging jobs succeeded).
