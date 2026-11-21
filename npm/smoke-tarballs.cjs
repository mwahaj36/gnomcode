'use strict';
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const version = require('./gnomcode/package.json').version;
const artifacts = path.resolve(process.argv[2] || '.');
const musl = process.platform === 'linux' && !process.report.getReport().header.glibcVersionRuntime;
const target = `${process.platform}-${process.arch}${musl ? '-musl' : ''}`;
const files = [`gnomcode-cli-${target}-${version}.tgz`, `gnomcode-${version}.tgz`].map(name => path.join(artifacts, name));
for (const file of files) if (!fs.statSync(file).isFile()) throw new Error(`Missing tarball: ${file}`);
const stage = fs.mkdtempSync(path.join(os.tmpdir(), 'gnomcode-packed-smoke-'));
function run(command, args, env) {
  if (process.platform === 'win32' && command === 'npm') {
    args = [path.join(path.dirname(process.execPath), 'node_modules/npm/bin/npm-cli.js'), ...args];
    command = process.execPath;
  }
  const result = spawnSync(command, args, { cwd: stage, env, encoding: 'utf8', timeout: 120000 });
  if (result.error || result.status !== 0) throw new Error(`${command} failed: ${result.error?.message || result.stderr}`);
  return result.stdout.trim();
}
try {
  fs.writeFileSync(path.join(stage, 'package.json'), '{"private":true}');
  run('npm', ['install', '--offline', '--ignore-scripts', '--no-audit', '--no-fund', ...files], process.env);
  const home = path.join(stage, 'home');
  fs.mkdirSync(home);
  const env = { ...process.env, GNOMCODE_HOME: home, HOME: home, USERPROFILE: home, APPDATA: home, LOCALAPPDATA: home,
    CLAUDE_CONFIG_DIR: home, CODEX_HOME: home, GEMINI_HOME: home, GNOMCODE_DISABLE_WSL: '1' };
  const launcher = path.join(stage, 'node_modules/gnomcode/bin/gnomcode.js');
  if (run(process.execPath, [launcher, '--version'], env) !== `gnomcode ${version}`) throw new Error('Packed binary version mismatch');
  run(process.execPath, [launcher, 'doctor'], env);
  run(process.execPath, [launcher, 'report'], env);
  console.log(`PASS: ${target} tarballs install offline and run version, doctor and report.`);
} finally { fs.rmSync(stage, { recursive: true, force: true, maxRetries: 5 }); }
