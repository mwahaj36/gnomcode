const test = require('node:test'), assert = require('node:assert/strict');
const { formatTrayMenu, formatTrayTooltip, plain } = require('../dist/tray-menu');

const NOW = new Date('2026-11-12T10:00:00Z');
const OPTIONS = { timeZone: 'UTC' };
const win = (over) => ({ label: 'Weekly', pool: null, window_label: 'Weekly', state: 'ok', remaining_pct: 98, resets_at: '2026-11-12T08:00:00Z', reset_passed: false, stale: false, ...over });

// Mockup sample: Claude Code ok, Codex out, Antigravity with one pool out.
const claude = { provider: 'claude-code', display_name: 'Claude Code', headline_window: 1, updated_at: '2026-11-12T09:59:00Z', windows: [
  win({ label: 'Session', window_label: 'Session', state: 'reset_awaiting', remaining_pct: null, resets_at: '2026-11-12T09:00:00Z', reset_passed: true }),
  win({ resets_at: '2026-11-19T08:00:00Z' }),
] };
const codex = { provider: 'codex', display_name: 'Codex', headline_window: 0, updated_at: '2026-11-12T09:58:00Z', windows: [
  win({ state: 'exhausted', remaining_pct: 0, resets_at: '2026-11-16T13:20:00Z' }),
] };
const antigravity = { provider: 'antigravity', display_name: 'Antigravity', headline_window: 0, updated_at: '2026-11-12T09:57:00Z', windows: [
  win({ pool: 'Gemini Pro', label: 'Gemini Pro Weekly', state: 'exhausted', remaining_pct: 0, resets_at: '2026-11-16T13:20:00Z' }),
  win({ pool: 'Gemini Flash', label: 'Gemini Flash Weekly', remaining_pct: 80, resets_at: '2026-11-15T08:00:00Z' }),
] };
const empty = { provider: 'codex', display_name: 'Codex', headline_window: null, updated_at: null, windows: [], enable_command: 'gnomcode limits enable codex' };
const running = { id: 'r1', title: 'Console UI overhaul plan', agent: 'claude-code', home_group: 'running', last_event_at: '2026-11-12T09:58:00Z' };

const labels = (items) => items.map(item => item.type === 'separator' ? '---' : item.label);

test('menu text for the mockup sample data', () => {
  const menu = formatTrayMenu([claude, codex, antigravity], [running], NOW, OPTIONS);
  assert.deepEqual(labels(menu), [
    'Running (1)', 'Console UI overhaul plan · Claude Code', '---',
    'Claude Code    Weekly 98% left · Thu',
    'Codex    Weekly out · back Mon 1:20 PM',
    'Antigravity    1 pool out',
    '---', 'Updated 1m ago', 'New session', 'Open Gnomcode', '---', 'Quit Gnomcode',
  ]);
  assert.equal(menu[0].enabled, false);
});

test('submenu lists windows disabled and ends with Open limits', () => {
  const [, , , claudeItem, codexItem] = formatTrayMenu([claude, codex], [running], NOW, OPTIONS);
  assert.deepEqual(labels(claudeItem.submenu), ['Session: reset, awaiting reading', 'Weekly: 98% left, resets Thu 8:00 AM', '---', 'Open limits']);
  assert.ok(claudeItem.submenu.slice(0, 2).every(item => item.enabled === false));
  assert.deepEqual(codexItem.submenu[0].label, 'Weekly: exhausted, back Mon 1:20 PM');
  assert.deepEqual(claudeItem.submenu.at(-1).action, { kind: 'open-view', view: 'usage' });
});

test('provider without data shows Limits off and a single copy action', () => {
  const menu = formatTrayMenu([empty], [], NOW, OPTIONS);
  assert.equal(menu[0].label, 'Codex    Limits off');
  assert.deepEqual(menu[0].submenu, [{ label: 'Copy enable command', action: { kind: 'copy', text: 'gnomcode limits enable codex' } }]);
  assert.ok(!labels(menu).some(text => String(text).startsWith('Updated')));
});

test('multi-pool providers group windows under disabled pool headers', () => {
  const menu = formatTrayMenu([antigravity], [], NOW, OPTIONS);
  assert.deepEqual(labels(menu[0].submenu), [
    'Gemini Pro', 'Weekly: exhausted, back Mon 1:20 PM', '---',
    'Gemini Flash', 'Weekly: 80% left, resets Sun 8:00 AM', '---', 'Open limits',
  ]);
});

