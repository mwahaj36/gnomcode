import test from 'node:test';
import assert from 'node:assert/strict';
import { relativeTime, isOnline, deviceKind, groupDevices } from '../lib/devices-presentation.mjs';

const now = Date.parse('2026-11-13T12:00:00Z');

test('relative time reads naturally and tolerates bad input', () => {
  assert.equal(relativeTime('2026-11-13T11:59:50Z', now), 'just now');
  assert.equal(relativeTime('2026-11-13T11:45:00Z', now), '15 min ago');
  assert.equal(relativeTime('2026-11-13T07:00:00Z', now), '5 h ago');
  assert.equal(relativeTime('2026-11-10T12:00:00Z', now), '3 d ago');
  assert.equal(relativeTime('nope', now), null);
});

test('online means a recent heartbeat', () => {
  assert.equal(isOnline('2026-11-13T11:58:00Z', now), true);
  assert.equal(isOnline('2026-11-13T11:50:00Z', now), false);
  assert.equal(isOnline('', now), false);
});

test('device kind comes from the enrolled name', () => {
  assert.equal(deviceKind('Phone browser 8a44'), 'phone');
  assert.equal(deviceKind('Desktop browser 1c2d'), 'browser');
});

test('devices group into this device, approved, pending and revoked', () => {
  const rows = [
    { id: 'a', state: 'approved', revoked: false },
    { id: 'b', state: 'approved', revoked: false },
    { id: 'c', state: 'pending', revoked: false },
    { id: 'd', state: 'approved', revoked: true },
  ];
  assert.deepEqual(groupDevices(rows, ['b']).map(g => [g.key, g.rows.map(r => r.id)]), [['this', ['b']], ['approved', ['a']], ['pending', ['c']], ['revoked', ['d']]]);
  assert.deepEqual(groupDevices([], []), []);
});
