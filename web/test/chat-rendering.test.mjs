import test from 'node:test';
import assert from 'node:assert/strict';
import { renderChatCards, renderCodeBlock, highlightCode } from '../lib/console-presentation.js';

const at = '2026-11-14T10:00:00Z';

test('fenced code renders label, copy, wrap and escaped highlighting', () => {
  const html = renderChatCards([{ role: 'assistant', created_at: at, content: '```js\nconst a = "<b>"; // x\n```' }]);
  assert.match(html, /<span class="sp-code-lang">js<\/span>/);
  assert.match(html, /data-code-wrap/);
  assert.match(html, /data-chat-copy="const a = &quot;&lt;b&gt;&quot;; \/\/ x"/);
  assert.match(html, /sp-hl-keyword">const/);
  assert.doesNotMatch(html, /<b>/);
});

test('diffs render as added and removed lines', () => {
  const html = renderCodeBlock('@@ -1 +1 @@\n-old\n+new');
  assert.match(html, /sp-code-lang">diff/);
  assert.match(html, /sp-diff-del">-old/);
  assert.match(html, /sp-diff-add">\+new/);
  assert.equal(highlightCode('<x>', 'text'), '&lt;x&gt;');
});

test('harness system events render as muted chips, never user bubbles', () => {
  const html = renderChatCards([
    { role: 'system', status: 'task_notification', created_at: at, content: 'Background task finished: Sample\n\nAll good' },
    { role: 'system', status: 'command', created_at: at, content: 'Ran /compact' },
  ]);
  assert.match(html, /<details class="sp-chat-system"[^>]*data-kind="task_notification"/);
  assert.match(html, /<div class="sp-chat-system"[^>]*data-kind="command"/);
  assert.doesNotMatch(html, /sp-chat-user/);
});

test('external images are not loaded and data images become thumbnails', () => {
  const html = renderChatCards([{ role: 'assistant', created_at: at, content: '![x](https://example.com/a.png) ![y](data:image/png;base64,AAAA)' }]);
  assert.doesNotMatch(html, /<img[^>]+example\.com/);
  assert.match(html, /External image not loaded: x \(opens example\.com\)/);
  assert.match(html, /data-image-open="data:image\/png;base64,AAAA"/);
});

test('attachment references become thumbnail placeholders and messages carry copy', () => {
  const html = renderChatCards([{ role: 'user', created_at: at, content: 'See this\n[Attachment: .gnomcode/attachments/s.png]' }]);
  assert.match(html, /data-attachment-path=".gnomcode\/attachments\/s.png"/);
  assert.match(html, /aria-label="Copy (?:your |assistant )?message"/);
});
