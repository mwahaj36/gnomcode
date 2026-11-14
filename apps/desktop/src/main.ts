import { app, BrowserWindow, clipboard, dialog, ipcMain, Menu, nativeImage, nativeTheme, Notification, session, shell, Tray, type MenuItemConstructorOptions } from 'electron';
import squirrelStartup from 'electron-squirrel-startup';
import { spawn, type ChildProcess } from 'node:child_process';
import { get, type IncomingMessage } from 'node:http';
import * as path from 'node:path';
import { activationArguments, workspaceLaunchHash, desktopNotification, daemonEnvironment, isAllowedExternalUrl, isAllowedResource, isDaemonNavigation, trayBadgeName, trayTooltip, windowChrome } from './security';
import { dataHome, readDaemon, daemonRequest } from './daemon-client';
import { formatTrayMenu, formatTrayTooltip, type TrayAction, type TrayItem, type TrayProvider, type TraySession } from './tray-menu';
import {retainNotification,repairNotificationShortcut} from './notifications';
import { launchVerifiedInstaller, parseUpdateStatus, verifyStagedInstaller, type DesktopUpdateStatus } from './updater';
import { getLoginItemEnabled, setLoginItemEnabled } from './autostart';

let window: BrowserWindow | null = null;
let tray: Tray | null = null;
let quitting = false;
let daemonPort = 0;
let spawnedDaemon: ChildProcess | null = null;
const retainedNotifications=new Set<Notification>();
let trayMenuOpen = false;
let trayMenuKey = '';
let trayLimitsTooltip: string | null = null;
let trayStatusTooltip = 'Gnomcode';

