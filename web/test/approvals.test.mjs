import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const read = path => readFileSync(new URL(path, import.meta.url), 'utf8');
const remote = read('../components/RemoteCompanion.tsx');
const consoleJs = read('../../crates/daemon/src/console-app.js');
const consoleHtml = read('../../crates/daemon/src/console.html');
const css = read('../app/gnomcode-console.css');

test('console lists pending approvals and sends one explicit decision per exact id', () => {
  assert.match(consoleHtml, /id="approvals"[^>]*hidden/);
  assert.match(consoleJs, /\/api\/v1\/approvals/);
  assert.match(consoleJs, /\/api\/v1\/approvals\/\$\{encodeURIComponent\(id\)\}\/decision/);
  assert.match(consoleJs, /'approval-decide': decideApproval/);
  // Content comes through escapeHtml; a decision is never inferred from anything but a button.
  assert.doesNotMatch(consoleJs, /auto.?(allow|approve)/i);
});

test('phone shows approve and deny for each pending request over the encrypted channel', () => {
  assert.match(remote, /type:'approvals'/);
  assert.match(remote, /type:'approval',id,decision/);
  assert.match(remote, /decideApproval\(item\.id,'deny'\)/);
  assert.match(remote, /decideApproval\(item\.id,'approve'\)/);
  assert.match(remote, /Approve \$\{item\.tool\} once/);
  assert.doesNotMatch(remote, /always allow|auto.?approve/i);
});

test('approval card keeps phone targets at 44px and uses neutral controls', () => {
  assert.match(css, /\.sp-approval-actions \.btn-control\{flex:1;min-height:44px\}/);
  assert.match(css, /\.sp-approval\{[^}]*border-radius:var\(--sp-radius-card\)/);
  assert.doesNotMatch(css.slice(css.indexOf('.sp-approvals{')), /#[0-9a-f]{3,6}\b/i);
});
