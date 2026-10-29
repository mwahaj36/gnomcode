## 2026-10-28 — Phase 5: M9 Remote prototype (incomplete)

- ADR 0019 and a preliminary threat model describe the intended protocol. They are design documents, not an independent security review.
- Domain types (`crates/core`) describe pairing and frames but do not perform key agreement or encryption.
- The Cloudflare Worker forwards opaque frames between two sockets and has mock tests. No encrypted desktop or phone peer is connected to it yet.
- The original pairing, status, and revoke endpoints claimed operations that did not occur; they are disabled until those operations are implemented.
- The original phone page simulated pairing locally; it was replaced with an honest unavailable state. End-to-end chat history, prompt submission, and live response streaming remain required.
