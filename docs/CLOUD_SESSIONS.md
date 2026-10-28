# Hosted cloud sessions removed

Hosted/cloud-session tracking and its browser device-pairing flow were withdrawn
in v1.4.1 at the owner’s request. The CLI no longer offers `pair` or
`cloud-session`; provider identities, collectors, presence heartbeats, usage-file
imports and native/web cloud-presence views have been removed.

The older desktop account sign-in and opt-in desktop summary sync remain.
The local console loading fix, pricing fixes, private credential handling and
verified update installers remain. Existing desktop account data is preserved.

Upgrade native clients to v1.4.1 to remove the feature from installed copies.
The v1.3.0 and v1.4.0 release history does not change already-downloaded binaries.
Old cloud-presence rows in a local database are ignored without deleting history.

The current workspace’s separate collector was stopped, its remote presence row
removed, its login revoked and its credential file deleted. No Supabase admin
credential is available here. To remove the retired backend tables and functions,
run `crates/store/migrations/supabase_remove_cloud_sessions_v5.sql` in the same
project’s SQL editor. This removes cloud-session/pairing data only and preserves
`spawnt_sync_state`, desktop account sync and Supabase Auth.
