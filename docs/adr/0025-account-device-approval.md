# ADR 0025: Account-wide Remote access with native consent

Status: Accepted; revised 2026-11-01 to cover the complete account-wide scope.

## Decision

Linking a native app grants Remote access to every device authenticated to the same Spawnt account. The native app presents a confirmation modal before password sign-in or browser-account transfer. It explains available conversation history and supported controls, existing and future signed-in devices, the online-host requirement, and how to disable access. Cancel performs no credential submission or linking. Native APIs reject omitted consent before any authentication or storage operation.

There is no OTP, phone approval or per-desktop browser approval. Each linked native app becomes discoverable to authenticated account devices. Browsers automatically request an encrypted connection through owner-scoped directory rows. The native app accepts account-owned requests only when its persisted consent and directory availability are both enabled. Enrollment keys are pinned to connections, reconnect remains challenge-bound, and each connection has independent encryption and proof credentials.

The account and its owner-only directory are the authorization authority. A compromised account or directory can grant access to a consenting native app; independent physical device verification is deliberately removed. End-to-end encryption still protects traffic from the forwarding relay, but does not make the authorization service untrusted. This is the tradeoff explicitly described by the native consent modal.

Native consent defaults off for existing links. Account-side `remote_enabled` defaults off and can disable one native app across all signed-in devices. Heartbeat upserts use ignore-duplicates; ordinary heartbeats never write this permission. Revocation survives restart. Native sign-out clears consent and suspends connections; enabling again needs the native modal. Cloud disable is enforced on the next successful directory poll. Offline desktops cannot be controlled, and chat is not queued or synced in plaintext.

Consent and connection records remain under owner-only `SPAWNT_HOME`, outside installation directories. Software updates and inactivity do not expire them. Browser sign-out removes local connection credentials; an account that is signed in again can establish fresh connections without further phone approvals. Independent Remote security review and physical-phone/platform acceptance remain release gates.

## Migration and rollout

The standalone, transactional SQL migration `supabase/migrations/20261101160844_account_wide_remote_access.sql` creates missing directory tables, adds the default-off availability field, and installs owner-only RLS and explicit grants. It works with or without the earlier directory migration and is repeat-safe. Apply it before publishing the matching native/web build. Applying SQL alone never opts existing native links into broader access.

## Consequences

Supersedes temporary-code discovery in ADR 0020, inactivity expiry in ADR 0021, and this PR's earlier per-host browser approval design. The account authorization boundary is explicit, while conversation content stays on its native host and travels over E2EE. Metadata reporting remains separate and retains its existing consent disclosures.
