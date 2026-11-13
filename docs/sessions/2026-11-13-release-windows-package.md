# Windows 1.0.0 local packaging evidence

Date: 2026-11-13. Owner: codex-release. Test candidate only; no publication approval.

## Goals and outcomes

Build the existing Windows package locally, verify the native/embedded payload and retain exact artifacts for Claude. No installer was installed or uninstalled. No release key was generated or read. No tags, uploads, GitHub releases or deployment settings changed.

Retained directory: `C:\Users\Wahaj\Documents\GitHub\fleethud-codex-release\artifacts\windows-1.0.0-candidate`. It contains the installer, ZIP, CLI, notices, LICENSE, versioned checksums, unsigned latest.json and build-record.json. It is outside the worktree so cleanup does not remove review artifacts.

| Artifact | Bytes (exact) | SHA-256 (exact) |
| --- | ---: | --- |
| Gnomcode-Setup.exe | 180,371,968 | `a8727e0d17bc66bf887fc5875da83272d9652c3bf7426dbbae846605a6ec4d94` |
| Gnomcode-v1.0.0-windows-x64.zip | 178,832,701 | `6c216042e911489ba39d60f7c1f08cec880fdc8dfb5751d61667fcc525d63265` |
| gnomcode.exe | 14,147,072 | `d31a95b9fcbe4c9fd913f385ce666b8d54dee28ca4dd19c4b1fcd27e5cc1aa94` |

These replace the earlier baseline-only candidate; use these hashes, not the initial build's hashes.

## Source composition

Native/desktop source baseline: `17267e59f7e94f6292ba5c3d56d67d25a7b8b4eb`. Temporary version overlay from PR #243 (`a130675dd17f3598f86e6a5c2fd141fe08d3ce3a`), and the update verifier source from PR #245 (`92a5172949550d4a9d973a58fdc6771835f9fffd`). The in-progress downloads preparation source was overlaid for web/manifest integration only; no duplicate source changes are committed in this evidence PR. All overlays and generated tracked files were restored after verification. Later main changes require a final rebuild after review/merge.

The candidate includes the new build-key verifier code but has **no embedded owner key**. Authenticode reports **NotSigned**. Its manifest is intentionally unsigned. This is not an approved shipping build. Rebuild with Wahaj's reviewed public key and sign the final artifacts/manifest through the approved procedure before publication. Never use the RFC test key.

## Verification

- CARGO_INCREMENTAL=0, dev/test debug symbols disabled: cargo fmt --all, workspace clippy --all-targets with -D warnings and 499 workspace tests passed for the combined source. One build-key-specific regression is ignored in a default unpinned build and was separately verified in PR #245.
- cargo deny check passed (duplicate-dependency warnings only); lockfile differences were local version metadata.
- Desktop npm ci/lint/test/typecheck/build passed, 40 tests. Web npm ci/test/typecheck/build passed, 127 tests. npm audit: 12 desktop and 6 web moderate findings; existing React hook/module warnings remain.
- Existing package-release.ps1 completed against the real 1.0.0 native binary. Browser suite: 68 passed. Installer checker verified version, exact embedded executable, notices and launcher resources. Options checker verified consent/options and preservation behavior in isolation. Explorer launcher checks passed. Electron package and final Next production build passed.
- Native CLI --version returned gnomcode 1.0.0. Doctor/integrate/protocol-approvals help was exercised without configuration mutation.
- Generated Windows-only manifest hash matches the actual installer and versioned SHA256SUMS. Local downloads redirects/rendered version and browser viewport checks passed against the source integration; public release URLs are not available yet.
- git diff --check passed; U+00C2 diff count 0. UTF-8 without BOM, LF for this record.

## Problems, decisions and unverified acceptance

The host initially approached the mandatory 20 GB free floor. Direct generated-directory cleanup was blocked by automatic policy; Git-owned worktree cleanup succeeded. Wahaj then cleared space, allowing full gates and rebuilds. No other worker's files were deleted.

A separate destination directory does not isolate this installer: it owns per-user uninstall registration, Start/Desktop shortcuts, PATH/autostart, Explorer integration and app identity. Running install/uninstall or the packaged desktop here could affect Wahaj's existing installation/daemon/shortcut. Clean installation and full desktop launch were therefore not attempted. A disposable account/VM is required. The packaged executable was inspected by the existing payload/options harnesses; that is not installed-app acceptance.

The existing packager embeds root LICENSE as the installer license and copies it to desktop EULA.txt. Root LICENSE references legacy tiers, while docs/legal/EULA.txt also exists. **Blocking review question: which approved EULA must the installer and desktop bundle?** No legal text or packaging selection was changed without that decision.

Owner-key signing, OS trust, clean install/upgrade/uninstall, final merged-source rebuild, real provider/phone acceptance and publication remain unverified. macOS/Linux artifacts are not claimed.

## Wahaj decisions - follow-up

Ship docs/legal/EULA.txt as-is in the installer and desktop EULA resource; EULA_DRAFT.md is not shipped. Packaging selection now uses that approved source. The previously retained installer predates this correction and must be rebuilt. Installed-app acceptance is **pending release smoke**: Claude will run it on Wahaj's machine at release; a disposable VM is no longer required. No installer was run by this worker. Version stays 1.0.0; Claude owns the historical FleetHUD tag/release rename at publication.

## Corrected approved-EULA candidate

This build supersedes every earlier artifact hash in this record. Source baseline: d76d3f2bf0c34c5a00a862efccb08a4ba6106f3f, with #243 version metadata and #245 verifier overlay. Installer/desktop/archive legal bytes now match docs/legal/EULA.txt unchanged, SHA-256 999d33491c59453474f8b7c1d57e809a24aca35a430656ee975e611910ac0a4e. EULA_DRAFT.md is unshipped.

| Artifact | Bytes (exact) | SHA-256 (exact) |
| --- | ---: | --- |
| Gnomcode-Setup.exe | 180,359,680 | `131864637ee466c2a156fc0b3d1812402622e61459599141cc69a1950a31164f` |
| Gnomcode-v1.0.0-windows-x64.zip | 178,833,976 | `9d7d67ab3a9309a3ed342090e579e9d61bdc5923e96263e7c08ec3ed945f826a` |
| gnomcode.exe | 14,133,760 | `d465ee73a5d5352adacd427636192c00bb69bfe29f5d89201fb9e1abb49a4bfa` |

The archive regression failed against root LICENSE, then passed after the source correction. The strengthened installer resource checker failed against the old installer with Embedded payload mismatch: LICENSE, then passed in the fresh package build. Direct desktop ZIP/resources EULA and release ZIP/LICENSE comparisons passed byte-for-byte. Fresh build: 79 browser tests against real native release binary, Electron package and C# payload/options/notices checks, final web build all passed. Rust fmt/clippy/workspace tests passed before the metadata overlay; release version remains 1.0.0. npm ci did not materialize Electron dist license files on the first attempt; running the installed official electron/install.js restored them, and the complete packaging workflow then passed. No arbitrary license substitution or skipped packaging check.

Installed launch/upgrade/uninstall acceptance: **pending release smoke** by Claude on Wahaj's machine, as explicitly decided. No disposable VM is required by the current release decision, and this worker has not run an installer. OS signing and owner-key signing remain pending; current candidate is unsigned/unpinned. Claude owns renaming FleetHUD v1.0.0 at 503aed8 to v1.0.0-fleethud at publication; no tag or release touched here.
