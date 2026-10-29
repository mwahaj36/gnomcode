# 0020 - Account discovery of temporary Remote pairing offers

- **Status:** Accepted
- **Date:** 2026-10-29

## Context

Moving a pairing link from desktop to phone is cumbersome. A signed-in user should find the publishing desktop in Profile without sending links. Account identity must not replace the existing two-device verification.

## Decision

Publish a temporary public pairing offer alongside the existing opt-in metadata snapshot in the owner-only `spawnt_sync_state` row. Reuse its existing authenticated owner RLS; add no public endpoint or database policy. The offer contains only the random room identifier, ephemeral desktop public key, relay endpoint, expiry, and a generic desktop label. Never publish private keys, bearer tokens, verification codes, prompts, tool output or filesystem paths.

A desktop must explicitly start pairing. Offers expire after five minutes and disappear after completed confirmation or revocation. Profile queries only the signed-in user's offer and checks expiry before offering Connect. Discovery fills the existing pairing parameters; X25519, encrypted transport, matching six-digit codes and confirmation on both devices remain mandatory. Account login alone grants no Remote access. Existing direct-link pairing remains available without an account.

The existing metadata row represents the account's latest publishing desktop; this change does not introduce a multi-desktop registry. Starting/revoking an offer wakes metadata publication immediately; ordinary reporting keeps its one-minute interval.

## Consequences

The cloud can observe the public room locator and public key, but cannot decrypt Remote content. A compromised account or cloud can substitute an offer, so the matching code and physical desktop confirmation remain the trust boundary. Offline or signed-out users retain direct-link pairing. Stale offers may remain in an offline snapshot until its next update but are rejected by expiry and the desktop connection gate.

## Alternatives considered

- New device registry: unnecessary schema and credential deployment for one publishing desktop.
- Trust the account automatically: rejected because it weakens independent device verification.
- External QR service: rejected because it discloses pairing material to another service.
