const { app, BrowserWindow, session } = require('electron');
const { createServer } = require('node:http');
const { mkdtempSync } = require('node:fs');
const { tmpdir } = require('node:os');
const path = require('node:path');
const { WebSocketServer } = require('ws');
const { isAllowedResource } = require('../dist/security.js');

app.setPath('userData', mkdtempSync(path.join(tmpdir(), 'spawnt-electron-test-')));
const timeout = setTimeout(() => { console.error('Terminal WebSocket timed out'); app.exit(1); }, 15000);
app.whenReady().then(async () => {
  const server = createServer((_request, response) => {
    response.writeHead(200, {'Content-Type':'text/html'}); response.end('<!doctype html><title>Terminal test</title>');
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const port = server.address().port;
  const sockets = new WebSocketServer({server});
  sockets.on('connection', socket => socket.on('message', data => {
    if (data.toString() === 'keyboard-test') socket.send(Buffer.from('terminal-output'));
  }));
  const isolated = session.fromPartition('terminal-test');
  isolated.webRequest.onBeforeRequest((details, callback) => callback({cancel:!isAllowedResource(details.url, port)}));
  const window = new BrowserWindow({show:false,webPreferences:{session:isolated,sandbox:true,contextIsolation:true,nodeIntegration:false,devTools:false}});
  await window.loadURL(`http://127.0.0.1:${port}`);
  const result = await window.webContents.executeJavaScript(`new Promise((resolve,reject) => {
    const socket = new WebSocket('ws://127.0.0.1:${port}/api/v1/sessions/test/pty?token=synthetic');
    socket.binaryType = 'arraybuffer';
    socket.onopen = () => socket.send(new TextEncoder().encode('keyboard-test'));
    socket.onmessage = event => { resolve(new TextDecoder().decode(event.data)); socket.close(); };
    socket.onerror = () => reject(new Error('Electron blocked terminal WebSocket'));
  })`);
  if (result !== 'terminal-output') throw new Error('Terminal roundtrip failed');
  console.log('PASS: real sandboxed Electron renderer sends keyboard input and receives binary terminal output');
  clearTimeout(timeout); window.destroy(); sockets.close(); server.close(); app.exit(0);
}).catch(error => { console.error(error.message); app.exit(1); });
