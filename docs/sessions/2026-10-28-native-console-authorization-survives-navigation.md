## 2026-10-28 — native console authorization survives navigation

- User screenshot showed a rendered console with all local reads unavailable. Direct authenticated daemon probes returned 200 for report, sessions and quota. Reproduction showed setView replaced the URL with `/` or `/#view`, dropping the bearer query; a subsequent page reload lost authorization. The brand link also navigated to an unauthenticated root.
- Preserve the complete current query when changing views or following the brand link. No auth bypass, automatic credential recovery or persistent browser token storage was added.
- Local reads now expose actionable authorization failures, and failed report reads resolve the daily-activity loading state. Render caches are committed only after successful rendering so a transient UI failure can be retried.
- Verification: functional navigation/reload and failed-401 regression reproduced the issue before the fix; native script/target checks and full Rust gate passed.
