import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
  parseGitStatusLines,
  renderChangesView,
} from '../lib/console-presentation.js';

const read = path => readFileSync(new URL(path, import.meta.url), 'utf8');

test('parseGitStatusLines maps porcelain status codes including untracked files', () => {
  const rows = parseGitStatusLines(' M src/app.ts\r\nA  src/new.ts\nD  old.ts\n?? untracked.md\n');
  assert.deepEqual(rows, [
    { code: 'M', label: 'Modified', path: 'src/app.ts' },
    { code: 'A', label: 'Added', path: 'src/new.ts' },
    { code: 'D', label: 'Deleted', path: 'old.ts' },
    { code: '??', label: 'Untracked', path: 'untracked.md' },
  ]);
  assert.deepEqual(parseGitStatusLines(''), []);
});

test('renderChangesView shows clean state, lossy banner, escaped diff, and stays strictly read-only', () => {
  assert.match(renderChangesView({ repo: '/repo', status: '', diff: '', lossy: false }), /Working tree clean/);

  const markup = renderChangesView({
    repo: '/repo/<script>',
    status: ' M src/a.ts\n?? notes/<img onerror=x>.md\n',
    diff: 'diff --git a/src/a.ts b/src/a.ts\n@@ -1,1 +1,2 @@\n-old <script>\n+new line\n',
    lossy: true,
  });

  assert.doesNotMatch(markup, /<script>|<img onerror=/);
  assert.match(markup, /Lossy UTF-8 decoding/);
  assert.match(markup, /Untracked/);
  assert.match(markup, /sp-diff-line--add/);
  assert.match(markup, /sp-diff-line--del/);
  assert.match(markup, /sp-diff-line--hunk/);
  assert.doesNotMatch(markup, /revert|discard|checkout/i);
});

test('changes view wires into local console and Remote without mutation controls', () => {
  const remote = read('../components/RemoteCompanion.tsx');
  const consoleHtml = read('../../crates/daemon/src/console.html');
  const consoleJs = read('../../crates/daemon/src/console-app.js');
  const css = read('../app/gnomcode-console.css');

  assert.match(consoleHtml, /id="chat-changes-btn"/);
  assert.match(consoleHtml, /id="chat-changes-panel"/);
  assert.match(consoleHtml, /id="chat-changes-body"/);
  assert.match(consoleJs, /\/api\/v1\/sessions\/\$\{encodeURIComponent\(activeChatSession\)\}\/changes/);
  assert.match(consoleJs, /'toggle-changes':\s*toggleChangesPanel/);
  assert.match(consoleJs, /'refresh-changes':\s*loadSessionChanges/);

  assert.match(remote, /type:\s*'changes'/);
  assert.match(remote, /reply\.type\s*===\s*'changes'/);
  assert.match(remote, /sp-changes-sheet|Changes/);

  const cssSlice = css.slice(css.indexOf('.sp-changes-panel{'));
  assert.ok(css.includes('.sp-changes-panel{'));
  assert.doesNotMatch(cssSlice, /#[0-9a-f]{3,6}\b/i);
});
