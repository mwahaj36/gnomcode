import { branding } from './branding';

export function daemonUrl(value: unknown): string | null {
  if (typeof value !== 'object' || value === null) return null;
  const { port, token } = value as Record<string, unknown>;
  if (typeof port !== 'number' || !Number.isInteger(port) || port < 1 || port > 65535 ||
      typeof token !== 'string' || !/^[a-f0-9]{64}$/.test(token)) return null;
  return `http://127.0.0.1:${port}/?token=${token}`;
}

export function isDaemonNavigation(input: string, port: number): boolean {
  try {
    const url = new URL(input);
    return url.protocol === 'http:' && url.hostname === '127.0.0.1' &&
      (url.port || '80') === String(port) && !url.username && !url.password;
  } catch {
    return false;
  }
}

export function daemonEnvironment(env: NodeJS.ProcessEnv): NodeJS.ProcessEnv {
  return { ...env };
}

export function trayTooltip(value: unknown): string {
  if (typeof value !== 'object' || value === null) return 'Spawnt: daemon unavailable';
  const rec = value as Record<string, unknown>;
  const count = rec.ongoing_count;
  if (typeof count !== 'number' || !Number.isSafeInteger(count) || count < 0) return 'Spawnt: daemon unavailable';
  const awaiting = rec.awaiting_approval_count;
  if (typeof awaiting === 'number' && Number.isSafeInteger(awaiting) && awaiting > 0) {
    return `${count} ongoing session${count === 1 ? '' : 's'} · ${awaiting} awaiting approval (estimated)`;
  }
  return count === 0 ? 'No ongoing sessions (estimated)' : `${count} ongoing session${count === 1 ? '' : 's'} (estimated)`;
}

export function trayBadgeName(value: unknown): string {
  const count = value && typeof value === 'object' ? (value as Record<string, unknown>).ongoing_count : null;
  if (typeof count !== 'number' || !Number.isSafeInteger(count) || count < 1) return 'spawnt-tray.ico';
  return `spawnt-tray-${count > 9 ? '9plus' : count}.png`;
}

export function isAllowedResource(input: string, port: number): boolean {
  if (input.startsWith('data:image/')) return true;
  if (input.startsWith('blob:')) {
    try { return new URL(input).origin === new URL(`http://127.0.0.1:${port}`).origin; } catch { return false; }
  }
  try {
    const url = new URL(input);
    if (url.protocol === 'ws:') {
      return url.hostname === '127.0.0.1' && (url.port || '80') === String(port) &&
        !url.username && !url.password && !url.hash &&
        /^\/api\/v1\/sessions\/[^/]+\/pty$/.test(url.pathname);
    }
  } catch { return false; }
  return isDaemonNavigation(input, port);
}

export function isAllowedExternalUrl(input: string): boolean {
  try {
    const url = new URL(input);
    return url.protocol === 'https:' && url.hostname === new URL(branding.baseUrl).hostname &&
      !url.port && !url.username && !url.password && !url.search && (!url.hash || url.hash === '#privacy');
  } catch {
    return false;
  }
}

export function windowChrome(platform: string, dark: boolean): Pick<Electron.BrowserWindowConstructorOptions, 'titleBarStyle' | 'titleBarOverlay' | 'autoHideMenuBar'> {
  return {autoHideMenuBar:true,titleBarStyle:platform === 'darwin' ? 'hiddenInset' : 'hidden',
    ...(platform === 'darwin' ? {} : {titleBarOverlay:{color:dark ? '#0d0d0f' : '#f6f7f8',symbolColor:dark ? '#ededee' : '#15191f',height:32}})};
}
