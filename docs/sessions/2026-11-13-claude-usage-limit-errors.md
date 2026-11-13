# 2026-11-13 - Claude usage-limit and API error parsing

## Goals
- Implement G8: parse Claude Code usage-limit and API error records from transcripts into `SessionState::Error`, `ActivityKind::RateLimited`/`Error`, and exact rejected `QuotaSnapshot` observations.

## Outcomes
- Extracted and anonymized real Claude Code 429 usage-limit transcript record into `fixtures/claude-code/2026-11-13/usage-limit.jsonl` using `anonymize-fixture.mjs`.
- Updated `crates/adapters/src/claude.rs` to handle `isApiErrorMessage`, `rate_limit_event`, and `result.is_error` records, avoiding `<synthetic>` model pollution.
- Added adapter tests in `crates/adapters/src/lib.rs` and ticked G8 in `docs/ROADMAP_1.0.md`.

## Verification
- `CARGO_INCREMENTAL=0 cargo fmt --all`
- `CARGO_INCREMENTAL=0 cargo clippy --workspace --all-targets -- -D warnings`
- `CARGO_INCREMENTAL=0 cargo test --workspace`
