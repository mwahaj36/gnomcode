// Verify retirement against the actual previously shipped schema in disposable PostgreSQL.
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { readFile } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import assert from 'node:assert/strict';
const require = createRequire(resolve(process.argv[2],'package.json'));
const { PGlite } = require('@electric-sql/pglite');
const db = new PGlite();
try {
  await db.exec(`CREATE ROLE authenticated; CREATE ROLE anon; CREATE SCHEMA auth;
    CREATE TABLE auth.users(id uuid PRIMARY KEY);
    CREATE FUNCTION auth.uid() RETURNS uuid LANGUAGE sql AS $$SELECT NULL::uuid$$;
    CREATE TABLE public.gnomcode_sync_state(user_id uuid,live_snapshot jsonb);
    INSERT INTO public.gnomcode_sync_state VALUES ('00000000-0000-0000-0000-000000000001','{"desktop":"preserved"}');`);
  for(const file of ['supabase_cloud_presence_v3.sql','supabase_device_pairing_v4.sql']) {
    const historical = execFileSync('git',['show',`4fd3d6f:crates/store/migrations/${file}`],{encoding:'utf8'});
    await db.exec(historical.replaceAll(/\b[a-z]+_(?=cloud_sessions|pairing_envelopes|pair_approve|pair_consume|cloud_heartbeat_time)/g, 'gnomcode_'));
  }
  const cleanup = await readFile(new URL('../crates/store/migrations/supabase_remove_cloud_sessions_v5.sql',import.meta.url),'utf8');
  await db.exec(cleanup);await db.exec(cleanup);
  const retired = await db.query(`SELECT relname FROM pg_class WHERE relname IN ('gnomcode_cloud_sessions','gnomcode_cloud_sessions_live','gnomcode_pairing_envelopes')`);
  assert.equal(retired.rows.length,0);
  const functions = await db.query(`SELECT proname FROM pg_proc WHERE proname IN ('gnomcode_pair_consume','gnomcode_pair_approve','gnomcode_cloud_heartbeat_time')`);
  assert.equal(functions.rows.length,0);
  assert.deepEqual((await db.query('SELECT live_snapshot FROM public.gnomcode_sync_state')).rows[0].live_snapshot,{desktop:'preserved'});
  assert.equal((await db.query("SELECT to_regclass('auth.users') AS users")).rows[0].users,'auth.users');
  console.log('PASS: retired schema and endpoints removed; desktop sync and Auth preserved; cleanup is repeatable');
} finally { await db.close(); }
