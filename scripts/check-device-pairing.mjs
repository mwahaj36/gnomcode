// Real PostgreSQL authorization and one-time exchange checks, without live credentials.
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { readFile } from 'node:fs/promises';
import assert from 'node:assert/strict';
const require = createRequire(resolve(process.argv[2], 'package.json'));
const { PGlite } = require('@electric-sql/pglite');
const db = new PGlite();
try {
  await db.exec(`CREATE ROLE authenticated; CREATE ROLE anon; CREATE SCHEMA auth;
    CREATE TABLE auth.users(id uuid PRIMARY KEY);
    CREATE FUNCTION auth.uid() RETURNS uuid LANGUAGE sql STABLE AS $$ SELECT nullif(current_setting('request.jwt.claim.sub',true),'')::uuid $$;
    GRANT USAGE ON SCHEMA auth TO authenticated, anon;
    GRANT EXECUTE ON FUNCTION auth.uid() TO authenticated, anon;
    INSERT INTO auth.users VALUES ('00000000-0000-0000-0000-000000000001');`);
  const migration = await readFile(new URL('../crates/store/migrations/supabase_device_pairing_v4.sql', import.meta.url), 'utf8');
  await db.exec(migration); await db.exec(migration);
  const id = 'a'.repeat(32), token = 'b'.repeat(64);
  const hash = "encode(sha256(convert_to('" + token + "','UTF8')),'hex')";
  const consume = t => db.query(`SELECT public.fleethud_pair_consume('${id}','${t}') AS envelope`);
  await db.exec('SET ROLE anon');
  assert.equal((await consume(token)).rows[0].envelope, null);
  await assert.rejects(db.exec(`SELECT public.fleethud_pair_approve('${id}',${hash},'${'x'.repeat(24)}','${'i'.repeat(16)}')`), /permission denied/);
  await assert.rejects(db.exec('SELECT * FROM public.fleethud_pairing_envelopes'), /permission denied/);
  await db.exec(`RESET ROLE; SET ROLE authenticated; SET request.jwt.claim.sub = '00000000-0000-0000-0000-000000000001'`);
  const approve = () => db.exec(`SELECT public.fleethud_pair_approve('${id}',${hash},'${'x'.repeat(24)}','${'i'.repeat(16)}')`);
  await approve();
  await assert.rejects(db.exec('SELECT * FROM public.fleethud_pairing_envelopes'), /permission denied/);
  await db.exec('RESET ROLE; SET ROLE anon');
  assert.equal((await consume('c'.repeat(64))).rows[0].envelope, null);
  assert.equal((await consume(token)).rows[0].envelope.ciphertext, 'x'.repeat(24));
  assert.equal((await consume(token)).rows[0].envelope, null);
  await db.exec('RESET ROLE; SET ROLE authenticated');
  await assert.rejects(approve(), /duplicate key/);
  await db.exec(`RESET ROLE; UPDATE public.fleethud_pairing_envelopes SET expires_at = now() - interval '1 second'; SET ROLE anon`);
  assert.equal((await consume(token)).rows[0].envelope, null);
  console.log('PASS: repeatable migration, authenticated approval, table isolation, secret verification, single consumption and replay rejection');
} finally { await db.close(); }
