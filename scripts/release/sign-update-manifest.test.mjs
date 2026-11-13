import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, writeFileSync, rmSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const script = new URL('./sign-update-manifest.mjs', import.meta.url);
test('signer refuses absent or conflicting key inputs without changing manifest', () => {
  const dir = mkdtempSync(path.join(os.tmpdir(), 'gnomcode-sign-test-'));
  try {
    const manifest = path.join(dir, 'latest.json');
    const original = '{"version":"1.0.0","platforms":{}}\n';
    writeFileSync(manifest, original);
    const env = { ...process.env };
    delete env.GNOMCODE_UPDATE_PRIVATE_KEY;
    delete env.GNOMCODE_UPDATE_PRIVATE_KEY_FILE;
    const run = extra => spawnSync(process.execPath, [fileURLToPath(script), manifest, dir], { env: { ...env, ...extra }, encoding: 'utf8' });
    const absent = run({});
    assert.equal(absent.status, 1);
    assert.match(absent.stderr, /Provide exactly one private-key source/);
    const conflict = run({ GNOMCODE_UPDATE_PRIVATE_KEY: 'SECRET-MUST-NOT-PRINT', GNOMCODE_UPDATE_PRIVATE_KEY_FILE: 'missing.pem' });
    assert.equal(conflict.status, 1);
    assert.doesNotMatch(conflict.stderr, /SECRET-MUST-NOT-PRINT/);
    assert.equal(readFileSync(manifest, 'utf8'), original);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
