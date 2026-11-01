# Complete account-wide Remote access

## Goal

Finish the account-wide scope of PR #116. A native app linked to an account must be accessible from every signed-in device without per-desktop browser approval. Show explicit native confirmation before linking and provide SQL for the owner to apply manually. Continue using ponytail.

## Outcomes

- Native password sign-in and browser-account transfer both show the same modal before sending credentials. It explains access from existing and future signed-in devices, available history and controls, the awake/online requirement, and how to disable access. Cancel submits nothing.
- Rust rejects native linking and enabling when `remote_access_confirmed: true` is omitted. Existing links default to no broader consent.
- After native consent, account-owned browser connection requests are authorized automatically on all consenting native apps. Removed the per-host browser approval actions and UI. Browser transport still pins enrollment keys and uses encrypted challenge-bound reconnect proof.
- Devices controls account-wide availability of a native app. Its `remote_enabled` switch can stop access for every signed-in client. Native sign-out clears local consent and suspends all connections; software updates and inactivity do not expire consent.
- Permission-enabling writes happen only in the explicit native consent operation. Registration uses ignore-duplicates and defaults off. Heartbeat writes never enable access, so account-side disable survives restart. Suspended connections retain protected credentials for later explicit re-enable.
- Added a standalone transactional migration and application instructions under `supabase/`. It supports fresh databases and the previous directory migration, can be rerun, preserves existing data, and leaves existing native apps disabled until the user confirms the broader permission.
- Updated SPEC, ADR 0025, design guidance and public instructions around the complete account authorization model. Account ownership and the owner-only directory are deliberately trusted: an account compromise can grant Remote access to a consenting native app. The relay remains an opaque E2EE forwarder.

## Problems and fixes

The previous implementation retained per-host browser approval and therefore did not meet the agreed scope. Replaced that boundary with explicit native account consent and automatic owner-scoped enrollment; no new approval system or production dependency was added.

The new automatic-connect browser regression exposed React Strict Mode cleanup leaving the manual-disconnect flag set. Reset it on effect setup; the signed-in phone now connects automatically on first visit, and deliberate disconnect still stays disconnected.

Generated Next.js route declarations were inconsistent during an initial typecheck. Regenerated them through `next typegen`; typecheck and the subsequent production build passed. No source workaround was added.

## Verification

- Observed the new missing-consent Rust test fail before implementation (400 versus required 428), then pass with the Rust guard. Observed the modal test fail before implementation, then pass.
- Full final Rust quality gate passed: rustfmt, workspace clippy with warnings denied, all workspace tests, cargo-deny and core dependency purity.
- Rust tests cover consent defaults, existing-link opt-in, the same browser across native apps, another signed-in client, consent persistence, native sign-out, real publication requests preserving cloud-side disable across restart, and explicit re-enable. Existing encrypted enrollment/restart/revocation tests pass.
- Native Playwright checks pass for password and browser-account linking, modal copy, mobile/desktop rendering, cancellation with no credential submission, explicit confirmation payloads, and native enable confirmation.
- Browser/local-relay Playwright checks pass for automatic initial access and opening a second native app from the same account with no approval. Reload, encrypted proof storage, bounded offline recovery, stopped retries, draft retention, no prompt replay, model selection and responsive controls continue passing.
- Standalone SQL passed on disposable PostgreSQL through PGlite both with and without the previous migration, including repeated application, two-owner RLS, anonymous denial, default-off access and persistent host disable.
- Web tests, typecheck and production Next.js build passed. Cross-platform review preserves `spawnt_home`, platform private-file helpers, blocking file I/O boundaries and loopback listeners; no new OS-specific code.

## Rollout boundary

The requested migration is provided for Wahaj to apply; no production SQL, site deployment or installer release was performed. Apply `supabase/migrations/20261101160844_account_wide_remote_access.sql` before rolling out the matching native/web build. Existing native links must confirm account access once. Physical-phone/macOS/Linux acceptance and independent Remote security approval remain the existing release gates. Keep the PR reviewable without silently deploying a broader permission model.
