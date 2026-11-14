import test from 'node:test';
import assert from 'node:assert/strict';
import { renderLiveBadge, renderActivityIndicator, formatElapsed, renderSessionList } from '../lib/console-presentation.js';

test('each live state has text and a shape; inferred states say so', () => {
  for (const [state, label] of [['working', 'Working'], ['waiting_for_you', 'Waiting for you'], ['idle', 'Idle'], ['stopped', 'Stopped']]) {
    const html = renderLiveBadge({ state, basis: 'exact', step: '' });
    assert.match(html, new RegExp(`data-live="${state}"`));
    assert.match(html, /sp-live-shape/);
    assert.match(html, new RegExp(`>${label}<`));
    assert.doesNotMatch(html, /Inferred/);
  }
  assert.match(renderLiveBadge({ state: 'idle', basis: 'estimated', step: '' }), /Inferred/);
  assert.equal(renderLiveBadge(null), '');
});

test('activity indicator shows step and elapsed only while working', () => {
  const now = Date.parse('2026-11-14T10:01:05Z');
  const html = renderActivityIndicator({ state: 'working', basis: 'exact', step: 'Running a command' }, '2026-11-14T10:00:00Z', now);
  assert.match(html, /Running a command/);
  assert.match(html, /1m 05s/);
  assert.equal(renderActivityIndicator({ state: 'idle', basis: 'exact', step: '' }, null, now), '');
  assert.equal(formatElapsed('2026-11-14T08:00:00Z', now), '2h 01m');
});

test('sidebar rows carry the live badge', () => {
  const html = renderSessionList([{ id: 'a', agent: 'codex', title: 'T', state: 'tool', live: { state: 'working', basis: 'estimated', step: 'Thinking' }, last_event_at: '2026-11-14T10:00:00Z' }], 'a');
  assert.match(html, /data-live="working"/);
  assert.match(html, /Inferred/);
});
