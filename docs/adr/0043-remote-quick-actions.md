# ADR 0043: Remote Quick-Actions Storage

- Status: Accepted
- Date: 2026-11-13

## Context

Worker B's Remote Companion UI needs user-configurable quick-action prompts (short labels + prompt templates) persisted locally by the daemon and reachable both over loopback HTTP (`GET`/`PUT /api/v1/quick-actions`) and over E2EE Remote (`quick_actions`).

## Decision

1. Store quick actions in `$GNOMCODE_HOME/quick_actions.json` (versioned schema, bounded list size, bounded label/prompt lengths, unique non-empty IDs, atomic write via temp file + rename).
2. Seed sensible read-only/status defaults when no file exists yet without writing until the user saves.
3. Expose `GET` and `PUT` `/api/v1/quick-actions` behind bearer + Host/Origin validation, plus Remote `quick_actions` (`action: "get" | "set"`), returning the canonical list.

## Consequences

- Zero business logic in TypeScript; all validation and persistence live in `crates/daemon/src/quick_actions.rs`.
- No secrets or conversation history are stored; quick actions are user-authored prompt shortcuts.
