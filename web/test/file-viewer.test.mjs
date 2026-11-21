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

test('file viewer wires into Remote', () => {
  const remote = read('../components/RemoteCompanion.tsx');
  const css = read('../app/gnomcode-console.css');

  assert.match(remote, /type:\s*'repo_file'/);
  assert.match(remote, /type:\s*'repo_files'/);
  assert.match(remote, /sp-files-sheet/);

  const cssSlice = css.slice(css.indexOf('.sp-files-panel{'));
  assert.ok(css.includes('.sp-files-panel{'));
  assert.doesNotMatch(cssSlice, /#[0-9a-f]{3,6}\b/i);
});

test('renderFileViewer bounds 40000-line text previews to prevent DOM stalls (#334)', () => {
  const big = Array.from({ length: 40000 }, (_, i) => `line-${i + 1}: payload`).join('\n');
  const html = renderFileViewer({
    path: 'logs/large.txt',
    content: big,
    lossy: false,
  });
  const lineCount = (html.match(/class="sp-file-line"/g) || []).length;
  assert.ok(lineCount > 0 && lineCount <= 500, `expected bounded lines, got ${lineCount}`);
  assert.match(html, /Showing first 500 of 40,000 lines/);
});

test('Remote has one composer toolbar, durable drafts, queue_if_busy, and onboarding offers account skip (#326 #337 #338 #333)', () => {
  const remote = read('../components/RemoteCompanion.tsx');
  const toolbarCount = (remote.match(/className="remote-composer-toolbar"/g) || []).length;
  assert.equal(toolbarCount, 1, 'Remote should have exactly one composer toolbar');
  assert.match(remote, /queue_if_busy:\s*true/);
  assert.match(remote, /gnomcode-remote-drafts/);

  const onboarding = read('../local-console/Onboarding.tsx');
  assert.match(onboarding, /data-onboarding-step="account"/);
  assert.match(onboarding, /data-onboarding-action="signin"/);
  assert.match(onboarding, /data-onboarding-action="skip-account"/);

  const setup = read('../../scripts/Setup.cs');
  assert.match(setup, /ClearGnomcodeOwnedCredentialsAndUiState/);
});

test('UI overhaul fixes Antigravity mark, Home full-width sessions, Settings isolation, and mobile Remote nav (#328 #322 #330 #331 #323)', async () => {
  const { PROVIDER_LOGOS } = await import('../lib/provider-logos.js');
  assert.ok(PROVIDER_LOGOS.antigravity.startsWith('M21.751'), 'Antigravity should use its own mark');

  const css = read('../app/gnomcode-console.css');
  assert.match(css, /\.gc-console \.sp-home-grid\{display:grid;grid-template-columns:minmax\(0,2fr\) minmax\(280px,1fr\)/);
  assert.match(css, /\.sp-heatmap-legend \.sp-heat/);
  assert.match(css, /@container \(max-width: 600px\)/);
  assert.match(css, /\.sp-workspace-modal/);

  assert.match(css, /:not\(\[hidden\]\) > \.sp-remote:is\(\.is-paired,\.is-reconnecting\)/);

  const settings = read('../local-console/SettingsView.tsx');
  assert.match(settings, /id="settings-pricing"[^>]*hidden=\{!shown\('settings-pricing'\)\}/);
  assert.doesNotMatch(settings, /Offline license/);
});


test('local document links open the session viewer while network links stay external', async () => {
  const { renderChatCards } = await import('../lib/console-presentation.js');
  const html = renderChatCards([{ event_id: 'link', role: 'assistant', content: '[Handoff](localdocs/handoff.md) [Site](https://example.org)', created_at: '2026-11-16T00:00:00Z' }]);
  assert.match(html, /data-workspace-file="localdocs\/handoff.md"/);
  assert.match(html, /href="https:\/\/example.org" target="_blank"/);
});
