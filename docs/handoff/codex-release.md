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

## Item 2 delivered - PR #245

https://github.com/mwahaj36/spawnt/pull/245

Build-pinned Rust public key, no runtime bypass, offline owner-key signer, ADR 0059 and key custody/rotation docs. fmt/clippy and 499 workspace tests pass; embedded-key test separately passed under absent/blank/replaced runtime values. Node signer rejection test passes. No private key generated/read/committed. Valid owner-key signing and installed updates unverified. Signature covers installer bytes, not envelope metadata; old unpinned clients retain development behavior. Signing worktree and target removed immediately after PR.

## Item 3 in progress

Windows worktree based on 17267e5. Native release build uses a temporary version overlay from #243 (no duplicate version changes will be committed here). No signing key supplied: test candidate is unsigned/unpinned, not release-approved. User reports disk cleanup is running. Maintain >=20 GB free before dependencies/Electron packaging. Installer cannot be safely installed into a separate directory in this account because shared registration/shortcuts are affected; disposable account/VM required.

## Item 3 delivered - PR #248

https://github.com/mwahaj36/spawnt/pull/248

Built Windows 1.0.0 test candidate from 17267e5 plus #243 version metadata and #245 verifier code, with local downloads integration. Final installer: 180,371,968 exact bytes; SHA-256 a8727e0d17bc66bf887fc5875da83272d9652c3bf7426dbbae846605a6ec4d94. Earlier baseline-only hash is superseded. Artifacts/checksums/unsigned manifest and build-record.json retained outside worktrees at C:\Users\Wahaj\Documents\GitHub\fleethud-codex-release\artifacts\windows-1.0.0-candidate.

Full combined fmt/clippy/499 tests/cargo-deny passed; desktop 40 tests plus lint/typecheck/build and web 127 tests plus typecheck/build passed; 68 release-binary browser checks passed. Installer payload/options/notices/version and Explorer harnesses passed. Authenticode NotSigned, no embedded owner key and unsigned update manifest: not shipping acceptance. Later main changes need a final rebuild. No safe install/full desktop launch/uninstall ran because registration/shortcuts affect the existing per-user app; use a disposable account/VM. Windows worktree/target/node_modules/.next removed immediately after PR.

Blocking review question: packager uses root LICENSE as installer/desktop EULA, while docs/legal/EULA.txt also exists. Which approved EULA should ship? No legal text/selection changed.

Tag review: source already has historical v1.0.0 at 503aed81e55dd3474b440a2b262a7d8c3951d34f. Binary-only repo has v0.9.4 but no v1.0.0 release. No tags created/overwritten. Claude must resolve source tag policy at release time, alongside the missing source v0.9.4 tag.

## Items 4 and 5 prepared

Downloads source and local manifest generator passed dedicated web 127 tests/typecheck/build and Rust 499 tests; repeated after rebase onto 3943dbb. Version 1.0.0 integration HTTP/rendered version and desktop/mobile browser checks passed; actual local Windows manifest hashes match. Smoke document source/CLI-help checks and Rust 499-test gate passed, including after rebase. Separate PRs follow; installed/provider/phone acceptance remains pending.
