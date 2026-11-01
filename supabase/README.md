# Account-wide Remote migration

Run the entire [account-wide access migration](migrations/20261101160844_account_wide_remote_access.sql) in your project's Supabase SQL Editor. This is the file to apply; the earlier device-directory migration is not a prerequisite.

The migration is transactional and repeat-safe. It supports a fresh database or the earlier device directory, preserves existing data, adds `remote_enabled` with a default of `false`, and installs account-owner RLS and authenticated grants. It does not enable Remote on existing native links.

After applying SQL, use the matching native and web builds from PR #116. Each native app needs the account-access confirmation once. Every current or future device signed in to that account can then open its available sessions without phone pairing or per-desktop approval. The native app must remain awake and online. Devices can disable a native app across the account; native heartbeat updates do not undo this.

Optional read-only schema checks:

```sql
select table_name, column_name, column_default, is_nullable
from information_schema.columns
where table_schema = 'public'
  and table_name = 'spawnt_desktops'
  and column_name = 'remote_enabled';

select tablename, rowsecurity
from pg_tables
where schemaname = 'public'
  and tablename in ('spawnt_desktops', 'spawnt_device_requests');

select tablename, policyname, roles, qual, with_check
from pg_policies
where schemaname = 'public'
  and tablename in ('spawnt_desktops', 'spawnt_device_requests');
```

The files under `tests/` are for a disposable test database, not the production SQL Editor.
