# Left to do before launch

Updated 2026-10-29. Owner: Wahaj. Keep release caveats here and in technical docs, separately from optimistic marketing. Record reviewer, date and evidence when closing a gate. Historical checks remain in [sessions](sessions/README.md).

## Blocking gates

| Gate | What blocks it | How to close it | Required evidence |
| :--- | :--- | :--- | :--- |
| Remote security approval | Independent review and production approval remain open | Review ADRs 0019–0021 and threat models independently. Exercise interception, cross-account discovery, replay, reconnect key reuse, expiry, concurrent revocation and authorization. Fix findings and obtain owner approval. | Review report, resolved findings, adversarial tests, dated approval |
| Physical-phone acceptance | Emulation does not test real keyboards, mobile suspension or storage | Test current desktop/site on real iOS Safari and Android Chrome: pair, launch, first/follow-up prompt, page history, folder selection, rotation, keyboard, reload/background, reconnect, revoke and re-pair. | Device/browser versions, sanitized captures and results |
| Real model/provider handoff | Synthetic processes do not establish CLI compatibility or entitlement | Use authenticated installed supported CLIs. Launch, approve native trust, send a task, switch to an available model/provider, verify reported target model/reply, preserve drafts/history and test non-Git folders. Exercise quota/resume with eligible observed windows. | CLI versions, observed model IDs, sanitized outcomes |
| Metadata privacy contract | `sync::build_remote_snapshot` serializes sessions, profiles, reports and relay records, not only hashed rollups | Inventory actual fields, including project names, stored titles, device labels and relay `cwd`. Decide per-field minimization/consent with owner; implement a Rust allowlist and serialized-payload tests. Test RLS with two accounts. Do not silently delete existing features. | Field inventory, approved disclosure, payload regression and two-account access results |
| Cross-platform release | Latest local acceptance is Windows-only | Run gates on macOS Intel/Apple Silicon and Linux plus Windows. Test PTY crash survival/reattach, env/path overrides, focus fallbacks, tray, autostart, shell and packaged startup. Document Wayland/WSL limits. | CI artifacts and actual OS matrix; narrow launch scope if needed |
| Distribution | 0.9.0 artifacts are development/draft builds | Follow `cut-release`, approve legal inputs, build selected targets, verify hashes/notices and clean install/uninstall. Smoke-test npm optional dependencies/offline CLI. Publish approved assets and update manifest only after approval. | Verified hashes, install logs, published URLs, rollback test |
| Signing and OS trust | Windows setup unsigned; macOS signing/notarization unverified | Obtain approved signing credentials securely. Sign/test Windows and macOS as advertised, or explicitly accept and document unsigned distribution. | Signature checks, clean-machine behavior and owner decision |
| Legal and payments | Jurisdiction, commercial terms, prices/refunds and payout eligibility unresolved | Confirm legal identity/contact and jurisdiction with owner/legal reviewer; timezone is not legal jurisdiction. Reconcile EULA, privacy, terms and refund policy. Configure payments/tax/payouts and appropriate hosting plan before selling. | Approved texts, jurisdiction, prices and checkout/refund tests |

## Product closure

| Item | Remaining work | Completion evidence |
| :--- | :--- | :--- |
| Teams | Define roles, rollups, budgets, invitations and consent in SPEC/ADR. Existing schema/snapshot is not a complete Teams service. | Approved scope, authorization tests, team acceptance |
| Model catalogs | Codex fresh local cache exists; Claude/Gemini discovery needs reliable official local sources. Retain native selectors. | Source contracts and fresh/stale/hidden-entry tests |
| Resume/automation | Verify native busy/approval handling, reset timing, restart and duplicate prevention. Do not promise restoration of arbitrary original prompt text. | Real agent acceptance and no-escalation regression results |
| Retention/deletion | Confirm actual retention configuration and scheduling; explain SQLite WAL and backups. Row deletion is not guaranteed forensic erasure. | Deletion/retention tests and backup policy |
| Adapters | Validate current anonymized real logs, especially Antigravity/Aider/editor variants; publish capability differences. | Golden fixtures and capability matrix |
| Performance | Measure all daemon/desktop/PTY processes at idle and under concurrent sessions per launch OS. | Reproducible measurements and accepted budgets |
| Network disclosure | Reconcile `doctor --network` with update, pricing, account auth/sync, relay, license and optional provider operations. | Endpoint audit, offline run and accurate privacy page |

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
