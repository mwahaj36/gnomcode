# 2026-11-04 — VAPID Server Push Integration

## Goals
- Provide complete server-side VAPID Web Push notification infrastructure for cold background delivery when the mobile device or browser is asleep.
- Ensure zero secrets or keys are committed to the repository; all credentials load strictly from environment variables.
- Connect client-side subscription and server-side dispatch API routes.

## Outcomes
- **Server API Routes**:
  - `POST /api/push/send`: Signs payloads with VAPID credentials via `web-push` and delivers directly to browser/OS push endpoints (APNs/FCM).
  - `GET /api/push/vapid-key`: Exposes the public key to the client when configured.
- **Client Infrastructure (`web/lib/notifications.ts`)**:
  - `subscribeToPushServer()`: Reads the server VAPID key and subscribes via `registration.pushManager.subscribe`.
  - `triggerServerPushTest()`: Sends test payload to verify server delivery.
- **Key Safety**:
  - `web/lib/vapid.ts` reads exclusively from `process.env.NEXT_PUBLIC_VAPID_PUBLIC_KEY`, `process.env.VAPID_PRIVATE_KEY`, and `process.env.VAPID_SUBJECT`.
  - Returns graceful `503` service unavailable when keys are not configured.

## Verification
- `npm --prefix web run typecheck`: clean (0 errors).
- `npm --prefix web test`: all 21 tests pass.
- `npm --prefix web run build`: static and dynamic routes compiled.
- `.agents/skills/rust-quality-gate/scripts/check.ps1`: 5/5 quality gates passed.
