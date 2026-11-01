# Audit remediation — 2026-11-01

Goal: address the independent audit's confirmed defects with the smallest complete changes and make remaining limits visible.

Changes: account snapshots now report the true count and disclose their bounded 200-session list. A session-field allowlist excludes titles, and unused profile and relay records are removed from new snapshots; account connection copy identifies the data and its separate encryption model. Repository diff failures no longer look like empty success. Search labels identify metadata-only scope, history deletion explains agent logs and backups, and guardrail copy identifies Claude-only, fail-open enforcement and estimated costs. Updated SPEC and product/audit documents.

Root causes: the snapshot used page length as the total and serialized broad domain objects, including potentially prompt-derived titles; the Git endpoint ignored process errors; presentation implied broader capabilities than the implementations establish. Regression tests reproduced the count, title and Git cases before correction.

Verification: Rust quality script completed formatting, clippy and workspace tests on Linux; cargo-deny was unavailable and skipped. Web tests and typecheck passed. Later small changes to the test URL and disclosure copy require the final verification recorded in the task response. Physical phones, authenticated agents, other operating systems, production RLS and independent Remote security were not available. Existing cloud copies are not erased by changing future payloads. Diff output and execution time are now bounded. Competitor opportunities require separate product decisions and are not implemented.
