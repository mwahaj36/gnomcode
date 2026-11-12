import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
  normalizeWorkspaceRelPath,
  renderFileEntries,
  renderFileViewer,
} from '../lib/console-presentation.js';

const read = path => readFileSync(new URL(path, import.meta.url), 'utf8');

test('normalizeWorkspaceRelPath confines inputs to workspace-relative paths', () => {
  assert.deepEqual(normalizeWorkspaceRelPath('src\\main.rs'), { ok: true, path: 'src/main.rs' });
  assert.deepEqual(normalizeWorkspaceRelPath('./README.md'), { ok: true, path: 'README.md' });
  assert.equal(normalizeWorkspaceRelPath('').ok, false);
  assert.equal(normalizeWorkspaceRelPath('/etc/passwd').ok, false);
  assert.equal(normalizeWorkspaceRelPath('C:\\Windows\\win.ini').ok, false);
  assert.equal(normalizeWorkspaceRelPath('../secret.txt').ok, false);
  assert.equal(normalizeWorkspaceRelPath('a/../../secret.txt').ok, false);
  assert.equal(normalizeWorkspaceRelPath('bad\u0000file').ok, false);
});

test('renderFileEntries and renderFileViewer escape content and stay read-only', () => {
  const listHtml = renderFileEntries({
    path: 'src',
    parent: '',
    entries: [
      { name: 'components', path: 'src/components', kind: 'dir' },
      { name: '<script>.ts', path: 'src/<script>.ts', kind: 'file', size: 128 },
    ],
  });
  assert.doesNotMatch(listHtml, /<script>/);
  assert.match(listHtml, /data-action="open-repo-dir"/);
  assert.match(listHtml, /data-action="open-repo-file"/);

  const viewHtml = renderFileViewer({
    path: 'src/<img onerror=x>.ts',
    content: 'const x = "<script>alert(1)</script>";\nreturn x;\n',
    lossy: true,
  });
  assert.doesNotMatch(viewHtml, /<script>|<img onerror=/);
  assert.match(viewHtml, /Read-only/);
  assert.match(viewHtml, /Lossy UTF-8 decoding/);
  assert.match(viewHtml, /sp-file-code/);
  assert.doesNotMatch(viewHtml, /<textarea|contenteditable|save|write|delete/i);
});

test('file viewer wires into local console and Remote', () => {
  const remote = read('../components/RemoteCompanion.tsx');
  const consoleHtml = read('../../crates/daemon/src/console.html');
  const consoleJs = read('../../crates/daemon/src/console-app.js');
  const css = read('../app/gnomcode-console.css');

  assert.match(consoleHtml, /id="chat-files-btn"/);
  assert.match(consoleHtml, /id="chat-files-panel"/);
  assert.match(consoleHtml, /id="chat-file-path-form"/);
  assert.match(consoleJs, /\/api\/v1\/repos\/file/);
  assert.match(consoleJs, /'toggle-files':\s*toggleFilesPanel/);

  assert.match(remote, /type:\s*'repo_file'/);
  assert.match(remote, /type:\s*'repo_files'/);
  assert.match(remote, /sp-files-sheet/);

  const cssSlice = css.slice(css.indexOf('.sp-files-panel{'));
  assert.ok(css.includes('.sp-files-panel{'));
  assert.doesNotMatch(cssSlice, /#[0-9a-f]{3,6}\b/i);
});
