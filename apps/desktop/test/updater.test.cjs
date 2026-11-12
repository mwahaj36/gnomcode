const test = require('node:test');
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const { parseUpdateStatus, verifyStagedInstaller, launchVerifiedInstaller } = require('../dist/updater');

test('verifyStagedInstaller accepts matching sha256 inside updates root and rejects tampering or traversal', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'gnomcode-updater-'));
  try {
    const versionDir = path.join(root, 'v0.9.5');
    fs.mkdirSync(versionDir, { recursive: true });
    const installer = path.join(versionDir, 'Gnomcode-Setup.exe');
    const bytes = Buffer.from('MZ-test-installer-bytes');
    fs.writeFileSync(installer, bytes);
    const sha256 = crypto.createHash('sha256').update(bytes).digest('hex');

    const status = parseUpdateStatus({
      state: 'ready',
      available_version: '0.9.5',
      downloaded_path: installer,
      sha256,
      auto_update_enabled: true,
    });
    assert.ok(status);
    assert.equal(verifyStagedInstaller(status, root), fs.realpathSync(installer));

    // Tampered hash -> fail open (null).
    assert.equal(verifyStagedInstaller({ ...status, sha256: '0'.repeat(64) }, root), null);

    // Path outside updatesRoot -> fail open (null).
    const outside = path.join(os.tmpdir(), 'outside-installer.exe');
    fs.writeFileSync(outside, bytes);
    assert.equal(verifyStagedInstaller({ ...status, downloaded_path: outside }, root), null);
    fs.unlinkSync(outside);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test('launchVerifiedInstaller spawns detached once and fails open on spawn errors', () => {
  const calls = [];
  const ok = launchVerifiedInstaller('C:\\Users\\test\\updates\\Gnomcode-Setup.exe', (cmd, args, opts) => {
    calls.push({ cmd, args, opts });
    return { unref() {} };
  });
  assert.equal(ok, true);
  assert.equal(calls.length, 1);
  assert.equal(calls[0].cmd, 'C:\\Users\\test\\updates\\Gnomcode-Setup.exe');
  assert.equal(calls[0].opts.detached, true);

  const failed = launchVerifiedInstaller('C:\\bad\\Gnomcode-Setup.exe', () => {
    throw new Error('ENOENT');
  });
  assert.equal(failed, false);
});