function stopDaemon(): void {
  if (spawnedDaemon && !spawnedDaemon.killed) {
    try {
      if (process.platform === 'win32' && spawnedDaemon.pid) {
        spawn('taskkill', ['/F', '/T', '/PID', String(spawnedDaemon.pid)], { windowsHide: true, stdio: 'ignore' });
      } else {
        spawnedDaemon.kill('SIGTERM');
      }
    } catch { /* daemon already exited; nothing to stop */ }
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

const DAEMON_CONNECT_ATTEMPTS = 160;
const DAEMON_CONNECT_INTERVAL_MS = 250;

function spawnDaemonBinary(): { child: ChildProcess; exitedEarly: () => boolean } {
  const extension = process.platform === 'win32' ? '.exe' : '';
  const binary = app.isPackaged
    ? path.join(process.resourcesPath, 'bin', `gnomcode${extension}`)
    : path.resolve(__dirname, '../../../target/debug', `gnomcode${extension}`);
  let exited = false;
  const child = spawn(binary, ['daemon'], {
    detached: true,
    stdio: 'ignore',
    windowsHide: true,
    env: daemonEnvironment(process.env),
  });
  child.on('error', () => { exited = true; });
  child.once('exit', () => { exited = true; });
  spawnedDaemon = child;
  child.unref();
  return { child, exitedEarly: () => exited };
}

async function connectDaemon(): Promise<string> {
  const running = await readDaemon(dataHome());
  if (running) return running;
  let spawned = spawnDaemonBinary();
  let retried = false;
  for (let attempt = 0; attempt < DAEMON_CONNECT_ATTEMPTS; attempt++) {
    await new Promise(resolve => setTimeout(resolve, DAEMON_CONNECT_INTERVAL_MS));
    const url = await readDaemon(dataHome());
    if (url) return url;
    // Retry spawning once if the first boot spawn exited prematurely during AV/login contention.
    if (spawned.exitedEarly() && !retried) {
      retried = true;
      await new Promise(resolve => setTimeout(resolve, 500));
      const raceUrl = await readDaemon(dataHome());
      if (raceUrl) return raceUrl;
      spawned = spawnDaemonBinary();
    }
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
  trayStatusTooltip = trayTooltip(status);
  tray?.setToolTip(trayLimitsTooltip ?? trayStatusTooltip);
  if (tray) {
    const badge = trayBadgeName(status);
    const resource = app.isPackaged ? process.resourcesPath : path.resolve(__dirname, '../../../scripts');
    tray.setImage(path.join(resource, badge === 'gnomcode-tray.ico' ? badge : path.join('tray-badges', badge)));
  }
}
let currentDaemonUrl = '';
let latestTrayTemplate: TrayItem[] | null = null;
let trayMenuOpenedAt = 0;

let latestUpdateStatus: DesktopUpdateStatus | null = null;
let installLaunched = false;

function maybeInstallUpdateOnExit(): boolean {
  if (installLaunched || !latestUpdateStatus) return false;
  const verified = verifyStagedInstaller(latestUpdateStatus, path.join(dataHome(), 'updates'));
  if (!verified) return false;
  installLaunched = true;
  if (!launchVerifiedInstaller(verified)) {
    installLaunched = false;
    return false;
  }
  return true;
}

function restartToUpdate(): boolean {
  if (!maybeInstallUpdateOnExit()) return false;
  quitting = true;
  stopDaemon();
  app.quit();
  return true;
}

function openView(view: 'usage' | 'sessions' | 'settings'): void {
  showWindow();
  if (!window) return;
  try {
    const target = new URL(window.webContents.getURL());
    target.hash = view;
    void window.loadURL(target.href).catch(() => {});
  } catch {
    // The window may have no page yet; showing it is enough.
  }
}

function runTrayAction(action: TrayAction): void {
  switch (action.kind) {
    case 'open': showWindow(); break;
    case 'quit': maybeInstallUpdateOnExit(); quitting = true; stopDaemon(); app.quit(); break;
    case 'restart-to-update': restartToUpdate(); break;
    case 'open-view': openView(action.view); break;
    case 'copy': clipboard.writeText(action.text); break;
    case 'focus-session':
      void daemonRequest(currentDaemonUrl, 'POST', `/api/v1/sessions/${encodeURIComponent(action.id)}/focus`, 5000).then(result => {
        if (!(result && typeof result === 'object' && (result as { focused?: unknown }).focused === true)) openView('sessions');
      });
      break;
  }
}

function toMenuItem(item: TrayItem): MenuItemConstructorOptions {
  if (item.type === 'separator') return { type: 'separator' };
  const options: MenuItemConstructorOptions = { label: item.label, enabled: item.enabled !== false };
  if (item.submenu) options.submenu = item.submenu.map(toMenuItem);
  const action = item.action;
  if (action) options.click = () => runTrayAction(action);
  return options;
}

// Rebuilding a native menu while it is open is unsafe on Windows, so a changed template waits for menu-will-close.
function applyTrayMenu(): void {
  const menuOpen = trayMenuOpen && Date.now() - trayMenuOpenedAt < 120000;
  if (!tray || !latestTrayTemplate || menuOpen) return;
  const key = JSON.stringify(latestTrayTemplate);
  if (key === trayMenuKey) return;
  const menu = Menu.buildFromTemplate(latestTrayTemplate.map(toMenuItem));
  menu.on('menu-will-show', () => { trayMenuOpen = true; trayMenuOpenedAt = Date.now(); });
  menu.on('menu-will-close', () => { trayMenuOpen = false; setImmediate(applyTrayMenu); });
  tray.setContextMenu(menu);
  trayMenuKey = key;
}

async function refreshTrayMenu(): Promise<void> {
  const [overview, sessions, updateRaw] = currentDaemonUrl
    ? await Promise.all([
        daemonRequest(currentDaemonUrl, 'GET', '/api/v1/providers/overview'),
        daemonRequest(currentDaemonUrl, 'GET', '/api/v1/sessions?status=ongoing&limit=100'),
        daemonRequest(currentDaemonUrl, 'GET', '/api/v1/update/status'),
      ])
    : [null, null, null];
  const providers = Array.isArray(overview) ? overview as TrayProvider[] : null;
  const list = sessions && typeof sessions === 'object' && Array.isArray((sessions as { sessions?: unknown }).sessions) ? (sessions as { sessions: TraySession[] }).sessions : null;
  latestUpdateStatus = parseUpdateStatus(updateRaw);
  latestTrayTemplate = formatTrayMenu(providers, list, new Date(), { update: latestUpdateStatus });
  trayLimitsTooltip = providers ? formatTrayTooltip(providers) : null;
  tray?.setToolTip(trayLimitsTooltip ?? trayStatusTooltip);
  applyTrayMenu();
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
  let connectedUrl = baseUrl;
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
  ipcMain.handle('desktop:restart-to-update', (event, ...args: unknown[]) => {
    if (args.length !== 0 || event.sender !== current.webContents || event.senderFrame !== current.webContents.mainFrame || !isDaemonNavigation(event.senderFrame.url, daemonPort)) throw new Error('Invalid IPC sender');
    return restartToUpdate();
  });
  ipcMain.handle('desktop:get-login-item', (event, ...args: unknown[]) => {
    if (args.length !== 0 || event.sender !== current.webContents || event.senderFrame !== current.webContents.mainFrame || !isDaemonNavigation(event.senderFrame.url, daemonPort)) throw new Error('Invalid IPC sender');
    return getLoginItemEnabled(app);
  });
  ipcMain.handle('desktop:set-login-item', (event, enabled: unknown, ...extra: unknown[]) => {
    if (extra.length !== 0 || typeof enabled !== 'boolean' || event.sender !== current.webContents || event.senderFrame !== current.webContents.mainFrame || !isDaemonNavigation(event.senderFrame.url, daemonPort)) throw new Error('Invalid IPC sender or login item state');
    return setLoginItemEnabled(enabled, app, process.execPath);
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
    currentDaemonUrl = baseUrl;
    latestTrayTemplate = formatTrayMenu(null, null, new Date());
    applyTrayMenu();
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
        } catch { /* unparsable current URL; keep target without hash */ }
        await current.loadURL(targetUrl.href);
        connectedUrl = nextUrl;
      }
      currentDaemonUrl = nextUrl;
      await refreshTray(nextUrl);
    } catch {
      currentDaemonUrl = '';
      trayLimitsTooltip = null;
      latestTrayTemplate = formatTrayMenu(null, null, new Date());
      applyTrayMenu();
      tray?.setToolTip('Gnomcode: daemon unavailable');
    } finally {
      checking = false;
    }
  };
  void checkDaemon();
  setInterval(() => { void checkDaemon(); }, 5000).unref();
  void refreshTrayMenu();
  setInterval(() => { void refreshTrayMenu(); }, 30000).unref();
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
  app.on('before-quit', () => { maybeInstallUpdateOnExit(); quitting = true; stopDaemon(); });
  app.on('window-all-closed', () => {
    if (!tray) app.quit();
  });
  app.whenReady().then(start).catch(() => {
    dialog.showErrorBox('Gnomcode could not start', 'Check that the local daemon is installed and try again.');
    app.exit(1);
  });
}
