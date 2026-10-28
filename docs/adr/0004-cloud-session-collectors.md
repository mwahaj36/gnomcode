> Withdrawn in v1.4.1 at the owner’s request. Retained as a historical decision record.

# 0004 — Opt-in cloud session collectors

- **Status:** Accepted
- **Date:** 2026-10-27

## Context

Hosted coding sessions may expose runtime identity without local rollout logs.
Desktop-only filesystem discovery cannot observe another machine. A single
per-user live snapshot also permits concurrent devices to replace one another.

## Decision

Run an explicit, portable `cloud-session` collector inside each environment.
Detect Codex cloud identity from runtime metadata; require explicit native IDs
for other supported agents. Record collector presence independently of agent
activity. Reuse native adapters for measured usage or import strictly typed,
metadata-only invocation deltas and price them in Rust. Never infer tokens from
elapsed time. Label unavailable data unknown and API-equivalent cost estimated.

Use a separate Supabase row per account/session with owner RLS, server timestamps
and expiry at read time. The web console combines these rows with desktop data.
Pairing is explicit and reuses the existing Supabase user-login refresh flow.
Publish only typed session summaries with repository labels and titles removed.

## Consequences

Windows, macOS and Linux share one Rust implementation. Basic cloud presence
works even when usage is unavailable. Provider telemetry exporters, account-wide
hosted-session discovery and cloud aggregate report charts remain future work.
Production visibility requires the additive database migration, an updated web
client, account pairing and a running collector. A live heartbeat represents the
collector, not agent activity. SQL and local storage both expire stale presence.

## Alternatives considered

- Guess costs from duration — cannot provide defensible usage numbers.
- Copy conversation content into the repository — violates content privacy.
- Overwrite the desktop sync snapshot — loses concurrent session data.
