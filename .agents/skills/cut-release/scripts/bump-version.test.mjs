import assert from 'node:assert/strict';
import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import test from 'node:test';

const root = fileURLToPath(new URL('../../../../', import.meta.url));
test('version bump synchronizes desktop lock metadata and workspace lock packages', () => {
  const fixture = mkdtempSync(path.join(tmpdir(), 'gnomcode-version-test-'));
  try {
    for (const rel of ['.agents/skills/cut-release/scripts/bump-version.mjs', 'Cargo.toml', 'Cargo.lock', 'apps/desktop/package.json', 'apps/desktop/package-lock.json', 'npm/gnomcode/package.json', 'npm/platform', 'web/package.json']) {
      mkdirSync(path.dirname(path.join(fixture, rel)), { recursive: true });
      cpSync(path.join(root, rel), path.join(fixture, rel), { recursive: true });
    }
    const result = spawnSync(process.execPath, [path.join(fixture, '.agents/skills/cut-release/scripts/bump-version.mjs'), '1.0.0'], { encoding: 'utf8' });
    assert.equal(result.status, 0, result.stderr);
    const lock = JSON.parse(readFileSync(path.join(fixture, 'apps/desktop/package-lock.json'), 'utf8'));
    assert.equal(lock.version, '1.0.0');
    assert.equal(lock.packages[''].version, '1.0.0');
    const cargo = readFileSync(path.join(fixture, 'Cargo.lock'), 'utf8');
    const packages = [...cargo.matchAll(/name = "(gnomcode(?:-[^"]+)?)"\nversion = "([^"]+)"/g)];
    assert.ok(packages.length >= 8);
    for (const [, name, version] of packages) assert.equal(version, '1.0.0', name);
  } finally {
    rmSync(fixture, { recursive: true, force: true });
  }
});
