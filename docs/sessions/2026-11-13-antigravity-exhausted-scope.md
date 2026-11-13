# 2026-11-13 - Antigravity RESOURCE_EXHAUSTED scoped to the failing pool

## Goals
- Follow up #218: the cross-check marked every non-100% Antigravity pool 100% used and exact after any `RESOURCE_EXHAUSTED` error.

## Outcomes
- The adapter no longer writes a quota row for the error (it cannot know the pool). The `rate_limited` activity event carries the model that was active (`tool_name`).
- `gnomcode_core::quota::model_matches_pool_name`: pure ordered whole-word match between a model name and a pool member.
- Store table `quota_pool_member` (created idempotently, no schema version bump). The daemon limits poll saves the model names each `RetrieveUserQuotaSummary` group lists; an empty read keeps the old list.
- `get_latest_quota_snapshots` marks only the single matching pool exact. No model, no match or several matches mark nothing exact and add an `estimated` hint row (five hours, never newer than the provider's newest reading so other pools are not superseded).
- Fixtures: `fixtures/antigravity/cli-1.5.8/quota-summary.json` (real reply shape, values rounded), `synthetic/resource-exhausted-model.jsonl`, `synthetic/resource-exhausted-no-model.jsonl`.

## Problems and decisions
- Live `RetrieveUserQuotaSummary` has no per-model ids, only a description (`Models within this group: ...`); that list is the mapping source. The `Gemini 4 Argon` group lists none, so only its name maps.
- The error does not name a window, so all windows of the matched pool are marked. This is documented in SPEC section 3.9 as a known limit.
- Events recorded by #218 have no model; they produce only an estimated hint.

## Verification
- `CARGO_INCREMENTAL=0 cargo fmt --all`, `clippy --workspace --all-targets -- -D warnings`, `test --workspace`.
