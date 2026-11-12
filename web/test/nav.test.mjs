import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { renderProviderSources, renderSessionList } from '../lib/console-presentation.js';

const read = path => readFileSync(new URL(path, import.meta.url), 'utf8');
const html = read('../../crates/daemon/src/console.html');
const app = read('../../crates/daemon/src/console-app.js');
const css = read('../app/gnomcode-console.css');

test('rail has exactly four places', () => {
  const views = [...html.matchAll(/class="tab-btn[^"]*" id="tab-btn-[a-z]+" data-view="([a-z]+)"/g)].map(match => match[1]);
  assert.deepEqual(views, ['overview', 'sessions', 'usage', 'settings']);
  assert.doesNotMatch(html, /id="view-(projects|automations)"/);
});

test('old views redirect', () => {
  assert.match(app, /VIEW_ALIASES = \{ limits: 'usage', projects: 'usage', automations: 'settings' \}/);
});

test('period selector shows only on Usage', () => {
  assert.match(html, /class="timeline-bar" data-local-view="usage"/);
});

test('local rail never expands on hover or focus', () => {
  const rules = css.split('\n').filter(line => line.includes('sp-native-console') && line.includes('sp-console-nav'));
  assert.ok(rules.length > 0);
  for (const line of rules.filter(rule => /:hover|focus-within|:has\(/.test(rule))) assert.doesNotMatch(line, /width|padding-left|rail-w|display/, line.slice(0, 120));
  assert.match(css, /body\.sp-native-console\[data-rail="expanded"\]\{--rail-w:208px\}/);
});

test('rail toggle persists in try/catch storage', () => {
  assert.match(app, /gnomcode-rail-expanded/);
  assert.match(app, /function storedFlag[^\n]*try \{[^\n]*catch/);
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
