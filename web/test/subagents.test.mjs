import assert from 'node:assert/strict';
import test from 'node:test';
import { renderHomeSessions, renderSessionList, renderSubagents } from '../lib/console-presentation.js';

const recent = new Date(Date.now() - 2 * 60000).toISOString();
const parent = over => ({
  id: 'claude-code:p', title: 'Parent', project_name: 'fleethud', agent: 'claude-code', model: 'opus-5', state: 'idle',
  home_group: 'running', last_event_at: recent, subagent_count: 3, subagents_ongoing: 1,
  subagents: [
    { id: 'claude-code:agent-a', title: 'Subagent', model: 'sonnet', state: 'tool', presence: 'running', is_ongoing: true, home_group: 'running', last_event_at: recent },
    { id: 'claude-code:agent-b', title: 'Subagent', model: 'Unspecified', state: 'idle', presence: 'inactive', is_ongoing: false, home_group: 'recent', last_event_at: recent },
  ],
  ...over,
});

test('home row shows a subagent count badge and an expandable nested list', () => {
  const html = renderHomeSessions([parent()]);
  assert.match(html, /fleethud · Claude Code · opus-5 · 3 subagents/);
  assert.match(html, /<details class="sp-subagents" data-key="sub:claude-code:p"><summary>3 subagents · 1 active<\/summary>/);
  assert.match(html, /data-action="open-home-session" data-arg="claude-code:agent-a"/);
  assert.match(html, /1 older not shown/);
  assert.equal((html.match(/class="sp-session-row"/g) || []).length, 1, 'subagents are not top-level rows');
});

test('workspace list nests subagents under the parent and opens them as chats', () => {
  const html = renderSessionList([parent({ subagent_count: 2 })], 'claude-code:p');
  assert.match(html, /<\/button><details class="sp-subagents"/);
  assert.match(html, /data-action="load-chat" data-arg="claude-code:agent-b"/);
  assert.doesNotMatch(html, /older not shown/);
});

test('sessions without subagents render no group and singular copy is used', () => {
  assert.equal(renderSubagents({ id: 'x' }), '');
  assert.match(renderSubagents(parent({ subagent_count: 1, subagents_ongoing: 0, subagents: [parent().subagents[1]] })), /<summary>1 subagent<\/summary>/);
});
