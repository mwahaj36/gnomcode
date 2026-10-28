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
   - The mobile device connects via a pairing link, exchanges public keys, and both sides compute a shared secret.
   - A 6-digit code derived from the room, roles, both public keys, and shared secret is compared and confirmed on both screens to defeat active MITM attacks.
   - Subsequent traffic uses directional AES-256-GCM keys and monotonic sequence numbers.

3. **Remote Capability Boundaries**:
   - The phone interface pages through available conversation history and polls for subsequent responses. Prompt input is limited to an idle Spawnt-hosted session with a verified local PTY binding; an explicit model handoff may launch a fresh supported agent session. Conversation traffic uses the encrypted Remote channel and never the unencrypted metadata sync.
   - Destructive operations, arbitrary file downloads, raw terminal command injection, and permission-escalation flags are strictly forbidden over remote channels.
   - Revocation from the desktop closes the room and invalidates keys. Starting a new pairing generates a fresh room and keypair.

## Consequences

The relay remains simple and holds only active room connections. Live two-device validation and an independent security review are required before production availability.
