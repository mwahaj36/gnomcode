const fs = require('node:fs');
const path = require('node:path');

const escape = value => String(value).replace(/[&<>"']/g, char => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[char]));

function renderNotices(directories, bundledFiles = []) {
  const sections = directories.map(dir => {
    const pkg = JSON.parse(fs.readFileSync(path.join(dir, 'package.json'), 'utf8'));
    const licenseName = fs.readdirSync(dir).find(name => /^licen[sc]e(?:\.(?:md|txt))?$/i.test(name));
    if (!licenseName) throw new Error(`License file missing for ${pkg.name}.`);
    const license = fs.readFileSync(path.join(dir, licenseName), 'utf8');
    if (!license.trim()) throw new Error(`License file is empty for ${pkg.name}.`);
    return `<section><h2>${escape(pkg.name)} ${escape(pkg.version)}</h2><p>${escape(pkg.license || 'See license')}</p><pre>${escape(license)}</pre></section>`;
  });
  for (const file of bundledFiles) {
    const license = fs.readFileSync(file, 'utf8');
    if (!license.trim()) throw new Error(`License file is empty: ${file}`);
    sections.push(`<section><h2>${escape(path.basename(file))}</h2><pre>${escape(license)}</pre></section>`);
  }
  return `<!doctype html><html lang="en"><meta charset="utf-8"><title>Desktop third-party notices</title><h1>Desktop third-party notices</h1><p>Electron and Chromium licenses are included in ELECTRON_LICENSE.txt and CHROMIUM_LICENSES.html. Rust notices are included in THIRD_PARTY_NOTICES.html.</p>${sections.join('\n')}</html>`;
}

function productionDependencies(root) {
  const visited = new Set();
  const directories = [];
  function visit(dir) {
    const pkg = JSON.parse(fs.readFileSync(path.join(dir, 'package.json'), 'utf8'));
    for (const name of Object.keys(pkg.dependencies || {})) {
      const manifest = require.resolve(`${name}/package.json`, { paths: [dir] });
      const child = path.dirname(manifest);
      if (visited.has(child)) continue;
      visited.add(child);
      directories.push(child);
      visit(child);
    }
  }
  visit(root);
  return directories;
}

if (require.main === module) {
  try {
    const out = path.join(__dirname, 'staging');
    const web = path.join(__dirname, '../../web/public');
    const bundledFiles = ['XTERM-LICENSE.txt', 'XTERM-FIT-LICENSE.txt', 'MARKDOWN-LICENSE.txt']
      .map(name => path.join(web, 'vendor', name));
    bundledFiles.push(...fs.readdirSync(path.join(web, 'fonts')).filter(name => name.endsWith('-LICENSE.txt'))
      .map(name => path.join(web, 'fonts', name)));
    const html = renderNotices(productionDependencies(__dirname), bundledFiles);
    const electronDir = path.dirname(require.resolve('electron/package.json'));
    const electronLicense = path.join(electronDir, 'dist', 'LICENSE');
    const chromiumLicenses = path.join(electronDir, 'dist', 'LICENSES.chromium.html');
    for (const file of [electronLicense, chromiumLicenses]) if (!fs.statSync(file).size) throw new Error('Electron license input is empty.');
    fs.mkdirSync(out, {recursive:true});
    fs.writeFileSync(path.join(out, 'DESKTOP_THIRD_PARTY_NOTICES.html'), html);
    fs.copyFileSync(electronLicense, path.join(out, 'ELECTRON_LICENSE.txt'));
    fs.copyFileSync(chromiumLicenses, path.join(out, 'CHROMIUM_LICENSES.html'));
    console.log('Desktop dependency notices staged.');
  } catch (error) { console.error(error.message); process.exitCode = 1; }
}
module.exports = { renderNotices, productionDependencies };
