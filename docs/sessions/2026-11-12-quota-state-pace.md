# 2026-11-12 - Quota state, pace and provider overview API

## Goals
- Phase 2 of the console overhaul plan: derive quota display state and pace in Rust, expose them on `/api/v1/quota`, and add `GET /api/v1/providers/overview` (first step of Phase 3) so interfaces render fields instead of computing them.

## Outcomes
- `crates/core/src/quota.rs`: `QuotaState`, `PaceVerdict`, `QuotaPace`, `window_length`, `remaining_pct`, `quota_state`, `quota_pace`, `is_stale`, `severity`. Pure; `now` is always a parameter. 14 unit tests with a fixed clock.
- `quota_views` now uses one `now` and the core functions. Each window gains `state`, `pace`, `stale`, `label`, `pool` and `window_label`; no existing field was renamed. `conversation_quota` gains `state` and keeps its existing `label`.
- `GET /api/v1/providers/overview`: one card per provider (Claude Code, Codex, Antigravity always listed), worst-window state, sorted windows, source label, enable hint and command when empty, and estimated recorded spend.
- SPEC 3.9 updated with the state and pace rules and the overview endpoint.

## Decisions
- Pool names that are snake or kebab ids (`gemini_pro`) are title-cased; names that already contain other characters (`Claude and GPT models`) are kept verbatim. The pool name is also exposed alone as `pool`, and the window as `window_label`, so the UI can group windows under a pool heading.
- Overview cost is keyed on `ModelReport.agent` (the agent slug), not `ProviderReport.provider`.
- The overview builder takes the vendor-limit flags as booleans, so tests never read or write the real Gnomcode home.
- `conversation_quota` keeps its short labels (`5h`, `Weekly`) because chat and Remote already consume them.

## Problems and root causes
- The plan assumed `ProviderReport.provider` equals the quota provider slug. It is a display label ("Claude Code", "OpenAI Codex", "Google Gemini CLI"). Fix: sum `ModelReport.agent` slugs, covered by `overview_costs_key_on_agent_slug_not_display_label`.
- Plan test 12 (`pace_ahead_without_run_out_when_reset_comes_first`) cannot be constructed: under linear projection a window that is ahead of pace always runs out before its reset (remaining < (1 - elapsed) * 100 implies remaining * elapsed < (1 - elapsed) * used). The `eta < resets_at` filter stays as a defensive guard; the test instead asserts the invariant at 90% elapsed and 96% used (`pace_ahead_late_in_window_still_runs_out_before_reset`).
- `web/lib/console-presentation.d.ts` has no typed quota pool (parameters are `unknown`), so no type change was made.

## Verification
- `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`: see the PR description for the final summary lines.

## Follow-up fix
- `sync::tests::remote_snapshot_contains_no_local_project_path_or_pairing_token` pins the key set of synced quota rows. The new derived fields (`state`, `pace`, `stale`, `label`, `pool`, `window_label`) are metadata only, so the expected keys were extended; the Remote and web console now receive state without recomputing it.

## Rebase and headline window
- Rebased onto main after the activity heatmap API merged; kept both routes and both session entries.
- Added `headline_window` to the provider overview (plan section 15.1): index of the window with the highest severity, ties by lowest remaining percentage, `None` without windows. Test: `overview_headline_window_is_tightest_by_severity_then_remaining`.
