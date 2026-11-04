# 0027 - Laptop-owned scheduling and quota recovery

- **Status:** Accepted
- **Date:** 2026-11-04

## Context

A scheduled message created on a phone must still reach its agent when the phone disconnects. The laptop already owns CLI processes and conversation identity. Routing policy existed without a quota-triggered execution path; the old reset queue spawned detached CLIs instead of reopening the hosted conversation.

## Decision

Use local SQLite for redacted scheduled messages, UTC deadlines, durable delivery claims and generic lifecycle notifications. Both authenticated loopback and authenticated encrypted Remote use the same Rust command handler. No scheduled text enters Supabase snapshots, the account directory or an unencrypted cloud command queue.

Run automation in a separate daemon worker from ingestion. Wait through busy or approval states and require fresh observed idle evidence between scheduled deliveries. Cancel through the same conversation-control lock. Erase text after terminal outcomes and cancel pending messages on chat deletion. Retain pending messages until execution/cancellation; expire terminal metadata and notifications after 30 days. A crash after claiming becomes uncertain on restart and is never replayed automatically.

Opt-in automatic routing uses a recent native rate-limit event, an authenticated/configured official CLI catalog and fresh known quota. Select the eligible model with the most headroom, including model-pool evidence for same-provider switching. Recheck policy and current conversation before switching, preserve native permission controls and bounded handoff context, and do not replay the previous task. Record an attempt before execution to prevent loops.

Reset reconnection requires observed exhaustion followed by fresh positive quota after eligibility across the provider's known windows. Claim the armed entry under the shared control lock. Reuse the live official CLI or reopen its recorded native conversation; never submit a continuation task. A confirmed reset permits messaging after that exact observed rate-limit error, without overriding a later error or approval. Switching a model cancels the old reset entry.

Persist generic event feedback for the UI. The daemon dispatches bounded native notifications for schedules, switches, completion, approval, errors, guardrails and quota changes. Remove overlapping approval/reset dispatch from the legacy Windows tray. OS notification availability is not an execution guarantee; local and Remote status remains readable.

## Consequences

The phone can close after receiving a save acknowledgment. The laptop must remain awake with Gnomcode running; sleeping machines are not awakened. Native terminal acceptance is not model completion or observed activation. Redaction can change scheduled text, which the UI discloses. Native CLI compatibility, physical-phone delivery and macOS/Linux packaged notification behavior remain release acceptance checks.

## Alternatives considered

- Browser timers: require the phone/browser to remain active and lose work after reload.
- Unencrypted cloud queues: violate the local-history boundary and introduce another execution authority.
- Automatic replay after uncertain delivery: risks duplicate coding operations.
- Detached CLI resume: loses hosted ownership and does not preserve the current conversation binding.
