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

## Item 4 delivered - PR #250

https://github.com/mwahaj36/spawnt/pull/250

Version-derived Windows downloads/SEO/llms copy, branding redirects, checksum/latest.json allow-list, and local real-artifact Windows-only manifest generation wired after packaging checks. No invented hashes/signatures, macOS/Linux claims or publication. Generator red/green and tamper-preservation test pass. Dedicated web 127 tests/typecheck/build and Rust fmt/clippy/499 tests passed, repeated after rebase onto 3943dbb. 1.0.0 integration HTTP/browser and actual artifact manifest checks passed. Public v1.0.0 assets absent, as disclosed. Depends on #243/#245/#248 in review order. Downloads worktree/target/node_modules/.next removed immediately after PR; smoke gate using that target had finished first.

## Item 5 delivered - PR #251

https://github.com/mwahaj36/spawnt/pull/251

Final packaged-app smoke procedure for Claude, including exact verified CLI/help/server options, private loopback update manifest, native/protocol approvals, notifications, Remote, doctor and uninstall. All acceptance boxes remain pending. Documentation/source gate: fmt/clippy/499 tests and diff/UTF-8/LF checks passed, repeated on 3943dbb. Smoke worktree removed immediately after PR; its shared target had already been removed with the downloads worktree after all checks finished.

## Final verification

Across all five item branches: no forbidden console/components/check-script/Playwright paths changed; shared SPEC/roadmap/session-index/CHANGELOG diffs are additive only; U+00C2 diff count 0. Retained artifact byte lengths and SHA-256 rechecked against build-record.json. All item worktrees, target directories, node_modules and .next outputs removed. Only this log worktree remains until the final push, then it is removed too. Free disk exceeded 53 GB after cleanup, above the 20 GB floor. No other worker's files removed.

Version PR follow-ups extend release-note coverage through 175f821 from owner-approved 7cec4e6; missing source v0.9.4 tag is disclosed. Source version metadata implementation is unchanged by those documentation follow-ups. The retained build's exact older composition remains 17267e5 plus the recorded overlays; never relabel it as the final merged-source/signing build.

## Status at stop

All five requested preparation items are reviewable. No PR merged, installer uploaded, tag/release created, deployment setting changed, credentials entered or release key generated/read. Log PR: https://github.com/mwahaj36/spawnt/pull/241 - **never merge**.

Review/merge order for Claude (the worker must not merge):

1. https://github.com/mwahaj36/spawnt/pull/243 - lockstep versions, bumper regression, CHANGELOG/release notes.
2. https://github.com/mwahaj36/spawnt/pull/245 - build-pinned verification and offline signer/key handling.
3. https://github.com/mwahaj36/spawnt/pull/248 - local Windows package evidence and final retained hashes.
4. https://github.com/mwahaj36/spawnt/pull/250 - downloads routes/copy and real-artifact manifest preparation.
5. https://github.com/mwahaj36/spawnt/pull/251 - final packaged smoke procedure.

Resume: fetch latest origin/main. After each approved merge, rebase the next branch onto origin/main and preserve every additive SPEC/roadmap/session-index/CHANGELOG block when resolving shared-doc append conflicts. Do not drop another worker's content. Rebuild from the final merged source with Wahaj's reviewed public key, select the approved EULA, sign only through the owner-controlled procedure, and run #251 in a disposable Windows account/VM. Retest/fix any failures with a new focused PR. Only Claude review and Wahaj sign-off can authorize upload/tag/release/deployment.

Blocking questions/inputs for release:

- Which approved EULA should the installer/desktop bundle: existing root LICENSE or docs/legal/EULA.txt (or another owner-approved input)?
- How should the existing historical source v1.0.0 tag at 503aed81e55dd3474b440a2b262a7d8c3951d34f be handled? No retagging is authorized. Source v0.9.4 is also missing; notes use explicitly approved 7cec4e6.
- Provide a disposable Windows account/VM for safe installed launch/upgrade/update/uninstall acceptance. A separate install directory is insufficient.
- Owner supplies the actual public build key and private signing input at release time. Valid owner-key signing, installed update acceptance, OS signing, physical phones, real provider acceptance and macOS/Linux acceptance are not verified here.

Retained artifacts: C:\Users\Wahaj\Documents\GitHub\fleethud-codex-release\artifacts\windows-1.0.0-candidate. Final installer SHA-256: a8727e0d17bc66bf887fc5875da83272d9652c3bf7426dbbae846605a6ec4d94 (180,371,968 exact bytes). build-record.json and BUILD-RECORD.md record all hashes and source composition. Existing npm audit findings remain 12 desktop / 6 web moderate; unstaged npm distribution binaries/notices/EULA were not published or certified.

## New authorized work - in progress

Wahaj selected docs/legal/EULA.txt unchanged for installer/npm. EULA_DRAFT.md must not ship. Version remains 1.0.0; Claude renames FleetHUD historical tag/release to v1.0.0-fleethud at publication. Do not touch tags/releases. Installed acceptance is now pending release smoke by Claude on Wahaj's machine; no disposable VM requirement.

