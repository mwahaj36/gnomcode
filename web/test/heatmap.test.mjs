import test from 'node:test';
import assert from 'node:assert/strict';
import { renderHeatmap, renderHourStrip, renderActivity } from '../lib/console-presentation.js';

// 53 ISO weeks ending Thursday 2026-11-12, starting Monday 2025-11-10 (371 days minus the unlived Fri-Sun tail = 368).
const start = Date.UTC(2025, 10, 10);
const today = Date.UTC(2026, 10, 12);
const iso = ms => new Date(ms).toISOString().slice(0, 10);
const sample = (over = {}) => {
  const days = [];
  for (let ms = start; ms <= today; ms += 86400000) {
    const index = (ms - start) / 86400000;
    const active = index > 120 && index % 3 !== 0;
    days.push({ date: iso(ms), total_tokens: active ? 1000 * index : 0, cost_usd: active ? 1.5 : 0, cost_basis: active ? 'estimated' : 'unknown', turns: active ? 4 : 0, level: active ? 1 + (index % 4) : 0 });
  }
  return { from: iso(start), to: iso(today), first_recorded: '2026-03-02', days, stats: { total_tokens: 412000000, active_days: 143, busiest: { date: '2026-10-03', total_tokens: 90000000 }, current_streak: 9, longest_streak: 21 }, ...over };
};
const count = (html, pattern) => (html.match(pattern) || []).length;

test('every day is one cell, including days before history', () => {
  const data = sample();
  const html = renderHeatmap(data);
  const grid = html.match(/<div class="sp-heatmap-grid"[\s\S]*?<\/div>/)[0];
  assert.equal(count(grid, /<i class="sp-heat/g), data.days.length);
});

test('days before the first recording are invisible placeholders', () => {
  const html = renderHeatmap(sample());
  const before = sample().days.filter(day => day.date < '2026-03-02').length;
  assert.equal(count(html, /sp-heat sp-heat--before/g), before);
  assert.doesNotMatch(html, /sp-heat--before"[^>]*data-date/);
});

test('a quiet day reads no recorded activity and never zero tokens', () => {
  const html = renderHeatmap(sample());
  assert.match(html, /Mar 3: no recorded activity/);
  assert.doesNotMatch(html, /0 tokens/);
});

test('active day titles carry tokens, est. spend and turns', () => {
  const html = renderHeatmap(sample());
  assert.match(html, /title="Nov 12: [0-9.]+[KM]? tokens · \$1\.50 est\. · 4 turns"/);
  const free = renderHeatmap(sample({ days: [{ date: '2026-03-02', total_tokens: 500, cost_usd: 0, cost_basis: 'unknown', turns: 1, level: 1 }] }));
  assert.match(free, /500 tokens · 1 turn"/);
  assert.doesNotMatch(free, /est\./);
});

test('without any history it shows the empty state', () => {
  const html = renderHeatmap(sample({ first_recorded: null }));
  assert.match(html, /No activity recorded yet\./);
  assert.doesNotMatch(html, /sp-heatmap-grid/);
});

test('the mini variant keeps at most 12 weeks and drops stats, months and legend', () => {
  const html = renderHeatmap(sample(), { weeks: 12, stats: false, months: false, legend: false });
  assert.ok(count(html, /<i class="sp-heat/g) <= 84);
  assert.ok(count(html, /<i class="sp-heat/g) >= 78);
  assert.doesNotMatch(html, /sp-heatmap-stats|sp-heatmap-months|sp-heatmap-legend/);
  assert.match(html, /sp-heatmap--mini/);
});

test('month labels mark each month boundary once', () => {
  const html = renderHeatmap(sample());
  const labels = [...html.matchAll(/<span style="--col:(\d+)">([A-Z][a-z]{2})<\/span>/g)].map(match => match[2]);
  for (const month of ['Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov']) assert.ok(labels.includes(month), month);
  assert.ok(!labels.includes('Dec') && !labels.includes('Feb'), 'no labels over pre-history');
  labels.forEach((label, index) => assert.notEqual(label, labels[index + 1]));
});

test('stats come straight from the data', () => {
  const html = renderHeatmap(sample());
  assert.match(html, /<strong>412M<\/strong><span>tokens, past year<\/span>/);
  assert.match(html, /<strong>143<\/strong><span>active days<\/span>/);
  assert.match(html, /title="Longest streak: 21 days"><strong>9<\/strong><span>day streak/);
  assert.match(html, /<strong>Oct 3<\/strong><span>busiest day/);
  assert.match(html, /aria-label="412M tokens over 143 active days\. Details below\."/);
});

test('an empty year with no busiest day says None and escapes input', () => {
  const html = renderHeatmap(sample({ stats: { total_tokens: 0, active_days: 0, busiest: null, current_streak: 0, longest_streak: 0 }, days: [{ date: '"><img>', total_tokens: 1, cost_usd: 0, cost_basis: 'unknown', turns: 1, level: 99 }], first_recorded: '2020-01-01' }));
  assert.match(html, /<strong>None<\/strong>/);
  assert.doesNotMatch(html, /<img>/);
  assert.match(html, /data-level="0"/);
});

test('the grid carries no inline styles besides the month column variable', () => {
  const html = renderHeatmap(sample());
  assert.equal(count(html, /style="/g), count(html, /style="--col:\d+"/g));
});

test('the hour strip always has 24 cells and uses Rust levels', () => {
  const noon = new Date(2026, 10, 12, 12, 30).toISOString();
  const html = renderHourStrip({ daily: [{ date: noon, total_tokens: 2400000, cost_usd: 3.1, cost_basis: 'estimated', turns: 12, level: 3 }] });
  assert.equal(count(html, /<i class="sp-heat"/g), 24);
  assert.match(html, /data-level="3" title="12:00: 2\.4M tokens · \$3\.10 est\. · 12 turns"/);
  assert.match(html, /00:00: no recorded activity/);
  assert.match(html, /<span>00<\/span><span>06<\/span><span>12<\/span><span>18<\/span>/);
});

test('Today shows the strip inside the activity section; other periods show none', () => {
  const row = { date: new Date(2026, 10, 12, 9).toISOString(), total_tokens: 100, level: 4, chart_pct: 100, cost_usd: 0, cost_basis: 'unknown', turns: 1 };
  const today = renderActivity({ range_label: 'Today', total_tokens: 100, daily: [row] });
  assert.match(today, /sp-heatstrip/);
  assert.match(today, /sp-activity-breakdown/);
  assert.doesNotMatch(renderActivity({ range_label: 'Last 30 days', total_tokens: 100, daily: [{ ...row, date: '2026-11-12' }] }), /sp-heatstrip/);
});
