# 0028 — Managed conversations and device notifications

- **Status:** Accepted
- **Date:** 2026-11-06

## Context

The live audit found imported records exposed as controllable chats, Codex tool IDs counted as sessions, stale context rotations, per-account snapshot overwrites, and notification paths that never connected native events to phone push. The Windows installer launched Edge despite an existing Electron shell.

## Decision

Persist ownership at Gnomcode launch and enforce it in Rust across prompt, switch, resume and scheduling. Imported histories remain view-only. Filter tool IDs and hide subagents by default. Let providers compact their own context. Package the existing single-instance Electron shell and attribute Windows alerts to it.

Store snapshots on each existing desktop directory row. Add authenticated, opt-in browser subscriptions and event-delivery IDs protected by owner RLS. The daemon sends only generic lifecycle event IDs; the server chooses fixed presentation text and sends it only to stored vendor-validated destinations for the authenticated account. Keep recent failed deliveries retryable and deduplicate accepted events. No message content enters this service.

## Consequences

A phone can receive generic notifications after closing Remote while its desktop remains online. Browser permission and a saved subscription are required; permission alone is not proof of delivery. A deployment and native update must be tested together. The server stores subscription keys and event identifiers as revocable account metadata. Actual notification display remains subject to OS permission and focus settings.

## Alternatives considered

- Browser polling alone cannot deliver when a phone suspends its page.
- Accepting arbitrary push destinations per send exposes an outbound request primitive and does not connect lifecycle events.
- Automatic rotations conflict with provider compaction and create stale prompts.
