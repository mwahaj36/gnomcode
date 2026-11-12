import { app, BrowserWindow, dialog, ipcMain, Menu, nativeImage, nativeTheme, Notification, session, shell, Tray } from 'electron';
import squirrelStartup from 'electron-squirrel-startup';
import { spawn, type ChildProcess } from 'node:child_process';
import { get, type IncomingMessage } from 'node:http';
import * as path from 'node:path';
import { activationArguments, workspaceLaunchHash, desktopNotification, daemonEnvironment, isAllowedExternalUrl, isAllowedResource, isDaemonNavigation, trayBadgeName, trayTooltip, windowChrome } from './security';
import { dataHome, readDaemon } from './daemon-client';
import {retainNotification,repairNotificationShortcut} from './notifications';

let window: BrowserWindow | null = null;
let tray: Tray | null = null;
let quitting = false;
let daemonPort = 0;
let spawnedDaemon: ChildProcess | null = null;
const retainedNotifications=new Set<Notification>();

function stopDaemon(): void {
  if (spawnedDaemon && !spawnedDaemon.killed) {
    try {
      if (process.platform === 'win32' && spawnedDaemon.pid) {
        spawn('taskkill', ['/F', '/T', '/PID', String(spawnedDaemon.pid)], { windowsHide: true, stdio: 'ignore' });
      } else {
        spawnedDaemon.kill('SIGTERM');
      }
    } catch (_) {}
  }
}

function notify(args: string[]): boolean {
  const payload=desktopNotification(args);
  if(!payload)return false;
  if(!app.isReady()){void app.whenReady().then(()=>notify(args));return true;}
  if(Notification.isSupported()){
    const notification=new Notification(payload);
    retainNotification(notification,retainedNotifications,showWindow,process.platform);
    notification.show();
  }
  return true;
}

async function connectDaemon(): Promise<string> {
  const running = await readDaemon(dataHome());
  if (running) return running;
  const extension = process.platform === 'win32' ? '.exe' : '';
  const binary = app.isPackaged ? path.join(process.resourcesPath, 'bin', `gnomcode${extension}`) : path.resolve(__dirname, '../../../target/debug', `gnomcode${extension}`);
  const child = spawn(binary, ['daemon'], { detached: true, stdio: 'ignore', windowsHide: true, env: daemonEnvironment(process.env) });
  child.on('error', () => {});
  spawnedDaemon = child;
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
  const status = await new Promise<unknown>(resolve => {
    const request = get({ hostname: '127.0.0.1', port: parsed.port, path: '/api/v1/sessions?status=ongoing&limit=1', headers: { Authorization: `Bearer ${parsed.searchParams.get('token')}`, Host: `127.0.0.1:${parsed.port}` }, timeout: 3000 }, response => {
      if (response.statusCode !== 200) { response.resume(); resolve(null); return; }
      let body = '';
      response.setEncoding('utf8');
      response.on('data', chunk => { body += chunk; if (body.length > 1024 * 1024) request.destroy(); });
      response.on('end', () => { try { resolve(JSON.parse(body)); } catch { resolve(null); } });
      response.on('error', () => resolve(null));
    });
    request.on('timeout', () => request.destroy());
    request.on('error', () => resolve(null));
  });
  tray?.setToolTip(trayTooltip(status));
  if (tray) {
    const badge = trayBadgeName(status);
    const resource = app.isPackaged ? process.resourcesPath : path.resolve(__dirname, '../../../scripts');
    tray.setImage(path.join(resource, badge === 'gnomcode-tray.ico' ? badge : path.join('tray-badges', badge)));
  }
}
function showWindow(): void {
  if (!window) return;
  if (window.isMinimized()) window.restore();
  window.show();
  window.focus();
}

function getAppIcon(): string {
  const iconName = process.platform === 'win32' ? 'gnomcode.ico' : 'gnomcode-64.png';
  return app.isPackaged
    ? path.join(process.resourcesPath, iconName)
    : path.resolve(__dirname, process.platform === 'win32' ? '../../../scripts/gnomcode.ico' : '../../../web/public/icons/gnomcode-64.png');
}

