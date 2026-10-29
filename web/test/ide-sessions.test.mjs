import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';

const source = await readFile(new URL('../lib/console-presentation.js', import.meta.url));
const presentation = await import(`data:text/javascript;base64,${source.toString('base64')}`);

test('session sidebar searches titles and escapes session metadata', () => {
  const sessions = [
    { id: 'one" onclick="bad', title: '<Review auth>', agent: 'codex', state: 'idle', last_event_at: '2026-10-29T00:00:00Z' },
    { id: 'two', title: 'Other work', agent: 'claude-code', state: 'thinking' },
  ];
  const markup = presentation.renderSessionList(sessions, sessions[0].id, 'auth');
  assert.match(markup, /&lt;Review auth&gt;/);
  assert.match(markup, /aria-current="true"/);
  assert.match(markup, /data-action="load-chat"/);
  assert.doesNotMatch(markup, /Other work|<Review|id="one" onclick/);
  assert.match(presentation.renderSessionList(sessions, '', 'missing'), /No matching sessions/);
});

test('conversation view excludes tool and system output', () => {
  const markup = presentation.renderChatCards([
    { role: 'user', content: 'Review auth', created_at: '2026-10-29T00:00:00Z' },
    { role: 'assistant', content: 'I found a bug.', created_at: '2026-10-29T00:00:01Z' },
    { role: 'tool', content: 'Command output: secret', created_at: '2026-10-29T00:00:02Z' },
    { role: 'system', content: 'Task: metadata', created_at: '2026-10-29T00:00:03Z' },
  ]);
  assert.match(markup, /Review auth/);
  assert.match(markup, /I found a bug/);
  assert.doesNotMatch(markup, /Command output|Task: metadata/);
});

test('agent replies render Markdown while user messages retain literal text', () => {
  const markup = presentation.renderChatCards([
    { role: 'user', content: '**Keep this literal**', created_at: '2026-10-29' },
    { role: 'assistant', content: '# Result\n\n**Fixed** with `cargo test`.\n\n- First\n- Second\n\n```rust\nlet x = "<script>";\n```\n\n| Check | Result |\n| --- | --- |\n| Tests | Pass |', created_at: '2026-10-29' },
  ]);
  assert.match(markup, /<h1>Result<\/h1>/);
  assert.match(markup, /<strong>Fixed<\/strong>/);
  assert.match(markup, /<code>cargo test<\/code>/);
  assert.match(markup, /<ul><li>First<\/li><li>Second<\/li><\/ul>/);
  assert.match(markup, /<pre><code[^>]*>let x = &quot;&lt;script&gt;&quot;;/);
  assert.match(markup, /<th>Check<\/th>/);
  assert.match(markup, /<td>Pass<\/td>/);
  assert.match(markup, /\*\*Keep this literal\*\*/);
  assert.doesNotMatch(markup, /<script>/);
});

test('reply formatting never enables HTML or executable links', () => {
  const markup = presentation.renderChatCards([{ role: 'assistant', content: '<img src=x onerror=alert(1)>\n\n[bad](javascript:alert) [safe](https://example.com)\n\n```html\n**literal**\n```', created_at: '2026-10-29' }]);
  assert.doesNotMatch(markup, /<img|href="javascript:/);
  assert.match(markup, /href="https:\/\/example.com"/);
  assert.match(markup, /<code[^>]*>\*\*literal\*\*/);
});

test('nested steps and multiline paragraphs retain their structure', () => {
  const markup = presentation.renderChatCards([{role:'assistant', content:'First line\nsecond line\n\n1. Run checks\n   - Unit tests\n   - Browser tests\n2. Review changes', created_at:'2026-10-29'}]);
  assert.match(markup, /<p>First line<br>second line<\/p>/);
  assert.match(markup, /<ol><li>Run checks<ul><li>Unit tests<\/li><li>Browser tests<\/li><\/ul><\/li><li>Review changes<\/li><\/ol>/);
});
