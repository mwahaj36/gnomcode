# Left to do before launch

Updated 2026-11-01 after PR #113. Owner: Wahaj. Keep release caveats here and in technical docs, separately from optimistic marketing. Record reviewer, date and evidence when closing a gate. Historical checks remain in [sessions](sessions/README.md).

## Completed since the previous update

- [PR #113](https://github.com/mwahaj36/spawnt/pull/113) corrected snapshot totals, limited published session fields and removed titles, profile and relay records from new desktop snapshots. It clarified account-sync, guardrail, search and deletion limits and bounded Git inspection output and runtime. Older snapshots and backups still need a retention/deletion decision.
- [CI run 41285486249](https://github.com/mwahaj36/spawnt/actions/runs/41285486249) passed Rust clippy and workspace tests on Windows, macOS and Linux, plus the Rust MSRV and dependency audit, web checks, npm shim tests and a sandboxed Electron test. Native release archives were not built in that run. This is automated test evidence, not packaged-app or physical-device acceptance.
- The shared model picker now uses supported native CLI metadata for Codex and Claude, and Gemini configuration and model-list metadata where available. Antigravity has no verified read-only catalog. Real authenticated provider coverage remains open.

- [Independent launch work](INDEPENDENT_WORK_2026-11-01.md) records the deletion/replay and retention fixes, conservative quota checks, endpoint inventory, adapter coverage, live-site checks and a synthetic Linux baseline. These checks do not close device, release, production RLS or independent security gates.

## Blocking gates

| Gate | What blocks it | How to close it | Required evidence |
| :--- | :--- | :--- | :--- |
| Remote security approval | Independent review and production approval remain open | Review ADRs 0019–0021 and 0025 and threat models independently. Exercise interception, cross-account discovery, replay, reconnect key reuse, durable approvals, concurrent revocation and authorization. Fix findings and obtain owner approval. | Review report, resolved findings, adversarial tests, dated approval |
| Physical-phone acceptance | Emulation does not test real keyboards, mobile suspension or storage | Test current desktop/site on real iOS Safari and Android Chrome: native account-access confirmation, automatic signed-in access across native apps, launch, first/follow-up prompt, history, folders, rotation, keyboard, reload/background, reconnect, account-wide host disable and native re-enable. | Device/browser versions, sanitized captures and results |
| Real model/provider handoff | Synthetic processes do not establish CLI compatibility or entitlement | Use authenticated installed supported CLIs. Launch, approve native trust, send a task, switch to an available model/provider, verify reported target model/reply, preserve drafts/history and test non-Git folders. Exercise quota/resume with eligible observed windows. | CLI versions, observed model IDs, sanitized outcomes |
| Metadata privacy contract | New snapshots still share project names, session identifiers, usage reports and quotas without end-to-end encryption; historical snapshots/backups may contain titles and relay metadata | Inventory remaining fields and destinations, including separate account settings. Approve field-level minimization and separate consent; verify existing snapshot replacement, backup retention/deletion and two-account RLS. Extend payload tests for the agreed contract. | Approved field inventory and disclosure, retention decision, payload regression and two-account access results |
| Cross-platform release | Automated three-OS CI passed, but packaged-app and real-agent acceptance across platforms and architectures remain open | Test approved builds on Windows, macOS Intel/Apple Silicon and Linux as advertised: PTY crash survival/reattach, env/path overrides, focus fallbacks, tray, autostart, shell and packaged startup. Document Wayland/WSL limits. | CI artifacts plus actual OS/architecture and packaged-build acceptance matrix; narrow launch scope if needed |
| Distribution | 0.9.0 artifacts are development/draft builds | Follow `cut-release`, approve legal inputs, build selected targets, verify hashes/notices and clean install/uninstall. Smoke-test npm optional dependencies/offline CLI. Publish approved assets and update manifest only after approval. | Verified hashes, install logs, published URLs, rollback test |
| Signing and OS trust | Windows setup unsigned; macOS signing/notarization unverified | Obtain approved signing credentials securely. Sign/test Windows and macOS as advertised, or explicitly accept and document unsigned distribution. | Signature checks, clean-machine behavior and owner decision |
| Legal and payments | Jurisdiction, commercial terms, prices/refunds and payout eligibility unresolved | Confirm legal identity/contact and jurisdiction with owner/legal reviewer; timezone is not legal jurisdiction. Reconcile EULA, privacy, terms and refund policy. Configure payments/tax/payouts and appropriate hosting plan before selling. | Approved texts, jurisdiction, prices and checkout/refund tests |

## Product closure

| Item | Remaining work | Completion evidence |
| :--- | :--- | :--- |
| Teams | Define roles, rollups, budgets, invitations and consent in SPEC/ADR. Existing schema/snapshot is not a complete Teams service. | Approved scope, authorization tests, team acceptance |
| Model catalogs | Validate supported Codex, Claude and Gemini native metadata against real CLI versions and accounts; Gemini configured status is not proof of authentication or entitlement. Antigravity lacks a verified read-only catalog; retain its native selector. | CLI versions, source contracts, real-account outcomes and fresh/stale/hidden-entry tests |
| Resume/automation | Validate native busy/approval handling and all quota windows. Design durable claims/recovery to prevent duplicate process launches across crashes and concurrent reconciliation. Do not promise restoration of arbitrary original prompt text. | Real agent acceptance and no-escalation regression results |
| Retention/deletion | Deletion and pruning now retain local hashed event-ID markers against stable-ID source replay; verify changed IDs, old backups, reset behavior and all-process upgrades. Define retention scheduling and backup policy; row deletion is not forensic erasure. | Deletion/retention tests and backup policy |
| Adapters | Replace missing-timestamp `Utc::now()` fallbacks with a designed, deterministic unknown-time policy that preserves history; validate current anonymized real logs, especially Antigravity/Aider/editor variants. Review handling of historical Aider activity that may contain raw commands and historical guessed model/cost records; the source of old guesses cannot reliably be reconstructed. | Golden fixtures, reviewed capability matrix and historical-data remediation decision |
| Performance | Extend the synthetic Linux debug baseline to release builds, real output, desktop and PTY processes under concurrency on each launch OS. | Reproducible measurements and accepted budgets |
| Network disclosure | Verify the recorded endpoint inventory against shipping builds and configured services, including whether the planned update check exists; repeat an offline audit. | Endpoint audit, offline run and accurate privacy page |

## Product discovery, not committed launch scope

The [product audit](APP_AUDIT_2026-11-01.md) identifies managed worktrees, integrated diff/PR review, checkpoints and coordinated agents as possible gaps. Validate demand and tradeoffs before selecting work. Record approved scope in SPEC and any architectural decision in an ADR; do not treat competitor parity as a completed or required launch feature.

## Website and operations

- Verify production canonical URLs, robots, sitemap, all guide/legal links and social preview after deployment. Disabled platform downloads must not become dead installer links.
- Validate JSON-LD with the Schema.org validator and inspect indexing after deployment. Never fabricate ratings, customers or production certification.
- Produce approved 1200 × 630 social artwork from its specification if the final asset is absent; verify the crop and readability.
- Approve privacy/terms before purchases. Governing jurisdiction requires owner/legal input; preserve the EULA's mandatory-law protections.
- Recheck vendor installation, pricing and quota sources at publication. Limits vary by plan and model.
- Separate account metadata consent from encrypted Remote consent; publish a working support contact.
- Confirm commercial hosting eligibility, relay capacity, abuse controls and recovery procedures without product analytics.

## Release decision

Close every blocking gate or record an explicit owner-approved scope reduction. Record exact commit/version/platforms, artifact hashes, security and legal approvals, production URL checks and rollback procedure. Marketing approval does not authorize Remote payments or publication of an unreviewed release.

On 2026-11-01, Wahaj explicitly approved merging PR #116 and publishing its matching site and Windows downloads as a **0.9.0 development rollout** while the remaining phone/security gates stay open. This is a scoped development exception, not v1.0, security certification, cross-platform acceptance or payment approval. Native source, artifact hashes and rollout verification are recorded in the [post-migration build session](sessions/2026-11-01-account-wide-native-build.md).
