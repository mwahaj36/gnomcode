import test from 'node:test';
import assert from 'node:assert/strict';
import { renderHomeSessions, renderTodayTiles, renderHeatmap } from '../lib/console-presentation.js';

const session = (id, group, over = {}) => ({ id, title: `Session ${id}`, project_name: 'fleethud', agent: 'Claude Code', model: 'opus-5', home_group: group, last_event_at: new Date(Date.now() - 12 * 60000).toISOString(), context_fill_pct: 62, ...over });
const count = (html, pattern) => (html.match(pattern) || []).length;

test('needs you rows come before running and recent', () => {
  const html = renderHomeSessions([session('r', 'recent'), session('n', 'needs_you'), session('u', 'running')]);
  assert.ok(html.indexOf('Needs you') < html.indexOf('>Running') && html.indexOf('>Running') < html.indexOf('>Recent'));
  assert.match(html, /sp-session-dot--needs_you/);
  assert.match(html, /sp-session-dot--running/);
});

test('empty groups are omitted and counts are shown', () => {
  const html = renderHomeSessions([session('a', 'running'), session('b', 'running')]);
  assert.doesNotMatch(html, /Needs you/);
  assert.doesNotMatch(html, />Recent/);
  assert.match(html, /Running <span>\(2\)<\/span>/);
});

test('recent is capped at six rows', () => {
  const html = renderHomeSessions(Array.from({ length: 9 }, (_, index) => session(String(index), 'recent')));
  assert.equal(count(html, /class="sp-session-row"/g), 6);
  assert.match(html, /data-action="view:sessions"/);
});

test('a session without a group falls back to recent', () => {
  const html = renderHomeSessions([session('a', undefined)]);
  assert.match(html, />Recent/);
});

test('rows open the conversation and show context and age', () => {
  const html = renderHomeSessions([session('abc', 'running')]);
  assert.match(html, /data-action="open-home-session" data-arg="abc"/);
  assert.match(html, /Context 62% full/);
  assert.match(html, /--fill:62%/);
  assert.match(html, />12m</);
  assert.doesNotMatch(renderHomeSessions([session('abc', 'running', { context_fill_pct: null })]), /Context \d+% full/);
});

test('zero sessions shows one line and a primary action', () => {
  const html = renderHomeSessions([]);
  assert.match(html, /sp-empty/);
  assert.match(html, /No sessions yet\./);
  assert.match(html, /class="btn-primary"[^>]*data-action="home-new-session">New session</);
  assert.equal(count(html, /sp-session-row/g), 0);
});

test('session text is escaped', () => {
  const html = renderHomeSessions([session('x', 'running', { title: '<img src=x>' })]);
  assert.doesNotMatch(html, /<img/);
});

test('today tiles show tokens, estimated spend and active sessions', () => {
  const html = renderTodayTiles({ report: { total_tokens: 2400000, total_cost_usd: 3.1, cost_basis: 'estimated', priced_turns: 4, unpriced_turns: 0 }, activeCount: 3 });
  assert.match(html, /Tokens today<\/span><strong class="sp-tile-value">2\.4M/);
  assert.match(html, /Est\. spend today<\/span><strong class="sp-tile-value">\$3\.10/);
  assert.match(html, /API equivalent/);
  assert.match(html, /Active sessions<\/span><strong class="sp-tile-value">3</);
  assert.equal(count(html, /class="sp-tile"/g), 3);
});

test('unavailable data reads Unknown, not zero', () => {
  const html = renderTodayTiles({});
  assert.equal(count(html, />Unknown</g), 3);
});

test('the mini grid is titled by its length', () => {
  const html = renderHeatmap({ from: '2026-11-01', to: '2026-11-02', first_recorded: '2026-11-01', days: [{ date: '2026-11-01', total_tokens: 1, cost_usd: 0, cost_basis: 'unknown', turns: 1, level: 1 }, { date: '2026-11-02', total_tokens: 0, cost_usd: 0, cost_basis: 'unknown', turns: 0, level: 0 }], stats: null }, { weeks: 12, stats: false, legend: false, months: false });
  assert.match(html, /Last 12 weeks/);
});
