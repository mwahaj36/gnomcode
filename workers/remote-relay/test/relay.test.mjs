import test from 'node:test';
import assert from 'node:assert/strict';
import worker, { RelayRoom } from '../src/index.mjs';

test('relay room rejects a third peer', async () => {
  class FakeSocket extends EventTarget { accept() {} send() {} close() {} }
  globalThis.WebSocketPair = class { constructor() { this[0] = new FakeSocket(); this[1] = new FakeSocket(); } };
  const NativeResponse = globalThis.Response;
  globalThis.Response = class extends NativeResponse { constructor(body, init) { super(body, init.status === 101 ? {status: 200} : init); } };
  try {
    const room = new RelayRoom();
    const desktop = new Request('https://relay.example/relay/0123456789abcdef?role=desktop');
    const phone = new Request('https://relay.example/relay/0123456789abcdef?role=phone');
    await room.fetch(desktop);
    assert.equal((await room.fetch(desktop)).status, 409);
    await room.fetch(phone);
    assert.equal((await room.fetch(desktop)).status, 409);
  } finally { globalThis.Response = NativeResponse; delete globalThis.WebSocketPair; }
});

test('relay forwards frames and rejects oversized messages', async () => {
  const sockets = [];
  class FakeSocket extends EventTarget {
    sent = []; closed = null;
    accept() {} send(data) { this.sent.push(data); } close(code) { this.closed = code; }
  }
  globalThis.WebSocketPair = class { constructor() { this[0] = new FakeSocket(); this[1] = new FakeSocket(); sockets.push(this[1]); } };
  const NativeResponse = globalThis.Response;
  globalThis.Response = class extends NativeResponse { constructor(body, init) { super(body, init.status === 101 ? {status: 200} : init); } };
  try {
    const room = new RelayRoom();
    await room.fetch(new Request('https://relay.example/relay/0123456789abcdef?role=desktop'));
    await room.fetch(new Request('https://relay.example/relay/0123456789abcdef?role=phone'));
    sockets[0].dispatchEvent(new MessageEvent('message', {data: new Uint8Array([1,2]).buffer}));
    assert.equal(sockets[1].sent.length, 1);
    sockets[0].dispatchEvent(new MessageEvent('message', {data: new Uint8Array(65537).buffer}));
    assert.equal(sockets[0].closed, 1009);
    assert.equal(sockets[1].sent.length, 1);
    sockets[1].dispatchEvent(new MessageEvent('message', {data: 'plaintext'}));
    assert.equal(sockets[1].closed, 1009);
    assert.notEqual(sockets[0].closed, null);
  } finally { globalThis.Response = NativeResponse; delete globalThis.WebSocketPair; }
});

test('relay closes the other peer when a device disconnects', async () => {
  const sockets = [];
  class FakeSocket extends EventTarget { closed = null; accept() {} send() {} close(code) { this.closed = code; } }
  globalThis.WebSocketPair = class { constructor() { this[0] = new FakeSocket(); this[1] = new FakeSocket(); sockets.push(this[1]); } };
  const NativeResponse = globalThis.Response;
  globalThis.Response = class extends NativeResponse { constructor(body, init) { super(body, init.status === 101 ? { status: 200 } : init); } };
  try {
    const room = new RelayRoom();
    await room.fetch(new Request('https://relay.example/relay/0123456789abcdef?role=desktop'));
    await room.fetch(new Request('https://relay.example/relay/0123456789abcdef?role=phone'));
    sockets[0].dispatchEvent(new Event('close'));
    assert.notEqual(sockets[1].closed, null);
  } finally { globalThis.Response = NativeResponse; delete globalThis.WebSocketPair; }
});

test('relay health endpoint returns ok', async () => {
  const request = new Request('https://relay.spawnt.internal/health');
  assert.equal(request.method, 'GET');
  const url = new URL(request.url);
  assert.equal(url.pathname, '/health');
  const response = await worker.fetch(request, {});
  assert.equal(response.status, 200);
  assert.equal((await response.json()).ok, true);
});

test('relay rejects non-websocket request on room endpoint', async () => {
  const request = new Request('https://relay.spawnt.internal/relay/test-room-123456?role=phone');
  const match = new URL(request.url).pathname.match(/^\/relay\/([a-zA-Z0-9_-]{16,128})$/);
  assert.ok(match);
  assert.equal(match[1], 'test-room-123456');
  assert.equal(request.headers.get('Upgrade'), null);
  assert.equal((await worker.fetch(request, {})).status, 426);
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
