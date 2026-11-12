import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
  DEFAULT_QUICK_ACTIONS,
  mergeQuickActions,
  normalizeQuickAction,
  renderQuickActionsStrip,
} from '../lib/console-presentation.js';

const read = path => readFileSync(new URL(path, import.meta.url), 'utf8');

test('default quick actions are curated canned prompts without slash commands or raw argv', () => {
  assert.ok(Array.isArray(DEFAULT_QUICK_ACTIONS) && DEFAULT_QUICK_ACTIONS.length >= 3);
  for (const item of DEFAULT_QUICK_ACTIONS) {
    assert.ok(typeof item.id === 'string' && item.id.length > 0);
    assert.ok(typeof item.label === 'string' && item.label.length > 0 && item.label.length <= 40);
    assert.ok(typeof item.prompt === 'string' && item.prompt.trim().length > 0 && item.prompt.length <= 4000);
    assert.equal(item.prompt.trimStart().startsWith('/'), false);
    assert.equal('argv' in item, false);
    assert.equal('command' in item, false);
    assert.equal('shell' in item, false);
  }
});

test('normalizeQuickAction validates plain prompts and strips unsafe fields', () => {
  const clean = normalizeQuickAction({
    id: 'tests',
    label: '  Verify build  ',
    prompt: 'Run cargo test and summarize any failures.\nKeep it brief.',
    argv: ['rm', '-rf', '/'],
    command: 'sh',
    shell: true,
  });
  assert.deepEqual(clean, {
    id: 'tests',
    label: 'Verify build',
    prompt: 'Run cargo test and summarize any failures.\nKeep it brief.',
    builtin: false,
  });

  assert.equal(normalizeQuickAction({ label: '', prompt: 'Hi' }), null);
  assert.equal(normalizeQuickAction({ label: 'Slash', prompt: '/status' }), null);
  assert.equal(normalizeQuickAction({ label: 'Ctrl', prompt: 'hello\u0007world' }), null);
  assert.equal(normalizeQuickAction({ label: 'Long', prompt: 'a'.repeat(4001) }), null);
});

test('mergeQuickActions keeps valid entries and falls back to defaults', () => {
  const merged = mergeQuickActions([
    { id: 'custom-1', label: 'Review diff', prompt: 'Review the current diff for edge cases.' },
    { id: 'bad', label: 'Slash', prompt: '/help' },
  ]);
  assert.equal(merged.length, 1);
  assert.equal(merged[0].id, 'custom-1');
  assert.deepEqual(mergeQuickActions(null), DEFAULT_QUICK_ACTIONS);
  assert.deepEqual(mergeQuickActions([]), DEFAULT_QUICK_ACTIONS);
});

test('renderQuickActionsStrip escapes content and respects disabled state', () => {
  const markup = renderQuickActionsStrip(
    [{ id: 'x"1', label: '<script>Test</script>', prompt: 'Prompt & check' }],
    { disabled: true },
  );
  assert.doesNotMatch(markup, /<script>/);
  assert.match(markup, /data-action="quick-action"/);
  assert.match(markup, /data-action="manage-quick-actions"/);
  assert.match(markup, /disabled/);
});

test('quick actions wire into console and remote using plain prompt delivery only', () => {
  const remote = read('../components/RemoteCompanion.tsx');
  const consoleHtml = read('../../crates/daemon/src/console.html');
  const consoleJs = read('../../crates/daemon/src/console-app.js');
  const css = read('../app/gnomcode-console.css');

  assert.match(consoleHtml, /id="chat-quick-actions"/);
  assert.match(consoleHtml, /id="quick-actions-dialog"/);
  assert.match(consoleJs, /'quick-action':\s*sendQuickAction/);
  assert.match(consoleJs, /'manage-quick-actions':\s*openQuickActionsDialog/);
  assert.match(consoleJs, /sendPromptText\(/);

  assert.match(remote, /sp-quick-actions/);
  assert.match(remote, /submitQuickAction/);
  assert.match(remote, /Manage quick prompts/);

  const cssSlice = css.slice(css.indexOf('.sp-quick-actions{'));
  assert.ok(css.includes('.sp-quick-actions{'));
  assert.doesNotMatch(cssSlice, /#[0-9a-f]{3,6}\b/i);
  assert.match(cssSlice, /max-height:450px/);
});
