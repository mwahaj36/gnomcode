/**
 * Zero-Knowledge E2EE Relay Worker for Spawnt Remote Companion.
 * Forwards sealed ciphertext frames between paired desktop and mobile peers.
 * Never inspects or stores message content.
 */

export interface Env {
  RELAY_ROOM: DurableObjectNamespace;
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);

    if (url.pathname === '/health') {
      return new Response(JSON.stringify({ ok: true, service: 'spawnt-remote-relay' }), {
        headers: { 'Content-Type': 'application/json' },
      });
    }

    // Match route: /relay/:roomId
    const match = url.pathname.match(/^\/relay\/([a-zA-Z0-9_-]+)$/);
    if (!match) {
      return new Response('Not Found', { status: 404 });
    }

    if (request.headers.get('Upgrade') !== 'websocket') {
      return new Response('Expected WebSocket upgrade', { status: 426 });
    }

    const roomId = match[1];
    const id = env.RELAY_ROOM.idFromName(roomId);
    const room = env.RELAY_ROOM.get(id);
    return room.fetch(request);
  },
};

export class RelayRoom {
  private peers: Map<WebSocket, string> = new Map();

  async fetch(request: Request): Promise<Response> {
    const url = new URL(request.url);
    const role = url.searchParams.get('role') || 'peer';

    // WebSocket pairing
    const pair = new WebSocketPair();
    const [client, server] = [pair[0], pair[1]];

    server.accept();
    this.peers.set(server, role);

    server.addEventListener('message', (event: MessageEvent) => {
      // Forward raw sealed ciphertext frame to any other peer in this room
      for (const [peer] of this.peers) {
        if (peer !== server) {
          try {
            peer.send(event.data);
          } catch {
            // Drop failing peer cleanly
            this.peers.delete(peer);
          }
        }
      }
    });

    const cleanup = () => {
      this.peers.delete(server);
    };

    server.addEventListener('close', cleanup);
    server.addEventListener('error', cleanup);

    return new Response(null, { status: 101, webSocket: client });
  }
}
