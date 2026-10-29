## 2026-10-27 — Withdraw hosted cloud integration, v1.4.1

- User requested removal of the cloud feature. The latest clarification explicitly preserves older desktop account sync and sign-in.
- Removed cloud runtime identity detection, collector command/startup, metadata usage-file ingestion, cloud session types, remote presence reads and native/web/tray presence presentation. Removed the new browser device pairing command, page and RPC client.
- Retained unrelated security/advisory, pricing/date, embedded-console, credential-permission, daemon-lock and update-integrity fixes. Restored existing manual desktop account pairing while retaining server-side login validation and credential protection; unsafe auth callback URLs stay removed.
- Stopped this workspace’s daemon and standalone collectors, deleted its remote presence row, revoked its independent Supabase login and deleted its credential file. Other desktop accounts were untouched.
- Retired SQL migration creates no new feature schema; the repeatable cleanup drops only hosted presence/pairing objects. Owner must run it in the SQL editor because no admin connection is available here.
- Added checks for retired local pairing endpoints returning 404 and inert historical rows being hidden without deleting data. Disposable PostgreSQL cleanup checks verify desktop sync and Auth survive.
- Updated the environment startup draft to remove cloud collection/pairing instructions; saving the draft does not publish it.
- Replacement native packages are v1.4.1, keeping Windows update ordering correct. Native Windows GUI and macOS hardware remain unavailable in this cloud environment.

- Verification: Rust gate (56 tests, Clippy, cargo-deny), MSRV, production web build and disposable PostgreSQL cleanup checks passed. Browser checks against the packaged daemon verify working local data/auth UI, preserved desktop snapshot reads, ignored legacy cloud rows, no presence fetches and retired pairing API/page returning 404. CLI rejects retired commands and a daemon in this actual cloud runtime creates no cloud-presence row. Live Supabase pairing/view/table probes now return 404; the older desktop sync endpoint still exists.