Active worktrees: version-decisions on #243 and windows-decisions on #248 under fleethud-codex-release. Both rebased onto origin/main. #243 has uncommitted npm approved-EULA byte equality checks/staging validation, failing-first then passing regression (npm suite 5 passed), release-steps notes and SPEC/session additions. #248 has uncommitted packager EULA source replacements and pending-release-smoke session addendum. Rust clippy passed for both; workspace tests still running in sessions 6491 and 34228, sharing windows-decisions/target. Logs: decisions-tests.log and version-decisions-tests.log in worker root. No decision updates have been force-pushed yet. Continue full re-gates (including web/desktop checks where required), commit with co-author, fetch/rebase and push --force-with-lease, update PR descriptions and this log. Existing retained installer predates approved EULA correction; do not certify its EULA or install it.

A branch/worktree fix/handoff-original-goal at handoff-goal is created from latest main, no code changes. Before eval saved to handoff-before.txt: 8/9 pass; long-session 47.5%, goals 0/2, missing java.time and leap days identical, files 1/2 missing src/calendar.kt. Ran real retained CLI evals handoff --no-save with isolated GNOMCODE_HOME. Root cause: daemon lib.rs reads only newest HANDOFF_CONTEXT_EVENTS=50 before continuation_prompt; handoff_eval.rs explicitly drains earlier events. continuation_prompt also consumes newest text first within 16k chars. Store conversation_chat(None) still caps at 1000/5000, so simply removing the 50-event cap is insufficient for arbitrarily long sessions. Need a real original-goal query/input and reserved bounded priority before recent events, retaining redaction. No pinned decisions mechanism found; asked Wahaj where pins are recorded or whether explicit decision text is intended. Question pending. Do not invent pin storage/semantics.

B investigation: automation::notify_approval_waiting_hook takes mutable Observations and a Session; it is dead_code. approvals::create_handler currently records a generic automation notice after successful hold. Need use the shared polling observations engine for dedupe/rate limits and wire protocol paths too; no implementation yet.

C investigation: remote.rs already calls room.activity separately for interrupt and includes approval in existing matches list. Missing continue_here, worktree_merge/discard/open_pr and approvals. Add without rewriting unrelated code; test actual timer behavior. No implementation yet.

D investigation: core presence OPEN_EXECUTION_LIMIT_MS=600000; store sessions.rs presence/state queries around lines 422 and 501 apply stale-open rules. Use real daemon host liveness input; do not change non-hosted behavior or UI. No implementation yet.

A-D still require separate branches/PRs, failing tests first, SPEC/ADR as appropriate, sessions/index/roadmap, full gates and before/after eval output for A. Never touch protected console files, api.rs root, platform or workflows. New work is ongoing; prior Status at stop describes only the completed original release-preparation batch.

## Owner decisions applied

#243 force-pushed after red/green EULA regression (npm 5 tests), fmt/clippy/499 Rust tests, desktop 40 tests/lint/typecheck/build and web 128 tests/typecheck/build. Installer/npm source is docs/legal/EULA.txt unchanged; draft stays unshipped. Release steps record Claude-only FleetHUD tag/release rename and unchanged 1.0.0. #248 force-pushed with actual corrected EULA installer rebuild, 79 browser checks and exact installer/desktop/ZIP EULA comparisons. New retained installer: 180,359,680 exact bytes, SHA-256 131864637ee466c2a156fc0b3d1812402622e61459599141cc69a1950a31164f. Earlier hashes superseded. Source composition/other hashes updated in artifact build-record.json. First packaging attempt found missing Electron dist licenses; installed official electron/install.js restored required inputs and full packaging then passed. No installer executed. Installed acceptance is pending release smoke by Claude on Wahaj's machine, not a disposable VM requirement. #251 smoke procedure also updated and force-pushed after fmt/clippy/499 tests. Earlier EULA/tag/VM questions are resolved by Wahaj's decisions. No tags/releases touched.

UTF-8 verification caught accidental existing SPEC character conversion; exact upstream text restored and shared-doc final changes remain additive. Use explicit UTF-8 for all Python reads/writes. Decision worktrees/build outputs cleaned; retained artifacts remain outside them.

A clarification resolved: no pins/decision heuristic. Preserve first retained user request and available session title, then newest events under budget; no new persistent copy or retention bypass. A implementation now passes all nine evals at 100% (long-session 47.5% before / 100% after), with no threshold changes. Added original-goal read independent of page cap, title lookup independent of session-list cap, protected first-message budget, redaction, deletion/5000+ event tests and legacy packet goal/title preservation. Full final gate running in exclusive handoff-goal target. Do not share target directories between differing worktree sources: the smoke gate overwrote remapped crate metadata; affected A crate artifacts were cleaned and A gate rerun. Before/after outputs in worker-root handoff-before.txt / handoff-after.txt. No A PR yet.

B branch fix/approval-live-notifications at approval-alerts: failing held-approval test proved structured push payload absent for Claude/Codex/Antigravity. Implementation uses shared Arc/Mutex notification engine for poll+live hints, background notification after successful hold so no hook delay, redacted Session metadata only. Targeted green test running, no B PR yet. C/D not implemented yet. Continue until all items have separate PRs and full gates; do not stop for routine progress.
