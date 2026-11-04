import test from 'node:test';
import assert from 'node:assert/strict';
import { approvedRequest } from '../lib/device-approval.mjs';

test('only the waiting browser approval resumes its desktop connection', () => {
  const waiting = {desktop: 'desk', request: 'new'};
  const other = {desktop_id: 'desk', id: 'old', state: 'approved', revoked: false, offer: {room: 'old'}};
  const own = {desktop_id: 'desk', id: 'new', state: 'pending', revoked: false, offer: null};
  assert.equal(approvedRequest([other, own], waiting), undefined);
  own.state = 'approved'; own.offer = {room: 'new'};
  assert.equal(approvedRequest([other, own], waiting), own);
  own.revoked = true;
  assert.equal(approvedRequest([other, own], waiting), undefined);
});
