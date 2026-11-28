#!/usr/bin/env node
/**
 * Zero-knowledge E2EE relay for Gnomcode Remote companion, run as a plain
 * long-lived process on Fly.io (Machines) instead of Cloudflare Workers.
 *
 * Same contract as workers/remote-relay/src/index.mjs: forwards raw sealed
 * ciphertext frames between exactly two WebSocket peers (one "desktop", one
 * "phone") in a room keyed by room id. Never inspects or stores message
 * content. Rooms live only in this process's memory; a restart drops all
 * in-flight connections, which is fine since clients already retry pairing.
 */
import http from 'node:http';
import { WebSocketServer } from 'ws';

const PORT = parseInt(process.env.PORT || '8080', 10);
const MAX_FRAME_BYTES = 65536;
const ROOM_PATH = /^\/relay\/([a-zA-Z0-9_-]{16,128})$/;

/** roomId -> Map(ws -> role) */
const rooms = new Map();

const server = http.createServer((req, res) => {
  const url = new URL(req.url, `http://localhost:${PORT}`);
  if (url.pathname === '/health') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ ok: true, service: 'gnomcode-remote-relay', rooms: rooms.size }));
    return;
  }
  res.writeHead(404);
  res.end('Not Found');
});

const wss = new WebSocketServer({ noServer: true });

server.on('upgrade', (req, socket, head) => {
  const url = new URL(req.url, `http://localhost:${PORT}`);
  const match = url.pathname.match(ROOM_PATH);
  if (!match) {
    socket.write('HTTP/1.1 404 Not Found\r\n\r\n');
    socket.destroy();
    return;
  }
  const role = url.searchParams.get('role');
  if (role !== 'desktop' && role !== 'phone') {
    socket.write('HTTP/1.1 400 Bad Request\r\n\r\n');
    socket.destroy();
    return;
  }
  const roomId = match[1];
  let room = rooms.get(roomId);
  if (!room) {
    room = new Map();
    rooms.set(roomId, room);
  }
  if (room.size >= 2 || [...room.values()].includes(role)) {
    socket.write('HTTP/1.1 409 Conflict\r\n\r\n');
    socket.destroy();
    return;
  }

  wss.handleUpgrade(req, socket, head, (ws) => {
    room.set(ws, role);

    const cleanup = () => {
      if (!room.has(ws)) return;
      room.delete(ws);
      for (const [peer] of room) {
        try { peer.close(1000, 'Peer disconnected'); } catch { /* already closed */ }
      }
      room.clear();
      rooms.delete(roomId);
    };

    ws.on('message', (data, isBinary) => {
      const len = data.length ?? data.byteLength ?? -1;
      if (len < 1 || len > MAX_FRAME_BYTES) {
        ws.close(1009, 'Frame too large');
        cleanup();
        return;
      }
      for (const [peer] of room) {
        if (peer !== ws && peer.readyState === peer.OPEN) {
          try {
            peer.send(data, { binary: isBinary });
          } catch {
            ws.close(1011, 'Peer unavailable');
            cleanup();
          }
        }
      }
    });

    ws.on('close', cleanup);
    ws.on('error', cleanup);
  });
});

server.listen(PORT, '0.0.0.0', () => {
  console.log(`[relay] Gnomcode remote relay listening on 0.0.0.0:${PORT}`);
});
