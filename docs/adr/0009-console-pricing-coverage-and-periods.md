# 0009 — Pricing coverage and source-owned reporting periods

- **Status:** Accepted
- **Date:** 2026-10-28

## Context

One unpriced invocation hid all report, provider and model costs. Today could show two calendar dates because a local-midnight cutoff was grouped by raw date strings. Overview placed ongoing sessions after historical charts and limits.

## Decision

Rust supplies priced and unpriced invocation counts for timeline, interval, provider, model, session and project projections. Dollar amounts exclude unpriced records and are explicitly partial when coverage is incomplete; no model rate is guessed in JavaScript. Existing unknown aggregate trust remains unknown.

Today uses local-midnight boundaries and UTC hourly buckets, displayed as local clock times. Longer periods use UTC daily buckets. Calendar month starts at source-local midnight on the first day. Cloud snapshots publish the daemon's precomputed month; web code never recomputes costs or periods.

The shared Overview places ongoing-session identity and provider limits before historical activity. Full session history supports a local project-ID query filter before pagination. Cloud project identity remains private; the web sessions filter only names already present in the opted-in snapshot.

## Consequences

Older snapshots lacking coverage stay conservative until the desktop updates. Missing intervals are not fabricated as zero. Both consoles use the same presenters and responsive layout, with separate authentication and data paths. Windows retains its existing tray runtime and Unix retains browser fallbacks.

## Alternatives considered

- Show partial sums as complete totals: misleading.
- Hide every provider when one is unpriced: discards available information.
- Compute calendar rollups in the browser: violates the Rust business-logic boundary.
