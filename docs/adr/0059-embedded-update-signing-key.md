# 0059 - Embed the desktop update verification key

- **Status:** Accepted
- **Date:** 2026-11-13
- **Amends:** 0033

## Context

A runtime-only public key permits a packaged update client to run without an authenticity requirement. SHA-256 alone establishes integrity against a manifest, not a trusted publisher.

## Decision

Embed GNOMCODE_UPDATE_PUBLIC_KEY using Rust option_env! at build time. An embedded value takes precedence over any runtime value, including malformed or empty embedded values: verification then refuses installation rather than downgrading to hash-only checks. Keep the existing runtime option only for builds without an embedded key. The daemon requires both SHA-256 and the existing Ed25519 signature over installer bytes before returning ready; Electron retains its ready-state, path and hash checks before explicit installation. Errors do not interrupt app or agent use.

Offline Node release tooling signs actual artifact bytes into manifest platform signature fields using an owner-supplied key, checks the expected public key and hashes, and never uploads anything. No production private or public key is created by this change.

## Consequences

A release must supply and review the public build input. Private keys remain outside builds and repositories. Existing clients with no embedded/runtime key retain development hash-only behavior. Envelope metadata remains protected by approved HTTPS origins and monotonic version selection, not by the artifact signature. Rotation requires an old-key-signed bridge or an independently trusted manual installation. See docs/release-signing.md.

## Alternatives considered

- Runtime key only: cannot make authenticity mandatory in a shipped binary.
- Committed production public key: valid future option, but the owner has not supplied one.
- Generate keys automatically: prohibited; owner-controlled release ceremony remains separate.
