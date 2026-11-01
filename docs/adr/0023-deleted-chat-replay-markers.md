# 0023 — Deleted chat replay markers

- **Status:** Accepted
- **Date:** 2026-11-01

## Context

Spawnt preserves agent-owned logs, and reconciliation can replay a source after a checkpoint reset or rewrite. Deleting local chat rows alone lets previously deleted events return. Source event identifiers may contain arbitrary text, and no automatic retention schedule is currently implemented. The stronger deletion and retention language in ADR 0017 does not establish forensic erasure or a running scheduler.

## Decision

Store SHA-256 digests of deleted event IDs in the local SQLite database, atomically with manual deletion or retention pruning. Direct chat insertion and source ingestion reject matching digests. Keep markers until the database is reset; never include them in account snapshots. Use one UTC instant cutoff for both marker creation and pruning. Continue to preserve agent-owned logs and permit new event IDs.

## Consequences

Stable-ID replay no longer restores deleted chat, including after restart. Markers grow with deletions and survive independently of the deleted rows. Digests are unsalted and predictable identifiers may be guessed, so protect them like the rest of the local database. Changed IDs, old backups, earlier deletions and a database reset are not covered. Older Spawnt binaries ignore markers and can reimport events; upgrade and restart all Spawnt processes together. Session titles and other metadata, agent-owned files, WAL pages and backups may retain information. Row deletion is not forensic erasure. Automatic retention, backup policy and broader deletion guarantees remain open.

## Alternatives considered

- Delete rows only — fails when sources replay.
- Delete or rewrite agent logs — violates the requirement to preserve agent-owned history.
- Store raw IDs — can retain arbitrary source-supplied text after chat deletion.
- Suppress an entire conversation indefinitely — could discard legitimate future messages.
