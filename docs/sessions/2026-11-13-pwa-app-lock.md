# 2026-11-13 — PWA app lock (passkey / device PIN)

## Goals

Protect the PWA (`/remote` and `/console`) with an optional device passkey (WebAuthn) or local device PIN without modifying or weakening Remote end-to-end encryption keys.

## Outcomes

- Authored [ADR 0034](../adr/0034-pwa-app-lock.md) and added its row to `docs/SPEC.md` §9.
- Implemented `web/lib/app-lock.js` & `.d.ts`: WebCrypto PBKDF2 (`SHA-256`, 210,000 iterations, random salt) PIN hashing, constant-time verification, WebAuthn platform passkey enrollment/assertion (`isSecureContext` + origin check), 10-attempt / 10-minute cooldown lockout, and fail-closed unlocked enrollment checks.
- Implemented `web/components/AppLockGate.tsx` (`AppLockGate` overlay + `AppLockSettings` panel) wired into `web/app/console/Client.tsx` and `web/components/SettingsPanel.tsx`, with safe `"Reset & forget device"` recovery that clears both lock state and remembered E2EE credentials (`forgetDevice`).
- Appended responsive `--sp-*` CSS rules in `web/app/gnomcode-console.css` (zero raw hex tokens, 44px mobile touch targets, 16px mobile inputs).

## Verification

- `web/test/pwa-app-lock.test.mjs`, `web/` `npm ci && npm test && npm run typecheck && npm run build`, headless Chromium visual check of the lock screen and settings panel at `390x844` and `1440x900` dark/light, and full `CARGO_INCREMENTAL=0` Rust workspace gate.
