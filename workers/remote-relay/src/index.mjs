/**
 * Zero-Knowledge E2EE Relay Worker for Gnomcode Remote Companion.
 * Forwards sealed ciphertext frames between paired desktop and mobile peers.
 * Never inspects or stores message content.
 */

export default {
  async fetch(request, env) {
    const url = new URL(request.url);

    if (url.pathname === '/health') {
      return new Response(JSON.stringify({ ok: true, service: 'gnomcode-remote-relay' }), {
        headers: { 'Content-Type': 'application/json' },
      });
    }

    // Match route: /relay/:roomId
    const match = url.pathname.match(/^\/relay\/([a-zA-Z0-9_-]{16,128})$/);
    if (!match) {
      return new Response('Not Found', { status: 404 });
    }

    if (request.headers.get('Upgrade')?.toLowerCase() !== 'websocket') {
      return new Response('Expected WebSocket upgrade', { status: 426 });
    }

    const roomId = match[1];
    const id = env.RELAY_ROOM.idFromName(roomId);
    const room = env.RELAY_ROOM.get(id);
    return room.fetch(request);
  },
};

export class RelayRoom {
  peers = new Map();

  async fetch(request) {
    const url = new URL(request.url);
    const role = url.searchParams.get('role');
    if (role !== 'desktop' && role !== 'phone') return new Response('Invalid role', { status: 400 });
    if (this.peers.size >= 2 || [...this.peers.values()].includes(role)) {
      return new Response('Room full', { status: 409 });
    }

    // WebSocket pairing
    const pair = new WebSocketPair();
    const [client, server] = [pair[0], pair[1]];

    server.accept();
    this.peers.set(server, role);

    server.addEventListener('message', (event) => {
      const size = event.data instanceof ArrayBuffer ? event.data.byteLength : -1;
      if (size < 1 || size > 65536) {
        server.close(1009, 'Frame too large');
        cleanup();
        return;
      }
      // Forward raw sealed ciphertext frame to any other peer in this room
      for (const [peer] of this.peers) {
        if (peer !== server) {
          try {
            peer.send(event.data);
          } catch {
            server.close(1011, 'Peer unavailable');
            cleanup();
          }
        }
      }
    });

    const cleanup = () => {
      if (!this.peers.delete(server)) return;
      for (const peer of this.peers.keys()) {
        this.peers.delete(peer);
        try { peer.close(1000, 'Peer disconnected'); } catch { /* Already closed. */ }
      }
    };

    server.addEventListener('close', cleanup);
    server.addEventListener('error', cleanup);

    return new Response(null, { status: 101, webSocket: client });
  }
}
