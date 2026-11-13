# 2026-11-13 - D3 ingest file watcher and idle back-off

## Goals
- Implement D3: `notify`-backed file watcher with exponential idle back-off in `crates/ingest/src/watcher.rs` and measure idle CPU (< 1%).

## Outcomes
- Added `crates/ingest/src/watcher.rs` (`IdleBackoff`, `IngestWatcher`) and exposed `pub mod watcher;` in `crates/ingest/src/lib.rs`.
- Ticked D3 in `docs/ROADMAP_1.0.md`.

## Verification
- `CARGO_INCREMENTAL=0 cargo fmt --all`
- `CARGO_INCREMENTAL=0 cargo clippy --workspace --all-targets -- -D warnings`
- `CARGO_INCREMENTAL=0 cargo test --workspace`
- Measured idle CPU of `IngestWatcher` vs tight polling.
