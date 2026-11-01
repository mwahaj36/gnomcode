# Account device approval and reconnect recovery

## Goals

Remove both Remote pairing-code exchanges, discover account-linked desktops, approve/revoke browsers from Devices, and investigate reported reconnect failures after desktop updates. Apply ponytail throughout and reuse the encrypted transport.

## Outcomes and decisions

- Account-linked desktops register independently; account discovery no longer overwrites a single temporary offer.
- Native account sign-in is the primary account connection. Existing browser-account transfer remains an advanced fallback for provider accounts; only required account fields are copied.
- A browser generates a nonexportable enrollment key and requests access. The selected desktop or one of its approved browsers approves that exact key with one action. Code-entry UI, code-confirmation handlers and enrollment endpoints are removed, without an OTP fallback.
- Each browser/desktop connection has an independent room and protected trust record. Cloud approval labels alone cannot authorize control. Approval remains scoped to the selected host; account-wide trust delegation is not implemented or claimed.
- Approval persists outside installation directories until revoked or disconnected. Inactivity no longer expires it. Existing server-side remembered trust remains readable; browsers with legacy unscoped credentials need a new code-free approval.
- Reconnect rediscovery checks account identity and pinned desktop identity. Retries stop after a bounded timeout; drafts and history remain visible with disabled sending and explicit recovery. No unacknowledged prompt is replayed.
- Sign-out revokes persisted approval even before the first directory poll. Account switching clears browser credentials for the prior owner. Local revocation is immediate; cloud revocation is applied when the desktop next reaches the directory.

## Investigation and root causes

The old policy explicitly expired trust after seven inactive days. The browser could retry an unreachable saved relay room indefinitely. Desktop trust was already stored under `SPAWNT_HOME`, outside installation folders; no evidence established that the updater itself deleted it. The reported physical-phone failure was not directly reproduced on the user's phone.

The new directory needs its migration before the new web/native flow is rolled out. Keep the PR unmerged until coordinated database/web/native rollout is arranged; the current release checklist still requires independent Remote security review and physical-phone acceptance. No production migration, deployment or installer release is included here.

## Verification

- Full `rust-quality-gate/scripts/check.ps1`: rustfmt, workspace clippy with warnings denied, workspace tests, cargo-deny and core purity passed.
- Rust encrypted integration: approved-key enrollment without codes, encrypted session/folder access, reconnect, daemon restart, replay-bound proof and concurrent revocation passed. An unapproved enrollment key receives no handshake/credential.
- Inactivity regression was observed failing before the fix. The key-binding regression was also checked by temporarily disabling the binding, observing the expected failure and restoring it before the final green gate.
- Native Playwright flow: sign-in submission, secret-field clearing, approval submission with the selected public key, existing console behavior and responsive layout passed.
- Browser/local-relay Playwright flow: multiple-desktop discovery, real browser key generation/request, automatic connection after approval, nonexportable proof storage, reload, background/stalled-connection recovery, timeout exhaustion, retry stopping, draft retention, no prompt replay, model controls and phone layout passed.
- Actual PostgreSQL RLS execution through disposable PGlite: two-account read/write isolation, anonymous denial, ownership reassignment denial, owner revocation, independent desktop rows and deletion cascade passed. Checked-in SQL is runnable against disposable Supabase/Postgres; hosted Supabase acceptance remains outstanding.
- Web typecheck, all web tests and production Next.js build passed.
- Cross-platform review: paths use `spawnt_home`/`PathBuf`; private records use the existing atomic owner-only writer through blocking tasks; no new platform-specific code, permission escalation or listeners. macOS/Linux and physical iOS/Android acceptance were not run.
