# 2026-11-13 - Read-only workspace file list and read over Remote (ADR 0050)

## Goals
- Expose read-only workspace directory listing (`GET /api/v1/repos/files`, Remote `repo_files`) and file reading (Remote `repo_file`) confined to the session workspace.

## Outcomes
- Added `crates/daemon/src/workspace_files.rs` with path traversal rejection, canonical-root containment (blocking symlink/junction escapes), 200-entry directory cap, and 512 KiB UTF-8 file cap.
- Wired `GET /api/v1/repos/files` in `crates/daemon/src/api.rs`, `repo_files`/`repo_file` dispatch in `crates/daemon/src/remote.rs`, and `mod workspace_files` in `crates/daemon/src/lib.rs`.
- Added ADR 0050 (`docs/adr/0050-remote-workspace-files.md`) and updated `docs/SPEC.md`.

## Verification
- `workspace_files::tests::remote_repo_files_and_file_stay_confined_and_enforce_size_limit`
- Full Rust gate (`cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`) with `CARGO_INCREMENTAL=0`.
