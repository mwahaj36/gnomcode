## 2026-10-28 — truthful cloud settings writes

- Removed unauthenticated loopback reads/writes from signed-in Settings. Cloud preferences and local desktop configuration have separate authority, matching the console boundary.
- Rejected saves now display errors, signed-out saves are blocked, duplicate saves are prevented, and pricing rows update only after persistence succeeds. Missing rates stay unknown; zero-valued plan preferences no longer revert to defaults. Added explicit field names and nonnegative plan constraints.
- Verification: mocked component regression exercises successful/rejected/signed-out writes; production web build and diff checks passed. Live account mutation was not performed.
