# Handoff fold: codex-release (PR #241)

Date: 2026-11-14. Source: running handoff log PR #241 (`docs/handoff/codex-release.md`), closed unmerged.

## Delivered (all merged)

- #243 lockstep 1.0.0 versions, cut-release bumper syncs Cargo.lock and desktop lock, CHANGELOG, release notes, npm approved-EULA byte checks.
- #245 build-pinned Rust update public key (no runtime bypass), offline owner-key signer, ADR 0059, key custody/rotation docs.
- #248 Windows 1.0.0 test candidate evidence.
- #250 version-derived downloads copy and real-artifact local manifest generator (Windows only, no invented hashes).
- #251 packaged-app smoke procedure.
- Follow-ups #259 to #262 (handoff original goal, held-approval alerts, remote activity timer, hosted open tool_start) and #263 (fail closed without update key).

## Decisions and evidence

- Release-note baseline: Wahaj explicitly approved commit `7cec4e6` (source tag v0.9.4 is missing locally and on origin). Notes cover `7cec4e6..496d740`, extended through 175f821 by follow-ups. No tag was created.
- EULA: Wahaj selected `docs/legal/EULA.txt` unchanged for installer and npm. `EULA_DRAFT.md` must not ship.
- Tags: source repo already has historical `v1.0.0` at `503aed81e55dd3474b440a2b262a7d8c3951d34f`. Plan: version stays 1.0.0; Claude renames the FleetHUD historical tag/release to `v1.0.0-fleethud` at publication. Binary-only repo has `v0.9.4`, no `v1.0.0`. Not touched here.
- Installed acceptance is run by Claude on Wahaj's machine (no disposable VM requirement).
- Retained 1.0.0 candidate (older composition 17267e5 plus overlays, not the final merged source): installer 180,371,968 bytes, SHA-256 `a8727e0d17bc66bf887fc5875da83272d9652c3bf7426dbbae846605a6ec4d94`, build-record.json in `fleethud-codex-release\artifacts\windows-1.0.0-candidate`. A later EULA-corrected rebuild superseded it in #248; always rebuild from final merged source.
- Candidate checks: Authenticode NotSigned, no embedded owner key, unsigned manifest. Not shipping acceptance.

## Unfinished (release gates, external)

- Rebuild from final merged source with Wahaj's reviewed public key; sign only through the owner-controlled offline procedure.
- Valid owner-key signing, installed update acceptance, OS signing.
- Physical phone Remote/approvals, installed PWA, real provider acceptance, macOS/Linux acceptance.
- Historical source v1.0.0 tag rename and missing v0.9.4 handling at release time.
- npm artifact readiness fails until binary/EULA/notices are staged; npm audit 12 desktop / 6 web moderate findings remain.

All tracked in `docs/ROADMAP_1.0.md` section G.
