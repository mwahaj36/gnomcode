const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { inspectReleaseInputs } = require('../check-inputs.cjs');

test('packaging rejects missing, empty or version-mismatched release inputs', () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'gnomcode-inputs-'));
  try {
    assert.ok(inspectReleaseInputs(dir, '0.9.0', '0.9.0', 'win32').length > 0);
    fs.mkdirSync(path.join(dir, 'bin'));
    for (const file of ['EULA.txt', 'THIRD_PARTY_NOTICES.html', 'DESKTOP_THIRD_PARTY_NOTICES.html', 'ELECTRON_LICENSE.txt', 'CHROMIUM_LICENSES.html', path.join('bin', 'gnomcode.exe')]) fs.writeFileSync(path.join(dir, file), '');
    assert.ok(inspectReleaseInputs(dir, '0.9.0', '0.9.0', 'win32').length > 0);
    for (const file of ['EULA.txt', 'THIRD_PARTY_NOTICES.html', 'DESKTOP_THIRD_PARTY_NOTICES.html', 'ELECTRON_LICENSE.txt', 'CHROMIUM_LICENSES.html', path.join('bin', 'gnomcode.exe')]) fs.writeFileSync(path.join(dir, file), 'test');
    assert.ok(inspectReleaseInputs(dir, '0.9.0', '1.5.3', 'win32').length > 0);
    assert.deepEqual(inspectReleaseInputs(dir, '0.9.0', '0.9.0', 'win32'), []);
  } finally { fs.rmSync(dir, {recursive: true, force: true}); }
});

test('packaging requires Electron, Chromium and desktop dependency notices', () => {
  const dir=fs.mkdtempSync(path.join(os.tmpdir(),'gnomcode-desktop-licenses-'));
  try {
    fs.mkdirSync(path.join(dir,'bin'));
    for (const file of ['EULA.txt','THIRD_PARTY_NOTICES.html',path.join('bin','gnomcode.exe')]) fs.writeFileSync(path.join(dir,file),'test');
    assert.ok(inspectReleaseInputs(dir,'0.9.0','0.9.0','win32').length > 0);
  } finally { fs.rmSync(dir,{recursive:true,force:true}); }
});

test('packaging refuses a draft EULA even when all files are present', () => {
  const dir=fs.mkdtempSync(path.join(os.tmpdir(),'gnomcode-draft-license-'));
  try {
    fs.mkdirSync(path.join(dir,'bin'));
    for (const file of ['EULA.txt','THIRD_PARTY_NOTICES.html','DESKTOP_THIRD_PARTY_NOTICES.html','ELECTRON_LICENSE.txt','CHROMIUM_LICENSES.html',path.join('bin','gnomcode.exe')]) fs.writeFileSync(path.join(dir,file),'test');
    fs.writeFileSync(path.join(dir,'EULA.txt'),'DRAFT: not approved for release');
    assert.ok(inspectReleaseInputs(dir,'0.9.0','0.9.0','win32').length > 0);
  } finally { fs.rmSync(dir,{recursive:true,force:true}); }
});
