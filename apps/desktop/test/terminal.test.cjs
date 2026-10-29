const test = require('node:test');
const assert = require('node:assert/strict');
const { spawn } = require('node:child_process');
const path = require('node:path');

test('Electron permits a real binary terminal roundtrip through its request filter', {timeout:25000}, async t => {
  const env = {...process.env}; delete env.ELECTRON_RUN_AS_NODE;
  const child = spawn(require('electron'), [path.join(__dirname, 'terminal-electron.cjs')], {env,windowsHide:true,stdio:['ignore','pipe','pipe']});
  t.after(() => { if (child.exitCode === null) child.kill(); });
  let output = ''; child.stdout.on('data', data => {output+=data;}); child.stderr.on('data', data => {output+=data;});
  const code = await new Promise((resolve,reject) => {child.on('exit',resolve);child.on('error',reject);});
  assert.equal(code, 0, output);
  assert.match(output, /PASS: real sandboxed Electron renderer/);
});
