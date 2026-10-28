# 0014 — Free local core

- **Status:** Accepted; implementation pending
- **Date:** 2026-10-28
- **Supersedes:** local Pro gating in 0002

## Decision

All local functionality, including CLI, guardrails, resume and analytics, is free, works offline, and needs no account or local license check. Only Remote and Teams are paid. The owner confirms there are no customers. Preserve existing user data and functionality while removing local feature gates and updating UI and documentation; do not silently remove cloud features.

## Consequences

License checks must not control local features, while future paid entitlements need an independently designed boundary. Current code may still contain local gates until migration is implemented and tested.
