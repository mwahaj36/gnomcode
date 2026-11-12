import test from 'node:test';
import assert from 'node:assert/strict';
import { renderMeterRow, renderPoolGroup, renderProviderCard, renderProviderCards, renderQuota, renderConversationLimits, updateProviderCards } from '../lib/console-presentation.js';

const future = hours => new Date(Date.now() + hours * 3600000).toISOString();
const seen = new Date(Date.now() - 120000).toISOString();
const win = (over = {}) => ({ provider: 'claude-code', limit_id: 'claude', window: 'five_hour', basis: 'api', state: 'ok', remaining_pct: 42, resets_at: future(2.2), observed_at: seen, stale: false, label: 'Session', pool: null, window_label: 'Session', pace: null, ...over });
const card = (over = {}) => ({ provider: 'claude-code', display_name: 'Claude Code', plan: 'max', state: 'ok', windows: [win()], headline_window: 0, updated_at: seen, source_label: 'Provider API', enable_hint: null, enable_command: null, cost: null, ...over });

test('an ok window shows percent left and a reset countdown', () => {
  const html = renderMeterRow(win());
  assert.match(html, /42% left/);
  assert.match(html, /Resets in 2h/);
  assert.match(html, /sp-meter-row--ok/);
  assert.match(html, /role="meter"[^>]*aria-valuenow="42"/);
  assert.match(html, /title="Provider API\. Seen 2m ago\."/);
});

test('an exhausted window reads Exhausted and says when it comes back', () => {
  const html = renderMeterRow(win({ state: 'exhausted', remaining_pct: 0 }));
  assert.match(html, /Exhausted/);
  assert.match(html, /sp-meter-row--exhausted/);
  assert.match(html, /Back in/);
});

test('a window whose reset passed awaits the next reading', () => {
  const html = renderMeterRow(win({ state: 'reset_awaiting', remaining_pct: null }));
  assert.match(html, /Awaiting next reading/);
  assert.match(html, /sp-meter-row--reset_awaiting/);
});

