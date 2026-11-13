# 2026-11-13 - Freeze unproven features behind off-by-default toggles (E3)

## Goals
- Freeze Teams, extra adapters (`GeminiCli`), unused scheduled messages, and cloud account sync behind off-by-default settings (`FeatureToggles`), preserving all existing data and code.

## Outcomes
- Added `gnomcode_core::FeatureToggles` (all fields `false` by default) and `Database::get_feature_toggles` / `set_feature_toggles` backed by `desktop_preferences`.
- Added `crates/daemon/src/features.rs` (`GET/POST /api/v1/settings/features` and Remote `features.get`/`features.set`), plus runtime gates in `crates/ingest/src/lib.rs`, `crates/daemon/src/automation.rs`, `crates/daemon/src/sync.rs`, and `crates/daemon/src/pro.rs`.
- Added ADR 0049 and ticked E3 in `docs/ROADMAP_1.0.md`.

## Verification
- `CARGO_INCREMENTAL=0 cargo fmt --all`
- `CARGO_INCREMENTAL=0 cargo clippy --workspace --all-targets -- -D warnings`
- `CARGO_INCREMENTAL=0 cargo test --workspace`
