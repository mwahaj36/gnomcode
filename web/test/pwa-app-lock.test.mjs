import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const lockMod = await import('../lib/app-lock.js');

test('PIN enrollment hashes with PBKDF2 salt and refuses weak PINs or locked updates', async () => {
  assert.equal(typeof lockMod.enrollPinLock, 'function');
  assert.equal(typeof lockMod.unlockWithPin, 'function');

  await assert.rejects(() => lockMod.enrollPinLock('12', { unlocked: true }), /4 to 8 digits/);
  await assert.rejects(() => lockMod.enrollPinLock('abcd', { unlocked: true }), /4 to 8 digits/);
  await assert.rejects(() => lockMod.enrollPinLock('2580', { unlocked: false }), /unlock/i);

  const state = await lockMod.enrollPinLock('258046', { unlocked: true, idleMinutes: 5 });
  assert.equal(state.mode, 'pin');
  assert.equal(state.idleMinutes, 5);
  assert.ok(state.pinSalt && state.pinHash);
  assert.doesNotMatch(JSON.stringify(state), /258046/);

  const ok = await lockMod.unlockWithPin(state, '258046');
  assert.equal(ok.unlocked, true);
  assert.equal(ok.nextState.failedAttempts, 0);

  const bad = await lockMod.unlockWithPin(state, '999999');
  assert.equal(bad.unlocked, false);
  assert.equal(bad.nextState.failedAttempts, 1);
});

test('10 consecutive wrong PIN attempts enforce 10-minute cooldown', async () => {
  let state = await lockMod.enrollPinLock('1357', { unlocked: true });
  let now = 1_700_000_000_000;

  for (let i = 0; i < 9; i++) {
    const res = await lockMod.unlockWithPin(state, '0000', now);
    assert.equal(res.unlocked, false);
    assert.equal(res.lockedOut, false);
    state = res.nextState;
  }

  const tenth = await lockMod.unlockWithPin(state, '0000', now);
  assert.equal(tenth.unlocked, false);
  assert.equal(tenth.lockedOut, true);
  assert.equal(tenth.nextState.lockedUntil, now + lockMod.LOCKOUT_DURATION_MS);

  // Even the right PIN cannot bypass active cooldown
  const duringCooldown = await lockMod.unlockWithPin(tenth.nextState, '1357', now + 60_000);
  assert.equal(duringCooldown.unlocked, false);
  assert.equal(duringCooldown.lockedOut, true);

  // Once cooldown expires, valid PIN succeeds
  const afterCooldown = await lockMod.unlockWithPin(tenth.nextState, '1357', now + lockMod.LOCKOUT_DURATION_MS + 1);
  assert.equal(afterCooldown.unlocked, true);
  assert.equal(afterCooldown.nextState.failedAttempts, 0);
});

test('PWA app lock wires into Client, SettingsPanel, and tokens CSS without touching E2EE keys', () => {
  const gate = readFileSync(join(root, 'web/components/AppLockGate.tsx'), 'utf8');
  const client = readFileSync(join(root, 'web/app/console/Client.tsx'), 'utf8');
  const settings = readFileSync(join(root, 'web/components/SettingsPanel.tsx'), 'utf8');
  const css = readFileSync(join(root, 'web/app/gnomcode-console.css'), 'utf8');
  const remoteDevice = readFileSync(join(root, 'web/lib/remote-device.ts'), 'utf8');

  assert.match(client, /AppLockGate/);
  assert.match(settings, /AppLockSettings/);
  assert.match(gate, /sp-lock-screen/);
  assert.match(gate, /forgetDevice/);
  assert.doesNotMatch(remoteDevice, /app-lock|gnomcode-pwa-lock/);

  const marker = css.indexOf('/* Section C PWA app lock */');
  assert.notEqual(marker, -1, 'Expected Section C PWA app lock CSS block');
  const sectionCss = css.slice(marker);
  assert.doesNotMatch(sectionCss, /#[0-9a-fA-F]{3,8}\b/, 'Use --sp-* tokens only');
});
