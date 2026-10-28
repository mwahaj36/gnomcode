# 0013 — Remote encrypted relay direction

- **Status:** Proposed; implementation requires separate owner confirmation, threat model and independent security review
- **Date:** 2026-10-28

## Decision

Remote is a paid, opt-in feature. Remote content may leave the machine only as end-to-end encrypted traffic. The relay must not receive plaintext or encryption keys. Device pairing, authentication, key storage, revocation, replay protection and metadata exposure need a threat model and independent review before launch. No cryptographic construction or relay provider is approved by this record.

## Consequences

Existing cloud sync is not automatically E2EE and must not be described as Remote. Its migration and the Teams metadata boundary require separate review. This record does not authorize M9 work.
