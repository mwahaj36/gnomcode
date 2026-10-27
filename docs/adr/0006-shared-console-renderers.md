# 0006 — Shared console renderers and daily measurements

- **Status:** Accepted
- **Date:** 2026-10-28

## Context

Matching token colors did not prevent local HTML and React layouts from drifting. Both consoles need daily activity and quota freshness, while retaining separate authentication and data transport.

## Decision

Share framework-neutral, escaped HTML presenters and a stylesheet for daily activity and quota panels. React and the daemon consume the same source files. Presenters only format Rust-produced measurements and proportions; aggregation and provider cache accounting remain in Rust. The updater preserves expanded day details and focus across polling. A shared layout stylesheet defines the navigation rail and its narrow-screen fallback.

Snapshots carry a source-generated timestamp independent of cloud settings-row updates. The daemon exposes authenticated metadata-only cloud publication status. Unobserved dates and unavailable daily measurements are explicit unknown/empty states rather than invented zeros.

## Consequences

These panels have identical markup and interaction in both consoles. Legacy snapshots remain readable and show daily measurements unavailable until the daemon is upgraded. Other existing sections retain their transport-specific controls. Release assets must include the renderer, stylesheet, fonts and licenses inside the daemon.

## Alternatives considered

- Duplicate equivalent React and HTML components — rejected because styling and state handling had already diverged.
- Merge authentication or proxy local data through the website — rejected because it would weaken the local/cloud privacy boundary.
