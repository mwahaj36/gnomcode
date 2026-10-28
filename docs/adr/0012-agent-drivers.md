# 0012 — Agent drivers with terminal fallback

- **Status:** Accepted direction; implementation pending M7
- **Date:** 2026-10-28

## Decision

Run official agent clients without reading, storing or forwarding subscription tokens. Prefer verified ACP-compatible drivers where they preserve the official client's authentication and permission semantics; use a verified native protocol when appropriate and a terminal fallback when structured integration cannot meet those conditions. Normalize events in Rust. Spawnt maintains its own locally persisted conversation history to support model switching, while retaining agent-owned history and native resume identifiers. Storage access controls, retention and deletion require a concrete design. Pin supported integrations only after verifying upstream documentation and real behavior.

## Consequences

Availability and exact protocol versions require a spike. Agent permission prompts must not be bypassed. PTY lifetime and reattachment require separate cross-platform testing before promising crash survival.
