import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const presentation = await import('../lib/console-presentation.js');

test('validateAttachmentCandidate enforces 15 MiB cap, safe filename, and allowed extensions', () => {
  assert.equal(typeof presentation.validateAttachmentCandidate, 'function');
  assert.equal(presentation.MAX_ATTACHMENT_BYTES, 15 * 1024 * 1024);

  const ok = presentation.validateAttachmentCandidate({ name: '../../evil\\shot.PNG', size: 2048, type: 'image/png' });
  assert.equal(ok.ok, true);
  assert.equal(ok.filename, 'shot.PNG');
  assert.equal(ok.mime_type, 'image/png');

  const tooBig = presentation.validateAttachmentCandidate({ name: 'dump.log', size: 16 * 1024 * 1024, type: 'text/plain' });
  assert.equal(tooBig.ok, false);
  assert.match(tooBig.reason, /15/i);

  const badExt = presentation.validateAttachmentCandidate({ name: 'payload.exe', size: 100, type: 'application/octet-stream' });
  assert.equal(badExt.ok, false);
  assert.match(badExt.reason, /type|supported/i);

  const emptyFile = presentation.validateAttachmentCandidate({ name: 'empty.txt', size: 0, type: 'text/plain' });
  assert.equal(emptyFile.ok, false);
});

test('renderAttachmentPills escapes filenames and formatPromptWithAttachments builds workspace path refs', () => {
  assert.equal(typeof presentation.renderAttachmentPills, 'function');
  assert.equal(typeof presentation.formatPromptWithAttachments, 'function');

  const html = presentation.renderAttachmentPills([
    { id: 'a1', filename: '<img src=x onerror=1>.png', size: 1536, status: 'saved', workspace_path: '.gnomcode/attachments/shot.png' },
    { id: 'a2', filename: 'trace.log', size: 204800, status: 'uploading' }
  ]);
  assert.doesNotMatch(html, /<img/);
  assert.match(html, /&lt;img src=x onerror=1&gt;\.png/);
  assert.match(html, /Saved|Uploaded/i);
  assert.match(html, /Uploading/i);

  const formatted = presentation.formatPromptWithAttachments('Check this crash log', [
    { id: 'a1', filename: 'shot.png', size: 100, status: 'saved', workspace_path: '.gnomcode/attachments/shot.png' }
  ]);
  assert.match(formatted, /Check this crash log/);
  assert.match(formatted, /\.gnomcode\/attachments\/shot\.png/);
});

test('attachments UI wires into Remote without raw hex CSS tokens', () => {
  const tsx = readFileSync(join(root, 'web/components/RemoteCompanion.tsx'), 'utf8');
  const css = readFileSync(join(root, 'web/app/gnomcode-console.css'), 'utf8');

  assert.match(tsx, /sp-attach-trigger/);
  assert.match(tsx, /upload_attachment/);
  assert.match(tsx, /Unsupported remote command/);
  assert.match(tsx, /validateAttachmentCandidate/);

  const marker = css.indexOf('/* Section C phone attachments */');
  assert.notEqual(marker, -1, 'Expected Section C phone attachments CSS block');
  const sectionCss = css.slice(marker);
  assert.doesNotMatch(sectionCss, /#[0-9a-fA-F]{3,8}\b/, 'Use --sp-* tokens only');
});
