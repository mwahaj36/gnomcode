import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const presentation = await import('../lib/console-presentation.js');

test('classifyArtifactKind and isSafeArtifactImageSrc enforce safe artifact preview kinds', () => {
  assert.equal(typeof presentation.classifyArtifactKind, 'function');
  assert.equal(typeof presentation.isSafeArtifactImageSrc, 'function');

  assert.equal(presentation.classifyArtifactKind({ path: 'out/report.html' }), 'html');
  assert.equal(presentation.classifyArtifactKind({ path: 'docs/summary.md' }), 'markdown');
  assert.equal(presentation.classifyArtifactKind({ path: 'charts/plot.png', mime_type: 'image/png' }), 'image');
  assert.equal(presentation.classifyArtifactKind({ path: 'notes.txt' }), 'other');

  assert.equal(presentation.isSafeArtifactImageSrc('data:image/png;base64,iVBORw0KGgo='), true);
  assert.equal(presentation.isSafeArtifactImageSrc('data:image/svg+xml;base64,PHN2Zz4='), true);
  assert.equal(presentation.isSafeArtifactImageSrc('/api/v1/sessions/s1/artifacts/raw?path=a.png'), true);
  assert.equal(presentation.isSafeArtifactImageSrc('javascript:alert(1)'), false);
  assert.equal(presentation.isSafeArtifactImageSrc('https://evil.example/tracker.png'), false);
  assert.equal(presentation.isSafeArtifactImageSrc('data:text/html;base64,PHNjcmlwdD4='), false);
});

test('renderArtifactPreview isolates HTML in sandbox="" iframe and escapes markdown XSS', () => {
  assert.equal(typeof presentation.renderArtifactsList, 'function');
  assert.equal(typeof presentation.renderArtifactPreview, 'function');

  const items = [
    { id: '1', path: 'out/preview.html', kind: 'html', status: 'M', content: '<h1>Hi</h1><script>window.parent.hacked=1</script>' },
    { id: '2', path: 'docs/notes.md', kind: 'markdown', status: 'A', content: '# Notes\n\n<script>alert(1)</script><img src=x onerror=alert(2)>' },
    { id: '3', path: 'img/shot.png', kind: 'image', data_url: 'data:image/png;base64,AAAA' },
    { id: '4', path: 'img/bad.png', kind: 'image', data_url: 'javascript:alert(3)' }
  ];

  const listHtml = presentation.renderArtifactsList(items, '1');
  assert.match(listHtml, /out\/preview\.html/);
  assert.match(listHtml, /\[HTML\]/);
  assert.match(listHtml, /\[MD\]/);
  assert.match(listHtml, /\[IMG\]/);

  const htmlPreview = presentation.renderArtifactPreview(items[0]);
  assert.match(htmlPreview, /<iframe[^>]*sandbox=""/);
  assert.doesNotMatch(htmlPreview, /allow-scripts|allow-same-origin/);
  assert.doesNotMatch(htmlPreview, /<script>/);
  assert.match(htmlPreview, /srcdoc=/);

  const mdPreview = presentation.renderArtifactPreview(items[1]);
  assert.match(mdPreview, /<h1>Notes<\/h1>/);
  assert.doesNotMatch(mdPreview, /<script>|<img[^>]*onerror=/);
  assert.match(mdPreview, /&lt;script&gt;/);

  const imgPreview = presentation.renderArtifactPreview(items[2]);
  assert.match(imgPreview, /<img[^>]*src="data:image\/png;base64,AAAA"/);

  const badImgPreview = presentation.renderArtifactPreview(items[3]);
  assert.doesNotMatch(badImgPreview, /<img/);
  assert.match(badImgPreview, /unavailable/i);
});

test('artifacts view wires into console and Remote without raw hex CSS tokens', () => {
  const html = readFileSync(join(root, 'crates/daemon/src/console.html'), 'utf8');
  const js = readFileSync(join(root, 'crates/daemon/src/console-app.js'), 'utf8');
  const tsx = readFileSync(join(root, 'web/components/RemoteCompanion.tsx'), 'utf8');
  const css = readFileSync(join(root, 'web/app/gnomcode-console.css'), 'utf8');

  assert.match(html, /id="chat-artifacts-btn"/);
  assert.match(html, /id="chat-artifacts-panel"/);
  assert.match(js, /\/api\/v1\/sessions\/.*\/artifacts/);
  assert.match(js, /renderArtifactsList/);
  assert.match(js, /renderArtifactPreview/);

  assert.match(tsx, /sp-artifacts-sheet/);
  assert.match(tsx, /type:\s*['"]artifacts['"]/);
  assert.match(tsx, /sandbox=""/);
  assert.match(tsx, /Unsupported remote command/);

  const marker = css.indexOf('/* Section C artifacts view */');
  assert.notEqual(marker, -1, 'Expected Section C artifacts view CSS block');
  const sectionCss = css.slice(marker);
  assert.doesNotMatch(sectionCss, /#[0-9a-fA-F]{3,8}\b/, 'Use --sp-* tokens only, no raw hex colors');
});
