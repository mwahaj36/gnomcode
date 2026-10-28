# 0019 — Remote access, zero-knowledge relay, and end-to-end encrypted pairing

- **Status:** Accepted
- **Date:** 2026-10-28

## Context

Users want to monitor agent sessions, respond to approval requests, and view quota status from mobile devices when stepping away from their computer. However, exposing local agent daemons directly to the public internet presents severe security risks (firewall traversals, open ports, DNS rebinding, and unauthorized shell execution).

Furthermore, routing conversation content or tool outputs unencrypted through a third-party server directly violates Spawnt's privacy and data protection principles.

## Direction

1. **Zero-Knowledge Relay (`workers/remote-relay`)**:
   - A Cloudflare Worker backed by Durable Objects manages stateful pairing rooms (`/relay/:roomId`).
   - The relay bridges two WebSocket connections (desktop daemon and phone PWA).
   - The relay stores zero message content and inspects no frame payloads: it forwards sealed binary ciphertext frames between paired devices.

2. **Ephemeral Pairing & E2EE Key Derivation (`crates/core`, `crates/daemon`)**:
   - The desktop initiates a pairing session, generating a one-time room ID and an ephemeral X25519 keypair.
   - The mobile device connects via QR code or pairing link, exchanges public keys, and both sides compute a shared secret.
   - A 6-digit confirmation code (derived from the shared secret) is verified by the user on both screens to defeat active MITM attacks.
   - Subsequent traffic is sealed with AEAD encryption (ChaCha20-Poly1305 / AES-GCM) with monotonic sequence numbers.

3. **Remote Capability Boundaries**:
   - The phone interface provides status inspection, chat streaming, prompt input, and low-risk approvals.
   - Destructive operations, arbitrary file downloads, raw terminal command injection, and permission-escalation flags are strictly forbidden over remote channels.
   - Revocation from the desktop immediately rotates the room and invalidates keys.

## Consequences

Remote access provides true mobile freedom without compromising local host security. The relay remains simple, stateless beyond active rooms, and cost-effective.
