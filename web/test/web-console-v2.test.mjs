import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { renderActivity, renderConversationLimitCard, renderPeriodTiles, renderValue } from '../lib/console-presentation.js';

const read = path => readFileSync(new URL(path, import.meta.url), 'utf8');
const client = read('../app/console/Client.tsx');
const css = read('../app/gnomcode-console.css');

test('web console uses the shared presenters, not the retired panels', () => {
  for (const name of ['updateHomeSessions', 'updateTodayTiles', 'updateQuota']) assert.match(client, new RegExp(name));
  for (const name of ['SummaryPanel', 'OngoingPanel', 'updateSummary', 'updateOngoing']) assert.doesNotMatch(client, new RegExp(name));
});

test('web rail has four places and old views alias', () => {
  assert.match(client, /\[\["overview", "Home"\], \["chat", "Workspace"\], \["usage", "Usage"\], \["settings", "Settings"\]\]/);
  assert.match(client, /limits: "usage", projects: "usage", automations: "settings"/);
});

test('web rail never expands on hover or focus', () => {
  for (const line of css.split('\n').filter(rule => rule.includes('sp-web-app') && rule.includes('sp-console-nav') && /:hover|focus-within/.test(rule))) {
    assert.doesNotMatch(line, /width|padding-left|opacity/, line.slice(0, 120));
  }
  assert.doesNotMatch(css, /@media\(hover:hover\) and \(pointer:fine\)\{\s*\.sp-web-app/);
});

test('the Usage totals are not repeated beside the grid', () => {
  const report = { range_label: 'Today', total_tokens: 10, daily: [{ date: '2026-11-13T01:00:00Z', total_tokens: 10, turns: 1 }] };
  assert.match(renderActivity(report), /Recorded tokens/);
  assert.doesNotMatch(renderActivity(report, { totals: false }), /Recorded tokens/);
});

test('period tiles say Unknown without pricing', () => {
  const out = renderPeriodTiles({ range_label: 'This month', total_tokens: 1200, cost_basis: 'unknown', total_cost_usd: 0 });
  assert.match(out, /Totals for this month/);
  assert.doesNotMatch(out, /\$0\.00/);
});

test('remote limits sheet renders one collapsed card with Rust fields', () => {
  const out = renderConversationLimitCard([{ label: '5h', window_label: 'Session', remaining_pct: 40, basis: 'exact', state: 'ok', resets_at: '2099-01-01T00:00:00Z' }], 'codex');
  assert.equal((out.match(/<details /g) || []).length, 1);
  assert.doesNotMatch(out, /<details[^>]* open/);
  assert.match(out, /Session/);
  assert.match(renderConversationLimitCard([], 'codex'), /Limits unavailable/);
});

test('subscription value renders labeled estimate tiles with a combined tile', () => {
  const row = (provider, spend, plan, multiplier) => ({ provider, spend_usd: spend, plan_usd: plan, multiplier });
  const html = renderValue([row('Claude Code', 1423.94, 53.33, 26.7), row('OpenAI Codex', 0, 10, 0), row('Google Antigravity', 80, 20, 4)], row('All', 1503.94, 73.33, 20.5));
  assert.match(html, /\$1,423\.94 value · \$53\.33 plan/);
  assert.match(html, /26\.7x<span class="sp-tile-unit">est\./);
  assert.equal(html.match(/sp-tile--value/g).length, 3 + 0);
  assert.match(html, /All providers/);
  assert.doesNotMatch(html, /OpenAI Codex|text-red|text-green/);
  assert.doesNotMatch(renderValue([row('Claude Code', 10, 5, 2)], row('All', 10, 5, 2)), /All providers/);
  assert.match(renderValue([], null), /No subscription usage/);
});
