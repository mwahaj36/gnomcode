import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { renderProviderSources, renderSessionList } from '../lib/console-presentation.js';

const read = path => readFileSync(new URL(path, import.meta.url), 'utf8');
const css = read('../app/gnomcode-console.css');

test('local rail never expands on hover or focus', () => {
  const rules = css.split('\n').filter(line => line.includes('gc-console') && line.includes('sp-console-nav'));
  assert.ok(rules.length > 0);
  for (const line of rules.filter(rule => /:hover|focus-within|:has\(/.test(rule))) assert.doesNotMatch(line, /width|padding-left|rail-w|display/, line.slice(0, 120));
  assert.match(css, /\.gc-console\[data-navigation=expanded\]\{--gc-rail:176px\}/);
});

test('provider sources show the Rust label and enable command', () => {
  const out = renderProviderSources([
    { provider: 'claude', display_name: 'Claude Code', source_label: 'Status line', windows: [] },
    { provider: 'antigravity', display_name: 'Antigravity', source_label: 'Off', enable_command: 'gnomcode limits enable antigravity', windows: [] },
  ]);
  assert.match(out, /Status line/);
  assert.match(out, /data-copy="gnomcode limits enable antigravity"/);
  assert.equal((out.match(/sp-copy/g) || []).length, 1);
});

test('conversation list groups only when something needs attention', () => {
  const rows = [{ id: 'a', agent: 'claude', title: 'A', state: 'idle', home_group: 'recent' }, { id: 'b', agent: 'claude', title: 'B', state: 'running', home_group: 'running' }];
  const grouped = renderSessionList(rows);
  assert.ok(grouped.indexOf('Running') < grouped.indexOf('>A<'));
  assert.doesNotMatch(renderSessionList([rows[0]]), /sp-group-label/);
});
