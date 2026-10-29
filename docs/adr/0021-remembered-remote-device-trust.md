# ADR 0021: Remembered Remote device trust

Status: Accepted (2026-10-29)

Phone reload/background suspension previously destroyed the only transport channel. Keep one locally protected desktop trust record and one browser nonexportable HMAC signing key in IndexedDB. Trust expires after seven days without foreground activity; polling, reconnect and discovery do not renew it. Revoke removes the private record under a serialized persistence gate and invalidates old room generations.

Each connection uses a new phone X25519 key and random desktop challenge. Bind the challenge into directional AES-GCM channel derivation, then require encrypted HMAC proof over room, challenge and phone public key. Never reset counters under the same directional keys. Initial trust is issued only after desktop verifies the code entered from the phone and the phone confirms. Desktop restart restores only unexpired trust. Browser sign-out/forget removes its remembered key. The owner may revoke from desktop at any time. No credential or transcript enters unencrypted cloud metadata.

Legacy five-minute pairing remains readable through an advertised protocol version; remembered trust requires the new desktop. The zero-knowledge relay keeps its existing opaque forwarding contract. Local secrets use platform owner-only atomic writes. A shared lifecycle gate prevents delayed writes from restoring revoked trust.

The phone reserves the visual viewport for conversation and bounded composer. New conversation is a top-level action; directory browsing uses a modal with fixed confirmation/cancel actions. Read-only imported history is explicitly labeled. A live supported hosted CLI accepts its initial prompt before history exists; shells and observed busy/approval states remain blocked over Remote.
