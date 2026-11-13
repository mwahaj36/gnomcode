# Platform support matrix for 1.0 (2026-11-13)

## Goal

Document exact per-OS verification status and known gaps for Gnomcode 1.0 in `docs/PLATFORMS.md` so marketing and release gates match observed evidence.

## Changes

- `docs/PLATFORMS.md`: added 1.0 platform support matrix covering Windows (`x64`, `arm64`), macOS (`arm64`, `x64`), and Linux (`x64`, `arm64`, `musl`), grounded in CI run `43464307883` and local Windows verification.
- `docs/SPEC.md`: referenced `docs/PLATFORMS.md` in §6.2.

## Verification

- Verified all cited run IDs (`43464307883`, `43475756520`) and PRs (#254, #256, #257) against session logs and CI outputs.
- Ran `cargo fmt --all --check` and `git diff origin/main` encoding check.
