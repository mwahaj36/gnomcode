import test from 'node:test';
import assert from 'node:assert/strict';
import { remoteRequestParts } from '../lib/remote-upload.mjs';
import { createChannel, generateKeyPair } from '../lib/remote-crypto.mjs';

test('15 MiB attachment crosses the relay in bounded encrypted frames without byte loss', async () => {
  const request = { type: 'upload_attachment', filename: 'notes.txt', data_base64: Buffer.alloc(15 * 1024 * 1024, 42).toString('base64') };
  const desktop = await generateKeyPair(), phone = await generateKeyPair();
  const sender = await createChannel(phone, desktop.publicHex, 'upload-test', 'phone');
  const receiver = await createChannel(desktop, phone.publicHex, 'upload-test', 'desktop');
  let reconstructed = '';
  for (const part of remoteRequestParts(request)) {
    const frame = await sender.seal(part);
    assert.ok(Buffer.byteLength(JSON.stringify(frame)) <= 65536);
    const decoded = await receiver.open(frame);
    reconstructed += decoded.data;
  }
  assert.deepEqual(JSON.parse(reconstructed), request);
});

test('small commands remain compatible and Unicode survives chunk boundaries', () => {
  assert.deepEqual([...remoteRequestParts({ type: 'sessions' })], [{ type: 'sessions' }]);
  const request = { type: 'upload_attachment', filename: 'a'.repeat(11930) + String.fromCodePoint(0x1f600), data_base64: 'b'.repeat(50000) };
  const parts = [...remoteRequestParts(request)];
  assert.equal(parts.at(-1).final, true);
  assert.deepEqual(JSON.parse(parts.map(p => p.data).join('')), request);
});