test('source issues appear as a disabled item in the provider submenu', () => {
  const issue = { ...claude, source_issue: 'Sign in again with the Claude CLI' };
  const submenu = formatTrayMenu([issue], [], NOW, OPTIONS)[0].submenu;
  const item = submenu.find(entry => entry.label === 'Sign in again with the Claude CLI');
  assert.equal(item.enabled, false);
  assert.equal(submenu.at(-1).label, 'Open limits');
});

test('needs-you sessions come first, longest waiting first, capped at three', () => {
  const waiting = (id, at) => ({ id, title: `Approve ${id}`, agent: 'codex', home_group: 'needs_you', last_event_at: at });
  const sessions = [running, waiting('c', '2026-11-12T09:30:00Z'), waiting('a', '2026-11-12T09:10:00Z'), waiting('b', '2026-11-12T09:20:00Z'), waiting('d', '2026-11-12T09:40:00Z'),
    { id: 'old', title: 'Done', agent: 'codex', home_group: 'recent', last_event_at: '2026-11-11T09:00:00Z' }];
  const menu = formatTrayMenu([], sessions, NOW, OPTIONS);
  assert.deepEqual(labels(menu).slice(0, 8), [
    'Needs you (4)', 'Approve a · Codex', 'Approve b · Codex', 'Approve c · Codex', 'and 1 more',
    'Running (1)', 'Console UI overhaul plan · Claude Code', '---',
  ]);
  assert.deepEqual(menu[1].action, { kind: 'focus-session', id: 'a' });
  assert.ok(!labels(menu).includes('Done'));
});

test('long titles are truncated and labels stay short', () => {
  const long = { ...running, title: 'x'.repeat(200) };
  const label = formatTrayMenu([], [long], NOW, OPTIONS)[1].label;
  assert.ok(label.startsWith('x'.repeat(30)) && label.includes('... · Claude Code'));
  assert.ok(label.length < 80);
});

test('every label is ASCII except the middle dot', () => {
  const messy = { ...claude, display_name: 'Cläude — Code', source_issue: 'Sign‑in “expired” 😀' };
  const everything = formatTrayMenu([messy, codex, antigravity, empty], [{ ...running, title: 'Café … fix' }], NOW, OPTIONS);
  const walk = (items) => items.flatMap(item => item.type === 'separator' ? [] : [item.label, ...(item.submenu ? walk(item.submenu) : [])]);
  for (const text of walk(everything)) assert.match(text, /^[\x20-\x7e·]*$/, text);
  assert.match(formatTrayTooltip([messy, codex]), /^[\x20-\x7e·]*$/);
  assert.equal(plain('a–b'), 'a-b');
});

test('daemon unavailable fails open to Open and Quit', () => {
  const menu = formatTrayMenu(null, null, NOW, OPTIONS);
  assert.deepEqual(labels(menu), ['Daemon unavailable', '---', 'Open Gnomcode', '---', 'Quit Gnomcode']);
  assert.equal(menu[0].enabled, false);
  assert.equal(formatTrayTooltip(null), 'Gnomcode: daemon unavailable');
});

test('overview failure alone keeps sessions and says limits are unavailable', () => {
  const labelsOut = labels(formatTrayMenu(null, [running], NOW, OPTIONS));
  assert.ok(labelsOut.includes('Limits unavailable') && labelsOut.includes('Running (1)'));
});

test('tooltip names the tightest limit and never exceeds 127 characters', () => {
  assert.equal(formatTrayTooltip([claude, codex]), 'Gnomcode · Claude weekly 98% · Codex out');
  assert.equal(formatTrayTooltip([empty]), 'Gnomcode');
  const many = Array.from({ length: 30 }, (_, index) => ({ ...claude, display_name: `Provider number ${index}` }));
  const tooltip = formatTrayTooltip(many);
  assert.ok(tooltip.length <= 127 && tooltip.endsWith('...'));
});

test('update available adds Settings item and ready update adds Restart to update', () => {
  const availableMenu = formatTrayMenu([claude], [running], NOW, {
    ...OPTIONS,
    update: { state: 'available', available_version: '0.2.0' },
  });
  const availableItem = availableMenu.find((item) => item.label === 'Update available: v0.2.0');
  assert.ok(availableItem);
  assert.deepEqual(availableItem.action, { kind: 'open-view', view: 'settings' });
  assert.equal(availableMenu.some((item) => item.label === 'Restart to update'), false);

  const readyMenu = formatTrayMenu([claude], [running], NOW, {
    ...OPTIONS,
    update: { state: 'ready', available_version: '0.2.0' },
  });
  const readyItem = readyMenu.find((item) => item.label === 'Update available: v0.2.0');
  const restartItem = readyMenu.find((item) => item.label === 'Restart to update');
  assert.ok(readyItem && restartItem);
  assert.deepEqual(restartItem.action, { kind: 'restart-to-update' });
});
