const fs = require('node:fs');
const path = require('node:path');

const targets = ['win32-x64','win32-arm64','darwin-x64','darwin-arm64','linux-x64','linux-arm64','linux-x64-musl'];
function inspectRelease(root, cargoPath) {
  const errors = [];
  const shim = JSON.parse(fs.readFileSync(path.join(root, 'spawnt/package.json'), 'utf8'));
  const cargo = fs.readFileSync(cargoPath, 'utf8');
  const workspaceVersion = cargo.match(/\[workspace\.package\][\s\S]*?\nversion\s*=\s*"([^"]+)"/);
  if (!workspaceVersion || workspaceVersion[1] !== shim.version) errors.push('spawnt: version does not match Cargo workspace');
  const dependencies = shim.optionalDependencies || {};
  const expected = targets.map(target => `@spawnt/cli-${target}`);
  if (Object.keys(dependencies).length !== expected.length || expected.some(name => !(name in dependencies))) errors.push('spawnt: platform dependency list is incomplete or unexpected');
  const checkFile = (dir, file, label) => {
    try { if (!fs.statSync(path.join(dir, file)).isFile() || fs.statSync(path.join(dir, file)).size === 0) throw new Error(); }
    catch { errors.push(`${label}: missing or empty ${file}`); }
  };
  for (const target of targets) {
    const name = `@spawnt/cli-${target}`;
    const dir = path.join(root, 'platform', target);
    try {
      const manifest = JSON.parse(fs.readFileSync(path.join(dir, 'package.json'), 'utf8'));
      if (manifest.name !== name || manifest.version !== dependencies[name] || manifest.version !== shim.version) errors.push(`${name}: version or name mismatch`);
    } catch { errors.push(`${name}: manifest missing or invalid`); }
    for (const file of ['EULA.txt', 'THIRD_PARTY_NOTICES.html', path.join('bin', target.startsWith('win32-') ? 'spawnt.exe' : 'spawnt')]) checkFile(dir, file, name);
  }
  for (const file of ['EULA.txt', 'THIRD_PARTY_NOTICES.html']) checkFile(path.join(root, 'spawnt'), file, 'spawnt');
  for (const dir of [path.join(root, 'spawnt'), ...targets.map(target => path.join(root, 'platform', target))]) {
    try { if (/\bdraft\b/i.test(fs.readFileSync(path.join(dir, 'EULA.txt'), 'utf8'))) errors.push(`${dir}: EULA is marked as a draft`); } catch {}
  }
  return errors;
}
if (require.main === module) {
  const errors = inspectRelease(__dirname, path.join(__dirname, '..', 'Cargo.toml'));
  if (errors.length) { console.error(errors.join('\n')); process.exitCode = 1; }
  else console.log('npm release inputs are present and versions match');
}
module.exports = { inspectRelease };
