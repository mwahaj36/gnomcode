# ADR 0049: Freeze unproven 1.0 subsystems behind off-by-default toggles (E3)

- Status: Accepted
- Date: 2026-11-13

## Context

`ROADMAP_1.0.md` E3 requires freezing unproven subsystems (Teams, extra adapters beyond Claude Code / Codex / Antigravity, unused scheduled messages, and cloud account sync) behind off-by-default settings without deleting existing data or code.

## Decision

1. Add `gnomcode_core::FeatureToggles` with four boolean flags, all defaulting to `false`:
   - `teams_enabled`
   - `extra_adapters_enabled`
   - `scheduled_messages_enabled`
   - `account_sync_enabled`
2. Persist `FeatureToggles` in SQLite `desktop_preferences` (`key = 'feature_toggles'`). Missing or invalid rows fall back to `FeatureToggles::default()` (all `false`).
3. Expose `GET /api/v1/settings/features` and `POST /api/v1/settings/features` (bearer-authenticated) plus E2EE Remote dispatch arms `features.get` and `features.set` in `crates/daemon/src/features.rs`.
4. Gate runtime behavior without deleting tables, rows, or code:
   - `crates/ingest/src/lib.rs`: `ingest_all` skips `AgentId::GeminiCli` when `extra_adapters_enabled` is `false`.
   - `crates/daemon/src/automation.rs`: `deliver_due` and `schedule` creation are disabled when `scheduled_messages_enabled` is `false` (reading and cancelling existing rows still work).
   - `crates/daemon/src/sync.rs`: `spawn_cloud_sync` skips `push_snapshot` when `account_sync_enabled` is `false`.
   - `crates/daemon/src/pro.rs`: `is_teams_enabled` returns `false` unless `teams_enabled` is `true`.

## Consequences

- Fresh 1.0 installations have zero background traffic from account sync, zero scheduled message deliveries, and zero extra-adapter scans while preserving upgrade compatibility and existing user data.
