const fs = require('node:fs');
const path = require('node:path');

const targets = new Set(['win32-x64', 'win32-arm64', 'darwin-x64', 'darwin-arm64', 'linux-x64', 'linux-arm64', 'linux-x64-musl']);

function stagePlatform({ root, target, binary, eula, notices }) {
  if (!targets.has(target)) throw new Error('Unsupported release target.');
  const folder = path.join(root, 'npm', 'platform', target);
  const manifest = JSON.parse(fs.readFileSync(path.join(folder, 'package.json'), 'utf8'));
  const cargo = fs.readFileSync(path.join(root, 'Cargo.toml'), 'utf8');
  const version = cargo.match(/\[workspace\.package\][\s\S]*?\nversion\s*=\s*"([^"]+)"/)?.[1];
  if (manifest.name !== `@spawnt/cli-${target}` || !version || manifest.version !== version) throw new Error('Platform package name or version does not match the workspace.');
  for (const source of [binary, eula, notices]) {
    const stat = fs.statSync(source);
    if (!stat.isFile() || stat.size === 0) throw new Error('A required release input is not a non-empty file.');
  }
  if (/\bdraft\b/i.test(fs.readFileSync(eula, 'utf8'))) throw new Error('Approved EULA required.');
  const bin = path.join(folder, 'bin');
  fs.mkdirSync(bin, { recursive: true });
  const targetBinary = path.join(bin, target.startsWith('win32-') ? 'spawnt.exe' : 'spawnt');
  fs.copyFileSync(binary, targetBinary);
  if (!target.startsWith('win32-')) fs.chmodSync(targetBinary, 0o755);
  fs.copyFileSync(eula, path.join(folder, 'EULA.txt'));
  fs.copyFileSync(notices, path.join(folder, 'THIRD_PARTY_NOTICES.html'));
}

if (require.main === module) {
  try {
    if (process.argv.length !== 6) throw new Error('Usage: node stage-release.cjs <target> <binary> <EULA.txt> <THIRD_PARTY_NOTICES.html>');
    const [, , target, binary, eula, notices] = process.argv;
    stagePlatform({ root: path.resolve(__dirname, '../..'), target, binary, eula, notices });
    console.log(`Staged ${target} release inputs.`);
  } catch (error) { console.error(error.message); process.exitCode = 1; }
}
module.exports = { stagePlatform };
