> Withdrawn in v1.4.1 at the owner’s request. Retained as a historical decision record.

# 0005: Browser device pairing

Status: Accepted. Date: 2026-10-27.

## Context

A cloud workspace cannot receive a browser callback on the user’s Windows
loopback interface. Copying a desktop session shares rotating refresh tokens and
requires handling raw account credentials. Provider runtime identity does not
prove FleetHUD account ownership.

## Decision

Use the existing Supabase project and add two narrow PostgreSQL RPCs. The
collector generates a random request ID and 256-bit encryption key. Its ten-minute
approval link keeps secrets in the fragment, with an eight-character comparison
code. The browser creates an independent GitHub PKCE or password login using
isolated session storage, requires explicit confirmation, then uploads an
AES-256-GCM envelope bound to the request ID. Only authenticated users may approve.

The collector polls with a derived high-entropy capability. The server stores
only its hash, encrypted login and IV. It atomically consumes the envelope once;
a tombstone prevents repeat approval until expiry. Tables have RLS and no direct
client grants. An account may approve at most ten requests per five minutes;
expired records are removed on the next approval. No service-role secret or
unauthenticated access to plaintext credentials is needed.

The collector validates the decrypted login against Supabase before atomic,
owner-only persistence. Refresh, pairing and disconnect serialize file writes.
Windows uses user-only ACLs; Unix uses mode 0600. File locks prevent a second
daemon from changing an existing daemon’s metadata. Pairing starts or reuses the
daemon. No provider configuration or permissions are modified.

## Consequences

The additional migration and a web build from the same Supabase project are
required. OAuth reuses the existing /console redirect, which forwards device
callbacks to /pair without the main browser client consuming them. A successful
browser approval means the envelope was sent; the terminal confirms completion.
No browser device screen can authorize arbitrary machines without user approval.
Ephemeral workspaces require a new pairing after their data is destroyed.
