const test = require('node:test');
const assert = require('node:assert/strict');
const { spawnSync } = require('node:child_process');
const path = require('node:path');

test('missing platform package explains how to recover', () => {
  const result = spawnSync(process.execPath, [path.join(__dirname, '../bin/gnomcode.js'), '--version'], { encoding: 'utf8' });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /optional|optionalDependencies/i);
  assert.match(result.stderr, /install/i);
});

test('runs the installed platform binary and forwards arguments and exit status', () => {
  const fs = require('node:fs');
  const os = require('node:os');
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'gnomcode-shim-'));
  try {
    const report = process.platform === 'linux' ? process.report.getReport().header : null;
    const suffix = `${process.platform}-${process.arch}${report && !report.glibcVersionRuntime ? '-musl' : ''}`;
    const pkg = path.join(dir, '@gnomcode', `cli-${suffix}`);
    fs.mkdirSync(path.join(pkg, 'bin'), { recursive: true });
    fs.writeFileSync(path.join(pkg, 'package.json'), '{}');
    fs.copyFileSync(process.execPath, path.join(pkg, 'bin', process.platform === 'win32' ? 'gnomcode.exe' : 'gnomcode'));
    const result = spawnSync(process.execPath, [path.join(__dirname, '../bin/gnomcode.js'), '-e', 'process.stdout.write(process.argv[1]); process.exit(17)', 'forwarded'], { encoding: 'utf8', env: { ...process.env, NODE_PATH: dir } });
    assert.equal(result.stdout, 'forwarded');
    assert.equal(result.status, 17);
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});
