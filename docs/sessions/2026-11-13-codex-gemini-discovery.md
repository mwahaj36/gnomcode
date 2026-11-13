# 2026-11-13 - Codex and Gemini native-install discovery (G13)

## Goals
- Discover `codex` and `gemini` executables in standard non-PATH install directories across Windows, macOS, and Linux, matching the resolver used for launches and `gnomcode doctor`.

## Outcomes
- Added `standard_cli(name)` in `crates/platform/src/executables.rs` checking `~/.local/bin`, `~/.npm-global/bin`, Windows `%APPDATA%\npm` (with npm `.cmd` -> `node.exe` entry-point resolution), `%LOCALAPPDATA%\Microsoft\WinGet\Links`, `%LOCALAPPDATA%\Programs\<agent>`, `%USERPROFILE%\scoop\shims`, macOS `/opt/homebrew/bin` and `/usr/local/bin`, and Linux `/usr/local/bin`.
- Wired `standard_cli` into `executable_available` and `agent_command` (and therefore `resolve_agent_executable` and `background_agent_command`).
- Updated `docs/SPEC.md` §3.8 and ticked the Batch row in `docs/ROADMAP_1.0.md`.

## Verification
- Red-green:
  - `executables::tests::codex_standard_install_works_without_path_entry`: failed before fix, passed after fix using an isolated temp root and empty `PATH`.
  - `executables::tests::gemini_standard_install_works_without_path_entry`: failed before fix, passed after fix using an isolated temp root and empty `PATH`.
- Full Rust gate (`cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`) with `CARGO_INCREMENTAL=0`.
