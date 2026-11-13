# Handoff fold: hydrogen-platforms (PR #255)

Date: 2026-11-14. Source: running handoff log PR #255, closed unmerged. Facts below are the part worth keeping.

## Delivered (all merged)

- #254 CI `package-desktop` matrix (ubuntu-22.04 x64 deb/zip, macos-15-intel x64 zip, macos-15 arm64 zip), gated on `inputs.package`. Live CI run: desktop package passed on all 3 targets.
- #256 macOS/Linux correctness: `claude_cli()` fallback, `agy` Unix lookups, macOS Antigravity IDE check, keepawake helper test. SPEC §3.5 marks macOS/Linux Antigravity data paths unverified on-device.
- #257 packaged smoke script `scripts/smoke/run-packaged-smoke.mjs` plus CI `package-smoke` job. Passed locally on Windows against built binary.
- #258 `docs/PLATFORMS.md` support matrix.

## Decisions

- macOS/Linux stay "built in CI, not hardware-accepted" for 1.0; gaps live in `docs/PLATFORMS.md`.

## Evidence gaps (unfinished)

- Unix-only tests from #256 (`agy_gemini_bin_install_works_without_path_entry`, `unix_helper_command_uses_expected_backend`) and `package-smoke` from #257 never ran on macOS/Linux runners: Actions billing/spending limit blocked job start.
- Follow-up: after billing is restored, run `gh workflow run ci.yml --ref main -f package=true` and confirm those jobs. Tracked in `docs/ROADMAP_1.0.md` section G (real-device and macOS/Linux checks).
