# 2026-11-13 - Split store lib.rs by domain (roadmap E2, part 2)

## Goals
- Break the 5,689-line `crates/store/src/lib.rs` into per-domain modules with pure moves: the single 3,000-line `impl Database` becomes one small `impl Database` block per domain, tests move with their code.

## Module map
- `lib.rs` (root, 122 lines): module list and re-exports, `Error`, `event_marker`, `mark_deleted`, the `Database` struct, `open`, `open_in_memory`.
- `schema.rs`: `migrate` (all schema SQL).
- `chat.rs`: chat events, format repair, retrieval, handoff preparation, deletion and retention.
- `rules.rs`: guardrail rules, alerts, resume queue (`RuleAlert`).
- `relay.rs`: context relay config and ledger.
- `settings.rs`: user profile, routing policy, auto-update and feature toggles.
- `pricing.rs`: custom model pricing and subscription auto-populate.
- `ingest.rs`: `SourceCheckpoint`, `commit_batch*`, `get_checkpoint`.
- `sessions.rs`: session listing, subagents, focus target, cwd.
- `report.rs`: usage report, timeline, daily rows, activity heatmap, database stats.
- `quota.rs`: quota snapshots, pool members, Antigravity exhaustion cross-check.
- `projects.rs`: project list, reassign, Antigravity project reconcile, project report.
- Existing `automation.rs`, `conversation.rs`, `trajectory.rs` are untouched.

## Mechanics
- Each child starts with `use super::*;`; the root re-exports every child that declares types with `pub use child::*`, so `gnomcode_store::X` paths are unchanged.
- Private methods and functions (`migrate`, `list_projects`, `generate_project_report`, `activity_heatmap`, `daily_usage_rows`, `attach_subagents`, `apply_antigravity_resource_exhausted_crosscheck`, `antigravity_workspace`, `group_activity`, `unknown_cost_basis`) became `pub(crate)`, because they now live in a different module than their callers. No `pub` item changed.
- `heatmap_db` test fixture is shared by report, project and session tests, so it is a `#[cfg(test)]` module-level item in `report.rs`.
- Every `obfstr!` call stays around its SQL string; no SQL text was edited.

## Verification
- Names: 153 `fn/struct/enum/const/static/type` declarations before and after, none missing or added; a line-multiset diff shows only `pub(crate)` prefixes, headers and test imports differ.
- Test attributes 41 before and after; workspace tests 499 before and after.
- Gate with `CARGO_INCREMENTAL=0`: fmt, clippy `-D warnings`, test `--workspace`.
