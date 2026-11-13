# macOS and Linux platform correctness (2026-11-13)

## Goal

Verify keep-awake backends (`caffeinate`, `systemd-inhibit`), Unix process-group stop, tray behavior compilation/tests, native CLI discovery (`claude`, `codex`, `agy`), and Antigravity macOS/Linux data path status in CI.

## Changes

- `crates/platform/src/executables.rs`:
  - `claude_cli()` now falls back to `standard_cli("claude")` (`~/.npm-global/bin`, `/opt/homebrew/bin`, `/usr/local/bin`) on Unix and Windows when `~/.local/bin/claude` is absent.
  - `antigravity_cli()` on Unix now checks `~/.local/bin/agy`, `~/.gemini/bin/agy`, and `standard_cli("agy")`.
  - `antigravity_ide_available()` on macOS now checks both `Antigravity.app` and `Antigravity IDE.app` under `/Applications` and `~/Applications`.
  - Added isolated unit tests `claude_standard_install_works_without_path_entry` and `agy_gemini_bin_install_works_without_path_entry` (`#[cfg(unix)]`).
- `crates/platform/src/keepawake.rs`:
  - Added `#[cfg(not(target_os = "windows"))]` unit test `unix_helper_command_uses_expected_backend` verifying `caffeinate -i -w` (macOS) and `systemd-inhibit --what=idle:sleep --mode=block` (Linux).
- `docs/SPEC.md`:
  - Marked macOS (`~/Library/Application Support/Antigravity`) and Linux (`~/.config/Antigravity`, `~/.gemini`) Antigravity IDE/2.0 app data paths as unverified on-device in §3.5.

## Verification

- Local Rust gate (`CARGO_INCREMENTAL=0`): `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.
- Three-OS CI (`ubuntu-latest`, `macos-latest`, `windows-latest`) dispatched via `gh workflow run ci.yml --ref fix/macos-linux-correctness`.
