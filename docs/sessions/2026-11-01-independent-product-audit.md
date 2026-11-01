# Independent product audit — 2026-11-01

Goal: inspect product behavior skeptically, document issues and market gaps, and replace the product brief with an evidence-based account.

Outcome: added `docs/APP_AUDIT_2026-11-01.md` and rewrote `PRODUCT.md`. Identified the capped account snapshot total, non-universal hard stops, broad sync metadata, masked diff failures and user-facing search/deletion limitations. Compared primary competitor documentation; proposals are not approved scope.

Verification: web `npm test` passed 18 tests and two presentation checks. `cargo test --workspace` could not start because cargo is unavailable. No real agent, phone, desktop, production account or cross-platform acceptance was performed. The review is risk-based and does not establish exhaustive coverage or security.

Decision: retain SPEC as the requirements authority; no product scope or architecture was changed. Launch and privacy risks remain open for owner decisions and independent validation.
