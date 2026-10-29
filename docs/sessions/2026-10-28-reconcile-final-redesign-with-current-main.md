## 2026-10-28 — reconcile final redesign with current main

- Final fetch found main had advanced through PRs 44–46, including the owner's withdrawal of hosted cloud sessions/browser device pairing. Integrated that main history before merge, retaining private desktop credential handling, desktop summary sync and the existing manual account-token flow while removing hosted-session reads/collectors from the redesigned web UI.
- Retained shared presenters, truthful freshness and reload behavior. Fixed merge-time missing status targets and a test AppState initializer. A regression reproduced retired hosted usage leaking into reports; all current report/daily/project queries now exclude those rows without deleting history.
- Renumbered redesign ADRs to 0006–0008 to preserve main's withdrawn ADR 0005 and avoid duplicate identities. Historical published release archives are retained from main; the current release remains 1.5.0.
- Windows full Rust gate including cargo-deny, Linux clippy/tests, Rust 1.87 check, production web build and source-matching Windows/Linux packaging passed. Native asset MIME/body tests and embedded syntax/runtime-target checks passed before authorized merge/publication. macOS remains deferred; browser visual checks remain unavailable.

- GitGuardian on the integration PR flags two historical occurrences of the same invalid mock JWT in main commits 0d4f190/5c2df63. PR 45 documents its synthetic signature and remediation; the fixture file is absent from current source after PR 46. No scanner configuration or exemptions were changed.
