const fs = require('node:fs');
const path = require('node:path');

function inspectReleaseInputs(staging, appVersion, workspaceVersion, platform) {
  const errors = [];
  if (appVersion !== workspaceVersion) errors.push('Desktop version does not match the Rust workspace.');
  const binary = platform === 'win32' ? 'gnomcode.exe' : 'gnomcode';
  for (const name of ['EULA.txt', 'THIRD_PARTY_NOTICES.html', 'DESKTOP_THIRD_PARTY_NOTICES.html', 'ELECTRON_LICENSE.txt', 'CHROMIUM_LICENSES.html', path.join('bin', binary)]) {
    try {
      const stat = fs.statSync(path.join(staging, name));
      if (!stat.isFile() || stat.size === 0) errors.push(`${name} must be a non-empty file.`);
    } catch { errors.push(`${name} is missing.`); }
  }
  try {
    if (/\bdraft\b/i.test(fs.readFileSync(path.join(staging, 'EULA.txt'), 'utf8'))) errors.push('EULA is marked as a draft.');
  } catch { /* Missing EULA is already reported above. */ }
  return errors;
}

if (require.main === module) {
  const manifest = require('./package.json');
  const cargo = fs.readFileSync(path.join(__dirname, '../../Cargo.toml'), 'utf8');
  const workspaceVersion = cargo.match(/\[workspace\.package\][\s\S]*?\nversion\s*=\s*"([^"]+)"/)?.[1];
  const errors = inspectReleaseInputs(path.join(__dirname, 'staging'), manifest.version, workspaceVersion, process.platform);
  if (errors.length) { console.error(errors.join('\n')); process.exitCode = 1; }
}
module.exports = { inspectReleaseInputs };
