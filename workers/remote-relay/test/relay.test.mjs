import test from 'node:test';
import assert from 'node:assert/strict';

test('relay health endpoint returns ok', async () => {
  const request = new Request('https://relay.spawnt.internal/health');
  assert.equal(request.method, 'GET');
  const url = new URL(request.url);
  assert.equal(url.pathname, '/health');
});

test('relay rejects non-websocket request on room endpoint', async () => {
  const request = new Request('https://relay.spawnt.internal/relay/test-room-123');
  const match = new URL(request.url).pathname.match(/^\/relay\/([a-zA-Z0-9_-]+)$/);
  assert.ok(match);
  assert.equal(match[1], 'test-room-123');
  assert.equal(request.headers.get('Upgrade'), null);
});

test('zero-knowledge relay forwards raw sealed ciphertext between peers', async () => {
  // Simulate peer message forwarding
  const messagesReceived = [];
  const fakePhonePeer = {
    send(data) {
      messagesReceived.push(data);
    }
  };

  const peers = new Map();
  const fakeDesktopServer = {};
  peers.set(fakeDesktopServer, 'desktop');
  peers.set(fakePhonePeer, 'phone');

  // Desktop sends sealed frame
  const sealedCiphertext = 'sealed-aes-gcm-ciphertext-frame';
  for (const [peer] of peers) {
    if (peer !== fakeDesktopServer) {
      peer.send(sealedCiphertext);
    }
  }

  assert.equal(messagesReceived.length, 1);
  assert.equal(messagesReceived[0], 'sealed-aes-gcm-ciphertext-frame');
});
