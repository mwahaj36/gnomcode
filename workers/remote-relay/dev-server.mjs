#!/usr/bin/env node
/**
 * Local development relay server for Gnomcode Remote companion.
 * Bridges desktop and phone/browser WebSocket connections for a pairing room.
 */
import http from 'node:http';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);

let WebSocketServer;
try {
  WebSocketServer = require('ws').Server;
} catch {
  WebSocketServer = require(path.resolve(__dirname, '../../apps/desktop/node_modules/ws')).Server;
}

const PORT = parseInt(process.env.PORT || '8787', 10);
const rooms = new Map(); // roomId -> Map(ws -> role)

const server = http.createServer((req, res) => {
  const url = new URL(req.url, `http://localhost:${PORT}`);
  if (url.pathname === '/health') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ ok: true, service: 'gnomcode-local-relay' }));
    return;
  }
  res.writeHead(404);
  res.end('Not Found');
});

const wss = new WebSocketServer({ noServer: true });

server.on('upgrade', (req, socket, head) => {
  const url = new URL(req.url, `http://localhost:${PORT}`);
  const match = url.pathname.match(/^\/relay\/([a-zA-Z0-9_-]{16,128})$/);
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
    console.log(`[relay] Connected ${role} to room ${roomId} (peers: ${room.size})`);

    const cleanup = () => {
      if (!room.has(ws)) return;
      room.delete(ws);
      console.log(`[relay] Disconnected ${role} from room ${roomId}`);
      for (const [peer] of room) {
        try { peer.close(1000, 'Peer disconnected'); } catch {}
      }
      room.clear();
      rooms.delete(roomId);
    };

    ws.on('message', (data, isBinary) => {
      const len = data.length || (data.byteLength ?? 0);
      if (len < 1 || len > 65536) {
        ws.close(1009, 'Frame too large');
        cleanup();
        return;
      }
      for (const [peer] of room) {
        if (peer !== ws && peer.readyState === 1) {
          peer.send(data, { binary: isBinary });
        }
      }
    });

    ws.on('close', cleanup);
    ws.on('error', cleanup);
  });
});

server.listen(PORT, '127.0.0.1', () => {
  console.log(`[relay] Gnomcode local relay running on ws://127.0.0.1:${PORT}/relay`);
  console.log(`[relay] Set environment variable: GNOMCODE_REMOTE_RELAY_URL=ws://127.0.0.1:${PORT}/relay`);
});
