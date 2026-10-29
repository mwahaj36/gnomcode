import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
const source = await readFile(new URL('../lib/console-presentation.js', import.meta.url));
const ui = await import(`data:text/javascript;base64,${source.toString('base64')}`);

test('session rows translate internal activity into public statuses and give empty views a next action', () => {
  for (const [state, label] of [['thinking', 'Running'], ['tool', 'Running'], ['awaiting_approval', 'Waiting'], ['idle', 'Idle'], ['error', 'Error']]) {
    const html = ui.renderSessionList([{ id: 's', agent: 'codex', state }], 's');
    assert.ok(html.includes(`codex · ${label}`), `${state}: ${html}`);
  }
  assert.match(ui.renderSessionList([], '', ''), /Launch|Start|Clear/);
  assert.match(ui.renderSessions([]), /Launch|Start|Clear/);
  assert.match(ui.renderSessionList([{ id: 'ended', state: 'ended' }]), /Session ended/);
});

test('estimated and missing monetary values retain explicit trust badges', () => {
  const html = ui.renderSummary({ report: { priced_turns: 1, total_cost_usd: 1.2, total_cache_savings_usd: .4 } });
  assert.match(html, /EST/);
  assert.match(ui.renderSummary(), /UNKNOWN/);
});
