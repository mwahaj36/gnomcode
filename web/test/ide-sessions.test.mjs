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
