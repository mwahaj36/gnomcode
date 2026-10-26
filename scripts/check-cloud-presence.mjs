// Run against embedded Postgres in a disposable database; no live credentials used.
// Install @electric-sql/pglite into a temporary npm prefix and pass that directory.
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { readFile } from 'node:fs/promises';
if (!process.argv[2]) throw new Error('Usage: node scripts/check-cloud-presence.mjs <temporary-npm-prefix>');
const require = createRequire(resolve(process.argv[2], 'package.json'));
const { PGlite } = require('@electric-sql/pglite');
const db = new PGlite();
try {
  await db.exec(`CREATE ROLE authenticated; CREATE ROLE anon; CREATE SCHEMA auth;
    CREATE TABLE auth.users(id uuid PRIMARY KEY);
    CREATE FUNCTION auth.uid() RETURNS uuid LANGUAGE sql STABLE AS $$ SELECT nullif(current_setting('request.jwt.claim.sub', true), '')::uuid $$;
    GRANT USAGE ON SCHEMA auth TO authenticated, anon;
    GRANT EXECUTE ON FUNCTION auth.uid() TO authenticated, anon;
    INSERT INTO auth.users VALUES ('00000000-0000-0000-0000-000000000001'), ('00000000-0000-0000-0000-000000000002');`);
  const root = new URL('../crates/store/migrations/', import.meta.url);
  const migration = await readFile(new URL('supabase_cloud_presence_v3.sql', root), 'utf8');
  await db.exec(migration);
  await db.exec(migration);
  await db.exec(await readFile(new URL('tests/cloud_presence.sql', root), 'utf8'));
  console.log('PASS: repeatable migration, account isolation, metadata validation, server timestamps, measured usage and heartbeat expiry');
} finally {
  await db.close();
}
