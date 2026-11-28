import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, cpSync, rmSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import test from 'node:test';

const root = fileURLToPath(new URL('../../', import.meta.url));
test('manifest uses package version, actual installer hash and only Windows x64', () => {
  const fixture = mkdtempSync(path.join(tmpdir(), 'gnomcode-manifest-'));
  try {
    const script = 'scripts/release/prepare-update-manifest.mjs';
    mkdirSync(path.join(fixture, 'scripts/release'), { recursive: true });
    mkdirSync(path.join(fixture, 'web'), { recursive: true });
    cpSync(path.join(root, script), path.join(fixture, script));
    writeFileSync(path.join(fixture, 'web/package.json'), '{"version":"1.0.0"}');
    writeFileSync(path.join(fixture, 'web/branding.json'), '{"releasesRepository":"https://github.com/example/binaries","remoteRelayUrl":"wss://relay.example/relay"}');
    const bytes = Buffer.from('MZ-synthetic-reviewed-installer');
    const sha = createHash('sha256').update(bytes).digest('hex');
    writeFileSync(path.join(fixture, 'Gnomcode-Setup.exe'), bytes);
    writeFileSync(path.join(fixture, 'SHA256SUMS-v1.0.0.txt'), `${sha}  Gnomcode-Setup.exe\n`);
    const output = path.join(fixture, 'latest.json');
    const run = () => spawnSync(process.execPath, [path.join(fixture, script), fixture, output], { encoding: 'utf8' });
    const first = run();
    assert.equal(first.status, 0, first.stderr);
    const manifest = JSON.parse(readFileSync(output, 'utf8'));
    assert.equal(manifest.version, '1.0.0');
    assert.equal(manifest.remote_relay_url, 'wss://relay.example/relay');
    assert.deepEqual(Object.keys(manifest.platforms), ['win32-x64']);
    assert.equal(manifest.platforms['win32-x64'].url, 'https://github.com/example/binaries/releases/download/v1.0.0/Gnomcode-Setup.exe');
    assert.equal(manifest.platforms['win32-x64'].sha256, sha);
    assert.equal(manifest.platforms['win32-x64'].file_name, 'Gnomcode-Setup.exe');
    assert.equal(manifest.platforms['win32-x64'].signature, undefined);
    writeFileSync(path.join(fixture, 'Gnomcode-Setup.exe'), 'MZ-tampered');
    assert.notEqual(run().status, 0);
    assert.equal(JSON.parse(readFileSync(output, 'utf8')).platforms['win32-x64'].sha256, sha);
  } finally {
    rmSync(fixture, { recursive: true, force: true });
  }
});
