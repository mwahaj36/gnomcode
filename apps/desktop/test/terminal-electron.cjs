const { app, BrowserWindow, session, ipcMain } = require('electron');
const { createServer } = require('node:http');
const { mkdtempSync } = require('node:fs');
const { tmpdir } = require('node:os');
const path = require('node:path');
const { WebSocketServer } = require('ws');
const { isAllowedResource, windowChrome } = require('../dist/security.js');

app.setPath('userData', mkdtempSync(path.join(tmpdir(), 'gnomcode-electron-test-')));
const timeout = setTimeout(() => { console.error('Terminal WebSocket timed out'); app.exit(1); }, 15000);
app.whenReady().then(async () => {
  const server = createServer((_request, response) => {
    response.writeHead(200, {'Content-Type':'text/html'}); response.end('<!doctype html><title>Terminal test</title><style>.sp-chat-column{position:fixed;inset:0 0 56px}</style><body class="sp-native-console"><div class="gc-console sp-console-workspace"><div class="sp-chat-column"><aside class="ide-sidebar"></aside></div></div></body>');
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const port = server.address().port;
  const sockets = new WebSocketServer({server});
  sockets.on('connection', socket => socket.on('message', data => {
    if (data.toString() === 'keyboard-test') socket.send(Buffer.from('terminal-output'));
  }));
  const isolated = session.fromPartition('terminal-test');
  isolated.webRequest.onBeforeRequest((details, callback) => callback({cancel:!isAllowedResource(details.url, port)}));
  const window = new BrowserWindow({...windowChrome(process.platform,true),width:700,show:false,webPreferences:{preload:path.resolve(__dirname,'../dist/preload.js'),session:isolated,sandbox:true,contextIsolation:true,nodeIntegration:false,devTools:false}});
  let appearance;ipcMain.handle('desktop:set-appearance',(_event,theme)=>{appearance=theme;});
  await window.loadURL(`http://127.0.0.1:${port}`);
  const chrome = await window.webContents.executeJavaScript(`new Promise(resolve=>setTimeout(()=>{document.documentElement.dataset.theme='dark';resolve({header:document.querySelector('.desktop-titlebar')?.textContent,drag:getComputedStyle(document.querySelector('.desktop-titlebar')).webkitAppRegion,padding:getComputedStyle(document.body).paddingTop});},100))`);
  if(chrome.header !== 'Gnomcode' || chrome.drag !== 'drag' || chrome.padding !== '32px') throw new Error('Desktop titlebar failed: '+JSON.stringify(chrome));
  if(await window.webContents.executeJavaScript("document.querySelector('.sp-chat-column').getBoundingClientRect().top") !== 32) throw new Error('Compact history drawer overlaps titlebar');
  await new Promise(resolve=>setTimeout(resolve,100));
  if(appearance !== 'dark') throw new Error('Titlebar theme did not synchronize');
  console.log('PASS: real sandboxed Electron titlebar, drag region and theme synchronization');
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
