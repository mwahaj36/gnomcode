import test from 'node:test';
import assert from 'node:assert/strict';
import { widgetSnapshot, publishWidgetSnapshot } from '../lib/widget-bridge.ts';

const quota = [
  { provider: 'codex', used_pct: 20, resets_at: '2026-11-29T00:00:00Z', basis: 'exact', observed_at: 'x' },
  { provider: 'codex', used_pct: 61, resets_at: '2026-12-01T00:00:00Z', basis: 'exact', observed_at: 'x' },
  { provider: 'claude-code', used_pct: 44, resets_at: null, basis: 'api', observed_at: 'x' },
];
const sessions = [
  { agent: 'claude-code', project_name: 'billing', home_group: 'needs_you', title: 'secret prompt' },
  { agent: 'codex', project_name: 'docs', home_group: 'running', title: 'other' },
];
const heatmap = { days: Array.from({ length: 200 }, (_, i) => ({ date: `d${i}`, level: i % 5, total_tokens: 9, cost_usd: 1 })), stats: { total_tokens: 1234, current_streak: 6, active_days: 9 } };

test('keeps the tightest window per provider and only needs-you sessions', () => {
  const snap = widgetSnapshot('2026-11-28T10:00:00Z', heatmap, quota, sessions);
  assert.deepEqual(snap.limits.map((l) => [l.provider, l.used_pct]), [['codex', 61], ['claude-code', 44]]);
  assert.deepEqual(snap.waiting, [{ agent: 'claude-code', project: 'billing' }]);
  assert.equal(snap.heatmap.days.length, 119);
  assert.deepEqual(snap.heatmap.stats, { total_tokens: 1234, current_streak: 6 });
});

test('never carries titles, costs or other content', () => {
  const json = JSON.stringify(widgetSnapshot('2026-11-28T10:00:00Z', heatmap, quota, sessions));
  assert.ok(!json.includes('secret prompt'));
  assert.ok(!json.includes('cost_usd'));
  assert.ok(!json.includes('observed_at'));
});

test('is a no-op outside the Android app', () => {
  assert.doesNotThrow(() => publishWidgetSnapshot('2026-11-28T10:00:00Z', heatmap, quota, sessions));
});
