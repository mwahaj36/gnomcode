# ADR 0034: PWA App Lock (Device Passkey / PIN)

- **Status:** Accepted
- **Date:** 2026-11-13
- **Owners:** Worker B (`web/lib/app-lock.js`, `web/components/AppLockGate.tsx`, `web/app/console/Client.tsx`, `web/components/SettingsPanel.tsx`)

## Context

Users installing Gnomcode's `/remote` or `/console` web app on shared or unlocked phones want a quick biometric passkey or device PIN lock before exposing paired agent chats and tool approvals. Adding a lock must never weaken or bypass the end-to-end encryption layer established with the paired desktop (`web/lib/remote-crypto.mjs`, `web/lib/remote-device.ts`).

## Decision

1. **Client-side presentation gate only:** App lock (`gnomcode-pwa-lock-v1` in `localStorage`, session timestamp in `sessionStorage`) gates rendering of `<Console />` via `<AppLockGate>`. It never reads, derives, wraps, exports, or replaces X25519/AES-GCM session keys or remembered device HMAC proof keys in IndexedDB (`gnomcode-remote-device`).
2. **Two local unlock modes:**
   - **Device PIN (4-8 digits):** Digested with WebCrypto `PBKDF2` (`SHA-256`, `210,000` iterations, random 16-byte per-device salt) and constant-time byte comparison. Plaintext PINs are never persisted.
   - **Passkey / WebAuthn (`isSecureContext` + `PublicKeyCredential`):** Uses platform authenticator (`userVerification: 'required'`) and validates `clientDataJSON.origin === window.location.origin` and non-empty `authenticatorData`.
3. **Brute-force cooldown & safe recovery:** Consecutive failed unlock attempts (`10` max) trigger a `10`-minute persisted cooldown. Users who forget their PIN/passkey can recover only via **Reset & forget device**, which clears app lock AND invokes `forgetDevice()` to remove remembered paired desktop keys from IndexedDB, forcing fresh pairing.
4. **Unlocked enrollment invariant:** Changing or enrolling a PIN or passkey fails closed unless the session is already unlocked.

## Consequences

- Paired desktops keep full end-to-end encryption guarantees regardless of whether PWA app lock is enabled.
- Automated workflows and unconfigured browsers experience zero friction when app lock is `off`.
