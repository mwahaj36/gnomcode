# 2026-11-14 - Live working state and stop notifications

## Goals
- Distinguish Working, Waiting for you, Idle and Stopped in the chat on desktop, web/PWA and Remote, with the current step, elapsed time and Stop.
- Notify only when an agent stops completely or needs the user, once per stop.

## Outcomes
- `core::presence::live_status` derives the live state, its trust basis and a content-free step label. `store` adds `live` and `turn_started_at` to session rows; the conversations API (local and Remote) adds the same through `Database::live_inputs`.
- `NotificationEngine` adds `session_stopped` and `needs_attention`. Busy or waiting then ended notifies once. Idle then ended stays silent. Errors notify when they first appear.
- Console: `renderLiveBadge` (header, list rows), `renderActivityIndicator` (step plus elapsed, Stop), and a Queue composer label while working. Push labels added in `web/lib/push-policy.ts`.

## Problems and root causes
- The UI guessed activity from raw state strings. Live state is now computed once in Rust, with `basis` so inferred states are labeled.
- An exit while working was never notified, because only Idle transitions produced alerts.

## Decisions
- The new alerts reuse the existing preferences (`turn_finished`, `approval_waiting`), so no settings schema migration is needed.
- Stop reuses the existing interrupt path; no new control endpoint.
- Remote keeps its existing rule that prompts are sent only when idle; it shows the state and Stop but does not queue.

## Verification
- `cargo test -p gnomcode-core` (live_status cases, six notification cases including no notification while working and turn end plus exit), store hosted-session test asserts `live`.
- `npm test` (`live-state.test.mjs`), Playwright `e2e/live-state.spec.ts`: every state at 1280px and 390px, plus reduced motion.
