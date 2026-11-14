'use strict';
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const root = __dirname;
const shim = path.join(root, 'gnomcode');
const version = require(path.join(shim, 'package.json')).version;
const header = process.platform === 'linux' ? process.report.getReport().header : null;
const suffix = `${process.platform}-${process.arch}${header && !header.glibcVersionRuntime ? '-musl' : ''}`;
const platform = path.join(root, 'platform', suffix);
if (!fs.existsSync(platform)) throw new Error(`No Gnomcode package for ${suffix}`);
for (const dir of [shim, platform]) {
  for (const name of ['LICENSE', 'THIRD_PARTY_NOTICES.html']) {
    if (!fs.statSync(path.join(dir, name)).size) throw new Error(`Empty ${name}`);
  }
  if (/\bdraft\b/i.test(fs.readFileSync(path.join(dir, 'LICENSE'), 'utf8'))) throw new Error('Approved license required.');
}

const stage = fs.mkdtempSync(path.join(os.tmpdir(), 'gnomcode-npm-smoke-'));
function run(command, args, cwd, env = process.env) {
  const result = spawnSync(command, args, { cwd, env, encoding: 'utf8', shell: process.platform === 'win32' });
  if (result.error || result.status !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed: ${result.error?.message || result.stderr}`);
  }
  return result.stdout.trim();
}
try {
  const packArgs = ['pack', '--ignore-scripts', '--json', '--pack-destination', stage];
  const platformPack = JSON.parse(run('npm', [...packArgs, platform], root))[0].filename;
  const shimPack = JSON.parse(run('npm', [...packArgs, shim], root))[0].filename;
  const install = path.join(stage, 'install');
  fs.mkdirSync(install);
  fs.writeFileSync(path.join(install, 'package.json'), '{"private":true}');
  run('npm', ['install', '--offline', '--ignore-scripts', '--no-audit', '--no-fund', path.join(stage, platformPack), path.join(stage, shimPack)], install);
  const binary = path.join(install, 'node_modules', '.bin', process.platform === 'win32' ? 'gnomcode.cmd' : 'gnomcode');
  const home = path.join(stage, 'home');
  fs.mkdirSync(home);
  const env = { ...process.env, GNOMCODE_HOME: home, HOME: home, USERPROFILE: home,
    APPDATA: home, LOCALAPPDATA: home, CLAUDE_CONFIG_DIR: home,
    CODEX_HOME: home, GEMINI_HOME: home };
  const actual = run(binary, ['--version'], install, env);
  if (actual !== `gnomcode ${version}`) throw new Error(`Unexpected version: ${actual}`);
  run(binary, ['doctor'], install, env);
  run(binary, ['report'], install, env);
  console.log(`PASS: ${suffix} npm tarballs install and run version, doctor and report offline`);
} finally {
  fs.rmSync(stage, { recursive: true, force: true });
}
