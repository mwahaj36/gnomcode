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

## Item 1 delivered - PR #243

https://github.com/mwahaj36/spawnt/pull/243

Lockstep 1.0.0, bumper lockfile fix, CHANGELOG and release notes. Gate before final rebase: fmt, clippy, 499 Rust tests and cargo-deny passed; desktop 40 tests plus lint/typecheck/build passed; web 133 tests plus typecheck/build passed. Version rendering and branding checks passed. npm artifact readiness fails for unstaged binary/EULA/notices, as disclosed in PR. npm audit: 12 desktop / 6 web moderate findings. origin/main advanced to bc347e9 during push; PR rebased onto it. Upstream changed console shell and web test commands; follow-up final-head gate still needed. Version tree including target/node_modules/.next removed with git worktree remove --force after clean status. Direct PowerShell generated-directory cleanup was rejected by automatic policy; Git-owned worktree cleanup succeeded. Free space recovered to 23.53 GB.

## Item 2 in progress

Branch feat/release-embedded-update-key. Public-key-only RFC 8032 regression compiled with a build key and run without a runtime key failed first (unsigned bytes accepted). After option_env! precedence fix it passes with runtime key absent, blank and replaced. Node signer rejects absent/conflicting inputs without modifying manifest or printing sentinel. No private key generated or committed. Full final-head gate pending.
