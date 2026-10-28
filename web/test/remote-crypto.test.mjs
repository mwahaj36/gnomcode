import test from 'node:test';
import assert from 'node:assert/strict';
import { createChannel, generateKeyPair } from '../lib/remote-crypto.mjs';

test('paired browsers agree on a code and reject replay, tampering and wrong rooms', async () => {
  const desktop = await generateKeyPair();
  const phone = await generateKeyPair();
  const a = await createChannel(desktop, phone.publicHex, 'room-test', 'desktop');
  const b = await createChannel(phone, desktop.publicHex, 'room-test', 'phone');
  assert.equal(a.verifyCode, b.verifyCode);
  const frame = await a.seal({ type: 'chat', text: 'hello' });
  assert.deepEqual(await b.open(frame), { type: 'chat', text: 'hello' });
  await assert.rejects(b.open(frame));
  const next = await a.seal({ type: 'chat', text: 'second' });
  const tampered = { ...next, ciphertext: next.ciphertext.replace(/^./, next.ciphertext[0] === '0' ? '1' : '0') };
  await assert.rejects(b.open(tampered));
  const wrong = await createChannel(phone, desktop.publicHex, 'different-room', 'phone');
  assert.notEqual(a.verifyCode, wrong.verifyCode);
  await assert.rejects(wrong.open(frame));
  const concurrent = await Promise.all([a.seal({ n: 1 }), a.seal({ n: 2 })]);
  assert.notEqual(concurrent[0].sequence_num, concurrent[1].sequence_num);
});

test('browser derives the same X25519 pairing code as the Rust peer', async () => {
  const seed = new Uint8Array(32).fill(9);
  const prefix = Uint8Array.from('302e020100300506032b656e04220420'.match(/../g), pair => Number.parseInt(pair, 16));
  const pkcs8 = new Uint8Array(prefix.length + seed.length);
  pkcs8.set(prefix); pkcs8.set(seed, prefix.length);
  const privateKey = await crypto.subtle.importKey('pkcs8', pkcs8, 'X25519', false, ['deriveBits']);
  const local = { privateKey, publicHex: '57db4b359f23ae5e146e4e2512056704722506348c150c14753d0c933d04d421' };
  const channel = await createChannel(local, '13be4feaeaf204c7fd3358fc9c00721881d174278128227ec674f37f7fe97b6d', 'test-room', 'phone');
  assert.equal(channel.verifyCode, '107539');
  const desktopSeed = new Uint8Array(32).fill(7);
  const desktopPkcs8 = new Uint8Array(prefix.length + desktopSeed.length);
  desktopPkcs8.set(prefix); desktopPkcs8.set(desktopSeed, prefix.length);
  const desktopPrivateKey = await crypto.subtle.importKey('pkcs8', desktopPkcs8, 'X25519', false, ['deriveBits']);
  const desktop = await createChannel({ privateKey: desktopPrivateKey, publicHex: '13be4feaeaf204c7fd3358fc9c00721881d174278128227ec674f37f7fe97b6d' }, local.publicHex, 'test-room', 'desktop');
  const frame = await desktop.seal({ type: 'sessions' });
  assert.equal(frame.ciphertext, '14ec0cfbb605d66599953653b73328b932fc4b0832f36df8dd6a690a3322ab4ddd82fc');
  assert.deepEqual(await channel.open(frame), { type: 'sessions' });
});
