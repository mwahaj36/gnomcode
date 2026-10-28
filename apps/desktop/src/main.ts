import { app, BrowserWindow, dialog, ipcMain, Menu, session, shell, Tray } from 'electron';
import squirrelStartup from 'electron-squirrel-startup';
import { spawn } from 'node:child_process';
import { get, type IncomingMessage } from 'node:http';
import { homedir } from 'node:os';
import * as path from 'node:path';
import { daemonEnvironment, isAllowedExternalUrl, isAllowedResource, isDaemonNavigation, trayTooltip } from './security';
import { readDaemon } from './daemon-client';

let window: BrowserWindow | null = null;
let tray: Tray | null = null;
let quitting = false;
let daemonPort = 0;

function dataHome(): string {
  return process.env.SPAWNT_HOME?.trim() || path.join(homedir(), '.spawnt');
}

async function connectDaemon(): Promise<string> {
  const running = await readDaemon(dataHome());
  if (running) return running;
  const extension = process.platform === 'win32' ? '.exe' : '';
  const binary = app.isPackaged ? path.join(process.resourcesPath, 'bin', `spawnt${extension}`) : path.resolve(__dirname, '../../../target/debug', `spawnt${extension}`);
  const child = spawn(binary, ['daemon'], { detached: true, stdio: 'ignore', windowsHide: true, env: daemonEnvironment(process.env) });
  child.on('error', () => {});
  child.unref();
  for (let attempt = 0; attempt < 40; attempt++) {
    await new Promise(resolve => setTimeout(resolve, 250));
    const url = await readDaemon(dataHome());
    if (url) return url;
  }
  throw new Error('The local daemon could not be started. Check the installation and try again.');
}

async function refreshTray(url: string): Promise<void> {
  const parsed = new URL(url);
  const tooltip = await new Promise<string>(resolve => {
    const request = get({ hostname: '127.0.0.1', port: parsed.port, path: '/api/v1/sessions?status=ongoing&limit=1', headers: { Authorization: `Bearer ${parsed.searchParams.get('token')}`, Host: `127.0.0.1:${parsed.port}` }, timeout: 3000 }, response => {
      if (response.statusCode !== 200) { response.resume(); resolve('Spawnt: daemon unavailable'); return; }
      let body = '';
      response.setEncoding('utf8');
      response.on('data', chunk => { body += chunk; if (body.length > 1024 * 1024) request.destroy(); });
      response.on('end', () => { try { resolve(trayTooltip(JSON.parse(body))); } catch { resolve('Spawnt: daemon unavailable'); } });
      response.on('error', () => resolve('Spawnt: daemon unavailable'));
    });
    request.on('timeout', () => request.destroy());
    request.on('error', () => resolve('Spawnt: daemon unavailable'));
  });
  tray?.setToolTip(tooltip);
}
function showWindow(): void {
  if (!window) return;
  if (window.isMinimized()) window.restore();
  window.show();
  window.focus();
}

