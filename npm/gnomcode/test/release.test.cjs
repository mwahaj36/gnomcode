const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { inspectRelease } = require('../../check-release.cjs');

test('rejects incomplete platform manifests and empty release files', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(),'gnomcode-npm-check-'));
  try {
    fs.mkdirSync(path.join(root,'gnomcode'),{recursive:true});
    fs.writeFileSync(path.join(root,'Cargo.toml'),'[workspace.package]\nversion = "0.9.0"\n');
    fs.writeFileSync(path.join(root,'gnomcode','package.json'),JSON.stringify({version:'0.9.0',optionalDependencies:{}}));
    for (const file of ['LICENSE','THIRD_PARTY_NOTICES.html']) fs.writeFileSync(path.join(root,'gnomcode',file),'');
    const errors=inspectRelease(root,path.join(root,'Cargo.toml'));
    assert.ok(errors.some(e=>/platform/i.test(e)));
    assert.ok(errors.some(e=>/license/i.test(e)));
  } finally { fs.rmSync(root,{recursive:true,force:true}); }
});

test('rejects draft license release inputs', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(),'gnomcode-npm-draft-'));
  try {
    fs.mkdirSync(path.join(root,'gnomcode'),{recursive:true});
    fs.writeFileSync(path.join(root,'Cargo.toml'),'[workspace.package]\nversion = "0.9.0"\n');
    fs.writeFileSync(path.join(root,'gnomcode','package.json'),JSON.stringify({version:'0.9.0',optionalDependencies:{}}));
    fs.writeFileSync(path.join(root,'gnomcode','LICENSE'),'DRAFT: not for release');
    fs.writeFileSync(path.join(root,'gnomcode','THIRD_PARTY_NOTICES.html'),'notice');
    assert.ok(inspectRelease(root,path.join(root,'Cargo.toml')).some(e=>/draft/i.test(e)));
  } finally { fs.rmSync(root,{recursive:true,force:true}); }
});

test('rejects a non-draft license different from the approved legal source', () => {
 const root=fs.mkdtempSync(path.join(os.tmpdir(),'gnomcode-eula-'));
 try {
  fs.mkdirSync(path.join(root,'gnomcode'),{recursive:true});
  fs.mkdirSync(path.join(root,'docs/legal'),{recursive:true});
  fs.writeFileSync(path.join(root,'LICENSE'),'approved terms');
  fs.writeFileSync(path.join(root,'Cargo.toml'),'[workspace.package]\nversion = "1.0.0"\n');
  fs.writeFileSync(path.join(root,'gnomcode/package.json'),JSON.stringify({version:'1.0.0',optionalDependencies:{}}));
  fs.writeFileSync(path.join(root,'gnomcode/LICENSE'),'different terms');
  assert.ok(inspectRelease(root,path.join(root,'Cargo.toml')).some(e=>/approved license/i.test(e)));
 } finally {fs.rmSync(root,{recursive:true,force:true});}
});
