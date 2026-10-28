# Remote pairing threat model

Status: development analysis; independent security review is required before enabling Remote.

## Assets and trust boundaries

Conversation content, approvals, device credentials, and session controls are sensitive. The local daemon and paired phone are endpoints. The relay and network are untrusted and may observe room identifiers, timing, frame sizes, and connection metadata. The relay must never receive plaintext, private keys, or bearer credentials.

## Threats and required controls

- **Active interception:** An attacker replacing public keys must not be able to complete pairing without the user comparing a short authentication string derived from the authenticated transcript. Neither endpoint may send sensitive data or accept commands before both confirmations.
- **Impersonation and replay:** Generate fresh ephemeral keys and room identifiers using a cryptographic random source. Bind roles, protocol version, room, and both public keys into key derivation and confirmation. Use separate directional keys and authenticated sequence numbers; reject duplicates, reordering, and exhausted counters.
- **Nonce reuse:** Each direction needs an independent key and a unique nonce for every encrypted frame. Restarting or reconnecting must create fresh keys or persist counters safely.
- **Relay abuse:** Limit room membership and frame sizes, expire unused rooms, and bound memory. A relay must forward opaque frames only; it must not authorize commands or expose credentials.
- **Revocation:** Revocation must close active connections, erase ephemeral key material, invalidate room state, and prevent old frames or reconnects from restoring access.
- **Unauthorized actions:** Remote capabilities must be explicitly allowlisted. Arbitrary shell input, unrestricted file access, destructive actions, and permission escalation are excluded. Approval requests require a genuine pending request and the user's explicit decision.
- **Local compromise:** Pairing secrets and tokens must not appear in logs, URLs, process arguments, or unprotected files. Local access controls remain necessary; E2EE does not protect a compromised endpoint.

## Verification required before enabling

Tests must cover both peers, transcript mismatch, wrong confirmation, replay and reordering, tampering, nonce uniqueness, expired pairing, disconnect and reconnect, revocation, oversized frames, and forbidden actions. An independent security review must assess the complete protocol and implementation before production release. Development pairing must display its actual connection state.