async function start(): Promise<void> {
  const url = await connectDaemon();
  daemonPort = Number(new URL(url).port || '80');
  const isolatedSession = session.fromPartition('spawnt-local');
  isolatedSession.webRequest.onBeforeRequest((details, callback) => callback({ cancel: !isAllowedResource(details.url, daemonPort) }));
  isolatedSession.setPermissionRequestHandler((_contents, _permission, callback) => callback(false));
  window = new BrowserWindow({
    title: 'Spawnt', width: 1280, height: 820, show: false,
    webPreferences: { preload: path.join(__dirname, 'preload.js'), contextIsolation: true, nodeIntegration: false, sandbox: true, devTools: !app.isPackaged, session: isolatedSession }
  });
  const current = window;
  current.webContents.setWindowOpenHandler(details => {
    if (isAllowedExternalUrl(details.url)) void shell.openExternal(details.url).catch(() => {});
    return { action: 'deny' };
  });
  current.webContents.on('will-navigate', (event, target) => { if (!isDaemonNavigation(target, daemonPort)) event.preventDefault(); });
  current.webContents.on('will-redirect', (event, target) => { if (!isDaemonNavigation(target, daemonPort)) event.preventDefault(); });
  current.on('close', event => { if (!quitting && tray) { event.preventDefault(); current.hide(); } });
  ipcMain.handle('desktop:get-version', (event, ...args: unknown[]) => {
    if (args.length !== 0 || event.sender !== current.webContents || event.senderFrame !== current.webContents.mainFrame || !isDaemonNavigation(event.senderFrame.url, daemonPort)) throw new Error('Invalid IPC sender');
    return app.getVersion();
  });
  ipcMain.handle('desktop:bring-to-front', (event, ...args: unknown[]) => {
    if (args.length !== 0 || event.sender !== current.webContents || event.senderFrame !== current.webContents.mainFrame || !isDaemonNavigation(event.senderFrame.url, daemonPort)) throw new Error('Invalid IPC sender');
    showWindow();
  });
  ipcMain.handle('desktop:launch-session', async (event, agent: unknown, repoPath: unknown, model: unknown) => {
    if (event.sender !== current.webContents || event.senderFrame !== current.webContents.mainFrame || !isDaemonNavigation(event.senderFrame.url, daemonPort)) throw new Error('Invalid IPC sender');
    if (typeof agent !== 'string' || typeof repoPath !== 'string') throw new Error('Invalid launch parameters');
    const parsed = new URL(connectedUrl);
    const postData = JSON.stringify({ agent, repo_path: repoPath, model: typeof model === 'string' ? model : undefined });
    return new Promise((resolve, reject) => {
      const request = get({
        hostname: '127.0.0.1', port: daemonPort, path: '/api/v1/sessions/launch', method: 'POST',
        headers: {
          Authorization: `Bearer ${parsed.searchParams.get('token')}`,
          Host: `127.0.0.1:${daemonPort}`,
          'Content-Type': 'application/json',
          'Content-Length': Buffer.byteLength(postData),
        },
        timeout: 10000,
      }, (res: IncomingMessage) => {
        let body = '';
        res.on('data', (c: Buffer | string) => { body += c.toString(); });
        res.on('end', () => {
          try { resolve(JSON.parse(body)); } catch { resolve({ ok: res.statusCode === 200 }); }
        });
      });
      request.on('error', reject);
      request.write(postData);
      request.end();
    });
  });
  ipcMain.handle('desktop:stop-session', async (event, sessionId: unknown) => {
    if (event.sender !== current.webContents || event.senderFrame !== current.webContents.mainFrame || !isDaemonNavigation(event.senderFrame.url, daemonPort)) throw new Error('Invalid IPC sender');
    if (typeof sessionId !== 'string') throw new Error('Invalid session ID');
    const parsed = new URL(connectedUrl);
    return new Promise((resolve, reject) => {
      const request = get({
        hostname: '127.0.0.1', port: daemonPort, path: `/api/v1/sessions/${encodeURIComponent(sessionId)}/stop`, method: 'POST',
        headers: {
          Authorization: `Bearer ${parsed.searchParams.get('token')}`,
          Host: `127.0.0.1:${daemonPort}`,
        },
        timeout: 5000,
      }, (res: IncomingMessage) => {
        let body = '';
        res.on('data', (c: Buffer | string) => { body += c.toString(); });
        res.on('end', () => {
          try { resolve(JSON.parse(body)); } catch { resolve({ ok: res.statusCode === 200 }); }
        });
      });
      request.on('error', reject);
      request.end();
    });
  });
  const iconName = process.platform === 'win32' ? 'spawnt-tray.ico' : 'spawnt-64.png';
  const icon = app.isPackaged ? path.join(process.resourcesPath, iconName) :
    path.resolve(__dirname, process.platform === 'win32' ? '../../../scripts/spawnt-tray.ico' : '../../../web/public/icons/spawnt-64.png');
  try {
    tray = new Tray(icon);
    tray.setToolTip('Spawnt');
    tray.setContextMenu(Menu.buildFromTemplate([{ label: 'Open Spawnt', click: showWindow }, { label: 'Quit', click: () => { quitting = true; app.quit(); } }]));
    tray.on('double-click', showWindow);
  } catch {
    tray = null;
  }
  await current.loadURL(url);
  showWindow();
  if (!tray) void dialog.showMessageBox(current, { type: 'info', message: 'System tray unavailable', detail: 'Keep this window open to use Spawnt. Closing it quits the desktop shell; the daemon keeps running.' });
  let connectedUrl = url;
  let checking = false;
  const checkDaemon = async (): Promise<void> => {
    if (checking) return;
    checking = true;
    try {
      const nextUrl = await readDaemon(dataHome()) ?? await connectDaemon();
      if (nextUrl !== connectedUrl) {
        daemonPort = Number(new URL(nextUrl).port || '80');
        await current.loadURL(nextUrl);
        connectedUrl = nextUrl;
      }
      await refreshTray(nextUrl);
    } catch {
      tray?.setToolTip('Spawnt: daemon unavailable');
    } finally {
      checking = false;
    }
  };
  void checkDaemon();
  setInterval(() => { void checkDaemon(); }, 5000).unref();
}

app.setName('Spawnt');
if (process.platform === 'win32') app.setAppUserModelId('app.spawnt.desktop');

if (squirrelStartup) app.quit();
else if (!app.requestSingleInstanceLock()) app.quit();
else {
  app.on('second-instance', showWindow);
  app.on('before-quit', () => { quitting = true; });
  app.on('window-all-closed', () => { if (!tray) app.quit(); });
  app.whenReady().then(start).catch(() => {
    dialog.showErrorBox('Spawnt could not start', 'Check that the local daemon is installed and try again.');
    app.exit(1);
  });
}
