# 2026-11-13 - Needs-you notifications and push redaction

## Goals

- Implement Rust-owned notification decisions in `gnomcode-core::notifications` for the four lifecycle events: `approval_waiting`, `turn_finished` (after strictly more than `N` minutes, default 2), `limit_low` (`QuotaState::Low`), and `limit_reset` (`reset_passed` followed by a fresh reading).
- Enforce deduplication, 5-minute per-(subject, event) rate limiting, burst grouping, and payload redaction (only redacted session title, agent name, and event kind; zero chat content).
- Deliver through the existing desktop notification path and the authenticated VAPID Web Push path (`web/lib/push-policy.ts`, `/api/push/send`), with per-event toggles and duration setting persisted in SQLite (`GET/POST /api/v1/settings/notifications`) and surfaced in Settings.
- Leave a documented hook point (`automation::notify_approval_waiting_hook` / `NotificationEngine::record_approval_waiting_hint`) for the separate phone-approvals feature without modifying existing approvals hooks.

## Outcomes

- Added `crates/core/src/notifications.rs` with `NotificationEngine`, `NotificationSettings`, `RedactedNotificationPayload`, burst grouping, title redaction, and 7 unit tests.
- Exposed `quota::reset_passed` as public for deterministic reset transition tracking.
- Added SQLite migration v15 in `crates/store` (`notification_settings` table and `automation_notice.push_payload` column) with persistence and push metadata roundtrip tests.
- Replaced ad-hoc polling logic in `crates/daemon/src/automation.rs` with `NotificationEngine` and added `notify_approval_waiting_hook` as the integration point for live approval events.
- Updated `crates/daemon/src/sync.rs` to forward redacted event/agent metadata (session titles stay on the desktop; reviewer fix) to `/api/push/send`, and added `parsePushNotificationRequest` in `web/lib/push-policy.ts` (tested in `web/test/push-policy.test.mjs`) to validate and render push alerts while maintaining backwards compatibility with generic test pushes.
- Added `/api/v1/settings/notifications` endpoints in `crates/daemon/src/api.rs` and the Settings -> Notifications form in `console.html`, `console-app.js`, and `gnomcode-console.css`.

## Decisions

- Kept notification state evaluation pure in `crates/core` so time-dependent transitions, 5-minute rate limits, burst grouping, and redaction are deterministic and unit-tested without I/O.
- Stored serialized `RedactedNotificationPayload` alongside `automation_notice` in `push_payload` so both desktop and background Web Push retries reuse the same redacted metadata without altering existing notice callers.
- Grouped multiple simultaneous alerts into a single bounded summary notification (`<= 240` chars, single line) while preserving per-(subject, event) rate-limit timestamps individually.

## Verification

- `cargo fmt --all`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `CARGO_INCREMENTAL=0 cargo test --workspace`
- `web/`: `npm test`, `npm run typecheck`, `npm run build`
