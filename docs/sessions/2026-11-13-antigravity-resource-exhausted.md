# 2026-11-13 - Antigravity RESOURCE_EXHAUSTED cross-check

## Goals
- Cross-check Antigravity quota snapshots against observed `RESOURCE_EXHAUSTED` transcript errors and prefer the error when they disagree.

## Outcomes
- Updated `crates/adapters/src/antigravity.rs` to detect `RESOURCE_EXHAUSTED` error steps, set `SessionState::Error`, emit `ActivityKind::RateLimited`, and emit an exact exhausted `QuotaSnapshot`.
- Updated `crates/store/src/lib.rs` (`apply_antigravity_resource_exhausted_crosscheck`) so `get_latest_quota_snapshots` overrides stale/optimistic Antigravity API readings when a recent `rate_limited` observation disagrees within the unreset window.
- Ticked the roadmap item in `docs/ROADMAP_1.0.md` and updated `docs/SPEC.md`.

## Verification
- `CARGO_INCREMENTAL=0 cargo fmt --all`
- `CARGO_INCREMENTAL=0 cargo clippy --workspace --all-targets -- -D warnings`
- `CARGO_INCREMENTAL=0 cargo test --workspace`