test('the pace tick only exists when Rust sent a pace', () => {
  assert.doesNotMatch(renderMeterRow(win({ pace: null })), /sp-meter-pace/);
  const html = renderMeterRow(win({ pace: { pace_remaining_pct: 55, delta_pct: -13, verdict: 'ahead', runs_out_at: null } }));
  assert.match(html, /sp-meter-pace" style="--pace:55%"/);
  assert.match(html, /13% ahead of pace/);
  assert.match(renderMeterRow(win({ pace: { pace_remaining_pct: 50, delta_pct: 12, verdict: 'behind', runs_out_at: null } })), /12% in reserve/);
  assert.match(renderMeterRow(win({ pace: { pace_remaining_pct: 50, delta_pct: 1, verdict: 'on_pace', runs_out_at: null } })), /On pace/);
  assert.match(renderMeterRow(win({ pace: { pace_remaining_pct: 50, delta_pct: -20, verdict: 'ahead', runs_out_at: future(3) } })), /Runs out in /);
});

test('provider cards are collapsed details with a summary overview', () => {
  const html = renderProviderCard(card());
  assert.match(html, /^<details class="sp-card sp-provider-card"/);
  assert.doesNotMatch(html.split('>')[0], /\sopen/);
  assert.match(html, /<summary>/);
  assert.match(html, /sp-chip sp-chip--ok">OK</);
  assert.match(html, /sp-strip/);
  assert.match(html, /sp-chevron/);
  assert.match(html, /data-key="provider:claude-code"/);
});

test('the summary line follows headline_window, not the first window', () => {
  const windows = [win({ label: 'Session', window_label: 'Session', remaining_pct: 90 }), win({ window: 'weekly', label: 'Weekly', window_label: 'Weekly', state: 'low', remaining_pct: 8 })];
  const summary = renderProviderCard(card({ windows, headline_window: 1, state: 'low' })).match(/<summary>[\s\S]*<\/summary>/)[0];
  assert.match(summary, /Weekly <b>8% left<\/b>/);
  assert.doesNotMatch(summary, /Session <b>/);
  assert.match(summary, /sp-chip--low">Low</);
  assert.equal((summary.match(/<i class="is-/g) || []).length, 2, 'one strip bar per window');
  const exhausted = renderProviderCard(card({ windows: [win({ state: 'exhausted', remaining_pct: 0 })], state: 'exhausted' })).match(/<summary>[\s\S]*<\/summary>/)[0];
  assert.match(exhausted, /<b>exhausted<\/b> · back in/);
  assert.match(exhausted, /sp-chip--exhausted">Out</);
});

test('multi-pool providers group compact rows under pool names', () => {
  const pools = ['Claude and GPT models', 'Gemini Models', 'Gemini 4 Argon'];
  const windows = pools.flatMap(pool => [win({ pool, label: `${pool} · Weekly`, window: 'weekly', window_label: 'Weekly' })]);
  const html = renderProviderCard(card({ provider: 'antigravity', display_name: 'Antigravity', windows }));
  assert.equal((html.match(/class="sp-pool"/g) || []).length, 3);
  for (const pool of pools) assert.match(html, new RegExp(`sp-pool-name">${pool}<`));
  assert.equal((html.match(/class="sp-mini sp-mini--/g) || []).length, 3);
  assert.doesNotMatch(html, /sp-meter-row/);
  assert.match(renderPoolGroup([win({ pool: 'P', state: 'exhausted', remaining_pct: 0 })]), /sp-mini--exhausted[\s\S]*<b>Out<\/b>/);
});

test('a source issue replaces the reset line in the summary and the affected meter', () => {
  const issue = 'Claude sign-in expired; run `claude` in a terminal to refresh it';
  const html = renderProviderCard(card({ source_issue: issue, windows: [win({ source_issue: issue, stale: true, observed_at: new Date(Date.now() - 9 * 3600000).toISOString() })] }));
  const summary = html.match(/<summary>[\s\S]*<\/summary>/)[0];
  assert.match(summary, /sp-provider-line--issue[^>]*>Claude sign-in expired/);
  assert.match(html, /sp-meter-row--issue/);
  assert.match(html, /<div class="sp-meter-bottom"><span>Claude sign-in expired[^<]*last seen 9h ago/);
});

test('an empty provider offers the enable command, or only the hint when there is none', () => {
  const withCommand = renderProviderCard(card({ windows: [], state: 'unknown', headline_window: null, enable_hint: 'Limits are off for Claude Code.', enable_command: 'gnomcode limits enable claude' }));
  assert.match(withCommand, /data-copy="gnomcode limits enable claude"/);
  assert.match(withCommand, /Limits are off for Claude Code\./);
  assert.match(withCommand, /sp-chip--unknown">No data</);
  const without = renderProviderCard(card({ provider: 'codex', display_name: 'Codex', windows: [], state: 'unknown', enable_hint: 'Appears after your next Codex request.' }));
  assert.doesNotMatch(without, /data-copy|<button/);
  assert.match(without, /Appears after your next Codex request\./);
});

test('the cost footer says est. and is omitted without cost or in the compact variant', () => {
  const withCost = renderProviderCard(card({ cost: { today_usd: 1.2, last_30_days_usd: 48, basis: 'estimated' } }));
  assert.match(withCost, /Today \$1\.20 est\. · 30 days \$48\.00 est\./);
  assert.match(withCost, /Provider API · Seen 2m ago/);
  assert.doesNotMatch(renderProviderCard(card({ cost: { today_usd: 1.2, last_30_days_usd: 48, basis: 'estimated' } }), { cost: false }), /Today \$/);
  assert.doesNotMatch(renderProviderCard(card({ cost: { today_usd: 1.2, last_30_days_usd: 48, basis: 'estimated' } }), { compact: true }), /sp-provider-foot|sp-meter-bottom/);
});

test('output has no emoji and escapes every interpolated string', () => {
  const evil = '<b>x</b>';
  const html = renderProviderCards([card({ display_name: evil, plan: evil, source_label: evil, enable_hint: evil, windows: [win({ label: evil, window_label: evil, source_issue: evil })] }), card({ provider: 'p"><i>', display_name: evil, windows: [], enable_hint: evil, enable_command: evil })], {});
  assert.match(html, /&lt;b&gt;x&lt;\/b&gt;/);
  assert.doesNotMatch(html, /<b>x<\/b>|<i>"/);
  assert.doesNotMatch(html, /\p{Extended_Pictographic}/u);
  assert.doesNotMatch(renderConversationLimits([win({ label: evil })]), /<b>x<\/b>/);
});

test('conversation limits are chips whose state comes from Rust', () => {
  const html = renderConversationLimits([win({ label: 'Weekly', state: 'low', remaining_pct: 12 }), { label: 'Old', remaining_pct: 50, basis: 'exact' }]);
  assert.match(html, /sp-chip sp-chip--low sp-conversation-limit" title="Provider API\./);
  assert.match(html, /Weekly 12% left/);
  assert.match(html, /sp-chip--unknown[^>]*>Old 50% left/, 'no state from an older desktop renders plainly');
});

test('raw pools from the web console group into one card per provider', () => {
  const html = renderQuota([win(), win({ window: 'weekly', label: 'Weekly', window_label: 'Weekly', state: 'low', remaining_pct: 5 }), win({ provider: 'codex', limit_id: 'codex' })], seen);
  assert.equal((html.match(/<details /g) || []).length, 2);
  assert.match(html, /<summary>[\s\S]*?Weekly <b>5% left<\/b>/, 'the worst Rust state is the headline');
  assert.match(renderQuota([], null), /No measured quota pools/);
});

function fakeElement() {
  const element = {
    cards: [], listeners: [],
    contains: () => false,
    addEventListener(type, handler, capture) { this.listeners.push({ type, handler, capture }); },
    querySelector(selector) { return selector.includes('details[data-provider]') ? this.cards[0] || null : null; },
    querySelectorAll(selector) {
      if (selector === 'details[open]') return this.cards.filter(card => card.open);
      if (selector === 'details' || selector.includes('details[data-provider]')) return this.cards;
      return [];
    },
    set innerHTML(markup) { this.cards = [...markup.matchAll(/<details [^>]*data-key="([^"]+)" data-provider="([^"]+)"/g)].map(match => ({ open: false, dataset: { key: match[1], provider: match[2] }, querySelector: () => null })); },
  };
  return element;
}

test('open state survives re-render and persists in localStorage, closed by default', () => {
  const store = new Map();
  globalThis.localStorage = { getItem: key => store.get(key) ?? null, setItem: (key, value) => store.set(key, value) };
  globalThis.document = { activeElement: null, querySelectorAll: () => [] };
  const element = fakeElement();
  const list = [card()];
  updateProviderCards(element, list);
  assert.equal(element.cards[0].open, false, 'collapsed by default');
  element.cards[0].open = true;
  updateProviderCards(element, [card({ windows: [win({ remaining_pct: 41 })] })]);
  assert.equal(element.cards[0].open, true, 'an open card stays open across the poll re-render');
  const listener = element.listeners.find(entry => entry.type === 'toggle');
  assert.ok(listener?.capture, 'toggle does not bubble, so it is observed in the capture phase');
  listener.handler({ target: { dataset: { provider: 'claude-code' }, open: true, matches: () => true } });
  assert.deepEqual(JSON.parse(store.get('gnomcode-provider-open')), { 'claude-code': true });
  const reloaded = fakeElement();
  updateProviderCards(reloaded, list);
  assert.equal(reloaded.cards[0].open, true, 'a stored open card opens on the next page load');
  globalThis.localStorage = { getItem() { throw new Error('blocked'); }, setItem() { throw new Error('blocked'); } };
  assert.doesNotThrow(() => updateProviderCards(fakeElement(), list));
});
