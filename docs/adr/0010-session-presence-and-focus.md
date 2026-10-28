# 0010 — Shared session presence and truthful focus

- **Status:** Accepted
- **Date:** 2026-10-28

## Decision

All agents use the same pure Rust presence classifier. A recent running observation or an unmatched execution start consistent with the source state establishes running. Approval waits remain until resolved. Idle remains ongoing for two minutes; stale observations without execution evidence become inactive, never implicitly ended. Only source-reported closure establishes ended. SQL filters, totals, CLI, tray and desktop snapshots use this projection; clients render supplied classifications.

Focus targets the recorded PID/host or known application, with blocking OS calls outside the database lock and async worker. The endpoint reports the actual result rather than unconditional success. Windows restores minimized windows and verifies foreground ownership; macOS and X11 use available OS controls. Unsupported focus offers a local workspace-copy fallback. Opening a window does not claim exact chat selection.

Windows app icons use the requested 0.65x inset; the tray retains a separate full-size icon generated from the same canonical SVG. No permission escalation, new telemetry or content storage is introduced.

## Consequences

Silent sources without execution-start evidence become inactive conservatively. Cloud views reflect the last daemon snapshot, not a browser-created presence estimate. OS focus remains best effort and macOS builds remain deferred. Windows credential ACL writes use the same protected owner-only rules through .NET without requesting audit privileges.
