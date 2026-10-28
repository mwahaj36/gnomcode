'use strict';
const fs = require('node:fs');
const path = require('node:path');

const targets = ['win32-x64', 'win32-arm64', 'darwin-x64', 'darwin-arm64', 'linux-x64', 'linux-arm64', 'linux-x64-musl'];
const [artifactRoot, eulaFile, noticesFile] = process.argv.slice(2).map(value => value && path.resolve(value));
if (!artifactRoot || !eulaFile || !noticesFile) {
  console.error('Usage: node npm/stage-release.cjs <artifact-directory> <approved-EULA.txt> <THIRD_PARTY_NOTICES.html>');
  process.exit(2);
}
const eula = fs.readFileSync(eulaFile);
const notices = fs.readFileSync(noticesFile);
if (!eula.length || /\bdraft\b/i.test(eula.toString('utf8')) || !notices.length) {
  throw new Error('An approved EULA and non-empty third-party notices are required.');
}
const binaries = targets.map(target => {
  const name = target.startsWith('win32-') ? 'spawnt.exe' : 'spawnt';
  const file = path.join(artifactRoot, target, name);
  const bytes = fs.readFileSync(file);
  const header = bytes.subarray(0, 4).toString('hex');
  const expected = target.startsWith('win32-') ? '4d5a' : target.startsWith('darwin-') ? 'cffaedfe' : '7f454c46';
  if (!bytes.length || !header.startsWith(expected)) throw new Error(`Unexpected binary format: ${file}`);
  return { target, name, file };
});
for (const { target, name, file } of binaries) {
  const dest = path.join(__dirname, 'platform', target);
  fs.mkdirSync(path.join(dest, 'bin'), { recursive: true });
  fs.copyFileSync(file, path.join(dest, 'bin', name));
  fs.copyFileSync(eulaFile, path.join(dest, 'EULA.txt'));
  fs.copyFileSync(noticesFile, path.join(dest, 'THIRD_PARTY_NOTICES.html'));
}
fs.copyFileSync(eulaFile, path.join(__dirname, 'spawnt', 'EULA.txt'));
fs.copyFileSync(noticesFile, path.join(__dirname, 'spawnt', 'THIRD_PARTY_NOTICES.html'));
console.log('Staged seven Spawnt npm platform packages and the shim.');
