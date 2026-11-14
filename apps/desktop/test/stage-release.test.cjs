const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { stagePlatform } = require('../stage-release.cjs');

test('stages only a matching platform binary and required legal notices', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'gnomcode-stage-test-'));
  try {
    const pkg = path.join(root, 'npm', 'platform', 'win32-x64');
    fs.mkdirSync(pkg, {recursive:true});
    fs.writeFileSync(path.join(pkg,'package.json'), JSON.stringify({name:'@gnomcode/cli-win32-x64',version:'0.9.0'}));
    fs.writeFileSync(path.join(root,'Cargo.toml'),'[workspace.package]\nversion = "0.9.0"\n');
    for (const file of ['binary','eula','notices']) fs.writeFileSync(path.join(root,file),file);
    assert.throws(() => stagePlatform({root, target:'../unsafe', binary:path.join(root,'binary'), eula:path.join(root,'eula'), notices:path.join(root,'notices')}));
    stagePlatform({root, target:'win32-x64', binary:path.join(root,'binary'), eula:path.join(root,'eula'), notices:path.join(root,'notices')});
    assert.equal(fs.readFileSync(path.join(pkg,'bin','gnomcode.exe'),'utf8'),'binary');
    assert.equal(fs.readFileSync(path.join(pkg,'LICENSE'),'utf8'),'eula');
    assert.equal(fs.readFileSync(path.join(pkg,'THIRD_PARTY_NOTICES.html'),'utf8'),'notices');
  } finally { fs.rmSync(root,{recursive:true,force:true}); }
});