async function start(): Promise<void> {
  const appIconPath = getAppIcon();
  if(process.platform==='win32'){
    // Electron's toast setup (re)creates the Start Menu shortcut without an icon, so repair after it.
    Notification.isSupported();
    repairNotificationShortcut(path.join(app.getPath('appData'),'Microsoft','Windows','Start Menu','Programs','gnomcode.lnk'),process.execPath,shell,appIconPath);
  }
  const baseUrl=await connectDaemon();
  const initial=new URL(baseUrl);initial.hash=workspaceLaunchHash(process.argv) || '';
  const url=initial.href;
  daemonPort = Number(new URL(url).port || '80');
  const isolatedSession = session.fromPartition('persist:gnomcode-local');
  isolatedSession.webRequest.onBeforeRequest((details, callback) => callback({ cancel: !isAllowedResource(details.url, daemonPort) }));
  isolatedSession.setPermissionRequestHandler((_contents, _permission, callback) => callback(false));
  Menu.setApplicationMenu(null);
  const iconImage = nativeImage.createFromPath(appIconPath);
  window = new BrowserWindow({
    ...windowChrome(process.platform, nativeTheme.shouldUseDarkColors),
    icon: iconImage.isEmpty() ? appIconPath : iconImage,
    title: 'Gnomcode', width: 1280, height: 820, show: false,
    webPreferences: { preload: path.join(__dirname, 'preload.js'), contextIsolation: true, nodeIntegration: false, sandbox: true, devTools: !app.isPackaged, session: isolatedSession }
  });
  if (!iconImage.isEmpty()) {
    window.setIcon(iconImage);
  }
  const current = window;
  current.once('ready-to-show', () => {
    if (!notify(process.argv) && !process.argv.includes('--background')) {
      showWindow();
    }
  });
  current.webContents.setWindowOpenHandler(details => {
    if (isAllowedExternalUrl(details.url)) void shell.openExternal(details.url).catch(() => {});
    return { action: 'deny' };
  });
  current.webContents.on('will-navigate', (event, target) => { if (!isDaemonNavigation(target, daemonPort)) event.preventDefault(); });
  current.webContents.on('will-redirect', (event, target) => { if (!isDaemonNavigation(target, daemonPort)) event.preventDefault(); });
  current.on('close', event => {
    if (!quitting && tray) { event.preventDefault(); current.hide(); }
  });
  ipcMain.handle('desktop:set-appearance', (event, theme: unknown, ...extra: unknown[]) => {
    if (extra.length || !['light','dark'].includes(String(theme)) || typeof theme !== 'string' || event.sender !== current.webContents || event.senderFrame !== current.webContents.mainFrame || !isDaemonNavigation(event.senderFrame.url, daemonPort)) throw new Error('Invalid IPC sender or appearance');
    const overlay = windowChrome(process.platform, theme === 'dark').titleBarOverlay;
    if (overlay && typeof overlay === 'object') current.setTitleBarOverlay(overlay);
  });  ipcMain.handle('desktop:get-version', (event, ...args: unknown[]) => {
    if (args.length !== 0 || event.sender !== current.webContents || event.senderFrame !== current.webContents.mainFrame || !isDaemonNavigation(event.senderFrame.url, daemonPort)) throw new Error('Invalid IPC sender');
    return app.getVersion();
  });
  ipcMain.handle('desktop:bring-to-front', (event, ...args: unknown[]) => {
    if (args.length !== 0 || event.sender !== current.webContents || event.senderFrame !== current.webContents.mainFrame || !isDaemonNavigation(event.senderFrame.url, daemonPort)) throw new Error('Invalid IPC sender');
    showWindow();
  });
  ipcMain.handle('desktop:choose-workspace', async (event, ...args: unknown[]) => {
    if (args.length !== 0 || event.sender !== current.webContents || event.senderFrame !== current.webContents.mainFrame || !isDaemonNavigation(event.senderFrame.url, daemonPort)) throw new Error('Invalid IPC sender');
    const result = await dialog.showOpenDialog(current, {title:'Choose workspace',properties:['openDirectory']});
    return result.canceled ? null : result.filePaths[0] || null;
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
  const iconName = process.platform === 'win32' ? 'gnomcode-tray.ico' : 'gnomcode-64.png';
  const icon = app.isPackaged ? path.join(process.resourcesPath, iconName) :
    path.resolve(__dirname, process.platform === 'win32' ? '../../../scripts/gnomcode-tray.ico' : '../../../web/public/icons/gnomcode-64.png');
  try {
    tray = new Tray(icon);
    tray.setToolTip('Gnomcode');
    tray.setContextMenu(Menu.buildFromTemplate([{ label: 'Open Gnomcode', click: showWindow }, { type: 'separator' }, { label: 'Quit Gnomcode', click: () => { quitting = true; stopDaemon(); app.quit(); } }]));
    if(process.platform==='win32')tray.on('click',showWindow);
    tray.on('double-click', showWindow);
  } catch {
    tray = null;
  }
  await current.loadURL(url);
  if(!notify(process.argv) && !process.argv.includes('--background')) {
    showWindow();
  }
  if (!tray) {
    void dialog.showMessageBox(current, { type: 'info', message: 'System tray unavailable', detail: 'Keep this window open to use Gnomcode. Closing it quits the desktop shell; the daemon keeps running.' });
  }
  let connectedUrl = baseUrl;
  let checking = false;
  const checkDaemon = async (): Promise<void> => {
    if (checking) return;
    checking = true;
    try {
      const nextUrl = await readDaemon(dataHome()) ?? await connectDaemon();
      if (nextUrl !== connectedUrl) {
        daemonPort = Number(new URL(nextUrl).port || '80');
        const targetUrl = new URL(nextUrl);
        try {
          const currentUrl = new URL(current.webContents.getURL());
          if (currentUrl.hash) targetUrl.hash = currentUrl.hash;
        } catch (_) {}
        await current.loadURL(targetUrl.href);
        connectedUrl = nextUrl;
      }
      await refreshTray(nextUrl);
    } catch {
      tray?.setToolTip('Gnomcode: daemon unavailable');
    } finally {
      checking = false;
    }
  };
  void checkDaemon();
  setInterval(() => { void checkDaemon(); }, 5000).unref();
}

app.setName('Gnomcode');
// Dev runs use their own identity so electron.exe never claims the installed app's taskbar icon.
if (process.platform === 'win32') app.setAppUserModelId(app.isPackaged ? 'app.gnomcode.desktop' : 'app.gnomcode.desktop.dev');

if (squirrelStartup) {
  app.quit();
} else if (!app.requestSingleInstanceLock({argv:process.argv})) {
  app.quit();
} else {
  app.on('second-instance', (_event,commandLine,_cwd,additionalData) => {
    const args=activationArguments(commandLine,additionalData);
    if(notify(args))return;
    const hash=workspaceLaunchHash(args);
    if(hash && window){const url=new URL(window.webContents.getURL());url.hash=hash;void window.loadURL(url.href);}
    showWindow();
  });
  app.on('before-quit', () => { quitting = true; stopDaemon(); });
  app.on('window-all-closed', () => {
    if (!tray) app.quit();
  });
  app.whenReady().then(start).catch(() => {
    dialog.showErrorBox('Gnomcode could not start', 'Check that the local daemon is installed and try again.');
    app.exit(1);
  });
}
