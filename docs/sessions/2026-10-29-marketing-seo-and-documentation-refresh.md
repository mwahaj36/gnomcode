# 2026-10-29 — Marketing, SEO and documentation refresh

## Goals

Deliver the supplied marketing/SEO brief alongside an accurate product brief, refreshed specification and one-file-per-session history. Owner clarified that marketing should be optimistic; release caveats belong in docs with a separate actionable launch checklist.

## Outcomes

- Split all 76 historical entries into individual files with an index and compatibility navigation at SESSION_LOG.md. Verified every original block is preserved exactly, including duplicate session names and historical verification language.
- Rewrote PRODUCT.md around actual workflows, audience, capabilities, commercial model, data boundaries and discussion questions. Refreshed SPEC and archived the previous specification rather than discarding design history. Updated contributor/session workflow references.
- Added LAUNCH_CHECKLIST.md with blockers, causes, closure steps and required evidence. Remote security/physical-phone/provider acceptance, metadata minimization, platform delivery, signing/legal/payment/distribution remain explicitly open.
- Repositioned landing copy around launch, model continuity and encrypted remote control; retained the established visual system and console shell. Added support row, workflow feature copy, platform picker, visible FAQ and matching SoftwareApplication/FAQ schema.
- Added 12 actual guide routes, including a 24-question FAQ, plus public Privacy/Terms and a byte-identical bundled EULA. Corrected old no-history/hash-only/privacy claims. No governing jurisdiction invented; legal approval remains a launch gate.
- Added per-route canonical/OG/Twitter metadata, private-route noindex, content-only sitemap, crawler exclusions, legal navigation and a rendered 1200 × 630 social image/specification.
- Standardized public activity labels and monetary trust badges, preserved explicit ended-session feedback, improved empty/error actions and removed obsolete local-Pro copy. No agent configuration, security protocol, listener or platform process behavior changed.

## Problems and root-cause fixes

- Existing docs mixed a pre-M0 header, historical paid-local tiers and later chat/Remote changes. Consolidated current decisions and separated historical records from normative status.
- Actual sync::build_remote_snapshot serializes reports/profile/session labels and automation metadata, rather than only HMAC rollups. Documented current behavior and a field-level minimization/consent gate; did not silently remove existing sync features.
- Next client pages cannot export metadata. Extracted their original contents to same-directory Client.tsx files and added thin server wrappers; updated the existing source-level check to inspect the client implementation.
- Browser automation required the bundled Playwright package and installed Chrome path. Initial acceptance failed on missing social metadata; the implemented metadata resolves it.
- A concurrent dev server wrote into the production .next directory during verification. Stopped the task-owned dev server, rebuilt, and ran production checks separately.
- Existing presentation assertions depended on old labels/paths. Updated those assertions while retaining escaping, unknown-data and pricing-coverage checks. New tests were observed failing before status/badge implementation and before restoring explicit ended feedback.

## Decisions

Keep marketing optimistic without inventing availability, prices, ratings or universal agent control. Keep exact implementation caveats in technical docs. “Zero-token” means no model summarization call, not zero receiving-model input cost. Preserve unknown measurement states and ended lifecycle feedback. Do not publish current local untracked download binaries as part of this change. Preserve the approved fonts/palette.

## Verification

- Full Rust gate: rustfmt, clippy -D warnings, 153 workspace tests, cargo-deny and core-purity checks passed.
- Web suite: 17 tests plus shared presentation checks passed; optimized Next build passed.
- Browser acceptance: 19 public routes checked for status, unique metadata, canonical/OG fields, parseable structured data, working internal guide links, FAQ parity, sitemap and robots. Phone/desktop layouts in dark/light captured and inspected.
- Session migration: all 76 source blocks preserved exactly. Current product/spec/checklist/README links resolve.
- Cross-platform review: only docs, presentation copy/formatting and Next metadata wrappers change; no path/process/IPC behavior changed. macOS/Linux and physical-phone execution are not claimed.
- Public production crawl/Schema.org service validation and legal approval remain launch checklist items. This work does not release an installer or certify Remote security.

## Review

Focused branch: feat/marketing-seo-overhaul. PR and deployment evidence are recorded in the final task report. Copy changes are listed in ../marketing-copy-audit.md and the PR description.
