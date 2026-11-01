# Account-wide Remote: post-migration native build

## Goal and outcome

Wahaj reported the account-wide migration applied. Prepare the matching Windows development installer without overwriting existing downloads or silently publishing a broader Remote permission model.

An anonymous live PostgREST probe of `spawnt_desktops` returned HTTP 401 / PostgreSQL `42501`, permission denied. This confirms the table is present and anonymous access is blocked; it is not authenticated owner/two-account acceptance.

## Problems and fixes

Packaging exposed a stale native-navigation test helper: it extracted the browser-account link function without supplying the new consent callback. Updated the helper to exercise cancelled consent, the explicit confirmation flag, and concurrent submission suppression.

The extended cancellation test then exposed a real UI issue: cancelling consent left the account's Connecting status visible. Observed the new assertion fail, cleared that status on cancellation for both native linking paths, and reran it successfully. No account or session data was changed.

## Verification and artifacts

- The full Rust quality gate passed again, including workspace tests, clippy, format, cargo-deny and core purity. Web tests and native console/navigation checks passed.
- The release runbook completed in the isolated `.tmp/account-wide-package` checkout: optimized Windows executable, notices, compiled setup, payload/version/resource checks, installer-options/uninstall-preservation checks, Explorer launcher checks, ZIP/checksums and production web build. The web build retains non-failing hook dependency warnings.
- A final refresh and byte-level check confirmed that the packaged native executable contains the cancellation fix. SHA-256 comparisons confirmed the original root download artifacts were untouched.
- Runtime source corresponds to `9d14003` plus `f79df12`. Private outputs are under `.tmp/account-wide-package/web/public/downloads/`:
  - `Spawnt-Setup.exe`: `93f3bdd582764653435e407ae7bbbcdeb187f7416b50942800db0226b6901567`
  - `Spawnt-v0.9.0-windows-x64.zip`: `e82310a17c6db898da76a6338bddffc194b44d0e35a10e817642998a12cb2943`
  - `spawnt.exe`: `6290e3760c3316a4fc661cb9f9b32fafb4609541f06d3a851d8f19e6d5095cd6`

## Rollout boundary

No production site, download channel, tag or release was published, and PR #116 remains a draft. The installer is an unsigned 0.9.0 development build, not evidence of physical-phone, clean-install or cross-platform acceptance. Production publication still requires the owner decision specified by the launch checklist; the reported migration alone does not close independent Remote security and device gates.

## Owner-approved development rollout

After reviewing the matching installer and the request to merge PR #116 and publish the matching site/downloads as a development rollout, Wahaj answered "yes" on 2026-11-01. This explicitly authorizes that scoped rollout with the phone/security gates remaining open. Preserve the distinction from a completed public launch or payment approval. Publication verification follows this authorization; no update-manifest channel or release tag is included.

## Publication verification

- Staged and then promoted deployment `dpl_36rVptQk2REdDMqPUtC3Gp6syASj` (`spawnt-e5i65akfa-wahajs-projects-81d8234d.vercel.app`) to the existing public project domain on 2026-11-01.
- Public `https://spawntv.vercel.app/console`, `/remote`, `/downloads` and `/docs/remote-access` returned successfully. The public Remote guide contains the new account-wide, no-per-desktop-approval instructions.
- Downloaded the public setup, ZIP and CLI from `/downloads/` and verified every SHA-256 against the private verified build listed above. The deployed downloads are the matching build, not stale root artifacts.
- Source was verified to match the approved runtime code before deployment; existing original root download files remain untouched. No tag, public GitHub release or update-manifest change was made.
- Previous public deployment for rollback: `dpl_CJjLKLDFCNWBmJv19GKkPgowLmZi` (`spawnt-mbds4n9pv-wahajs-projects-81d8234d.vercel.app`). `vercel promote dpl_CJjLKLDFCNWBmJv19GKkPgowLmZi` restores its site/downloads if needed. Keep native and web versions matched; do not drop the additive migration or erase device/history data. A live rollback and physical-phone login were not exercised.
