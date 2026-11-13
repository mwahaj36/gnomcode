# codex-release handoff

Date: 2026-11-13. Log branch: docs/handoff-codex-release. Never merge this PR.

## Scope and constraints

Own release metadata, signing, Windows packaging, downloads and packaged smoke checklist. One item per branch/PR from origin/main. No uploads, tags, releases, credentials or deployment changes. Shared docs append-only. Console and shared check scripts remain untouched.

## Item 1 - version preparation in progress

Branch chore/release-1-0-versions, baseline 496d740. Cut-release bumper now synchronizes Cargo.lock and desktop lock metadata. Isolated regression failed first on desktop 0.9.3, then passed. Versions prepared at 1.0.0. Branding check passed. Full Rust, desktop and web gates running.

The v0.9.4 source tag is missing locally and on origin. Wahaj explicitly approved commit 7cec4e6 as release-note baseline. Notes cover 7cec4e6..496d740. Claude must decide any tag correction at release time. No tag created.

## Questions and risks

- Clean Windows install cannot be isolated solely by choosing another install folder: the existing installer owns shared per-user registration and shortcuts. Inspect before running.
- Maintain at least 20 GB free; initial available space 25.82 GB. No other agent artifacts may be removed.
- macOS/Linux artifact acceptance, real key provisioning, OS signing and release publication remain external gates.
