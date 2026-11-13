# 2026-11-13 - Split daemon api.rs by domain (roadmap E2, part 1)

## Goals
- Break the 5,470-line `crates/daemon/src/api.rs` into per-domain modules under `crates/daemon/src/api/` with pure moves: no behavior or signature changes, tests move with their code.

## Module map
- `api.rs` (root): `AppState`, query/health types, `build_router`, console asset handlers (kept here because their `include_str!` paths are relative to this file), `validate_auth`, `cors_headers`, `health_handler`, and the six root-level auth/host tests.
- `api/sessions.rs`: focus, launch, workspace terminal, stop, hosted list, prompt, PTY stream and proxy, session list, live snapshot.
- `api/handoff.rs`: model switch, continue, legacy resume.
- `api/models.rs`: model catalog, installed agents, routing policy.
- `api/conversation.rs`: conversation list and chat history.
- `api/approvals.rs`: tool permission decisions.
- `api/files.rs`: repository file and diff, `git_output`.
- `api/projects.rs`: projects and workspace browser.
- `api/quota.rs`, `api/providers.rs`: quota pool views and provider overviews.
- `api/report.rs`: report, heatmap, stats, export, ROI.
- `api/settings.rs`: license, profile, notification settings.
- `api/automation.rs`: guardrail rules, alerts, resume queue, relay, automation.
- `api/remote.rs`, `api/doctor.rs`, `api/update.rs`, `api/sync.rs`, `api/pricing.rs`, `api/account.rs`.
- Worktree, evals, attachments, artifacts and changes routes already live in their own `crates/daemon/src/*.rs` files and are untouched.

## Mechanics
- Each child starts with `use super::*;`; the root re-exports each child with a glob (`pub use`, `pub(crate) use` or `use`, matching the strongest visibility the child exports) so `crate::api::...` paths keep working.
- Items that were private became `pub(super)` (visible only inside `api`), because the router in the root now reaches handlers in children. `pub` and `pub(crate)` items are unchanged. `QuotaPoolView` fields became `pub(super)` for the same reason.
- `pool()` test fixture is shared by quota and provider tests, so it is a `#[cfg(test)]` module-level item in `quota.rs` instead of inside its `tests` module.
- No SQL, `obfstr!` call or `include_str!` was touched.

## Verification
- Names: a script compares every `fn/struct/enum/const/static/type` name before and after (200 = 200, none missing or added); a line-multiset diff shows only `pub(super)` prefixes, module headers and test imports differ.
- Test attributes 53 before and after; workspace `cargo test` 499 passed before and after.
- Gate with `CARGO_INCREMENTAL=0`: fmt, clippy `-D warnings`, test `--workspace`.
