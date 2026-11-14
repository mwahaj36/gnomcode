import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { homedir } from 'node:os';
import * as path from 'node:path';

export interface LoginItemAppAdapter {
  getLoginItemSettings(): { openAtLogin?: boolean };
  setLoginItemSettings(settings: { openAtLogin: boolean; args?: string[] }): void;
}

const WIN_RUN_KEY = String.raw`HKCU\Software\Microsoft\Windows\CurrentVersion\Run`;
const WIN_APPROVED_KEY = String.raw`HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run`;
const WIN_VALUE_NAME = 'Gnomcode';

export function parseWindowsRegQueryValue(stdout: string, valueName = WIN_VALUE_NAME): string | null {
  const escaped = valueName.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const match = stdout.match(new RegExp(`^\\s*${escaped}\\s+REG_\\w+\\s+(.+)$`, 'im'));
  return match ? match[1].trim() : null;
}

export function isWindowsStartupApprovedDisabled(stdout: string, valueName = WIN_VALUE_NAME): boolean {
  const raw = parseWindowsRegQueryValue(stdout, valueName);
  if (!raw) return false;
  const firstByte = Number.parseInt(raw.slice(0, 2), 16);
  return Number.isFinite(firstByte) && (firstByte & 1) === 1;
}

export function windowsAutostartCommand(execPath: string): string {
  return `"${execPath}" --background`;
}

export function linuxDesktopFilePath(env: NodeJS.ProcessEnv = process.env, home: string = homedir()): string {
  const configRoot = env.XDG_CONFIG_HOME?.trim() || path.join(home, '.config');
  return path.join(configRoot, 'autostart', 'gnomcode.desktop');
}

export function renderLinuxDesktopEntry(execPath: string): string {
  const escaped = execPath.replace(/\\/g, '\\\\').replace(/"/g, '\\"');
  return `[Desktop Entry]\nType=Application\nName=Gnomcode\nExec="${escaped}" --background\nTerminal=false\nX-GNOME-Autostart-enabled=true\n`;
}

export function getLoginItemEnabled(
  appAdapter: LoginItemAppAdapter,
  platform: NodeJS.Platform = process.platform,
  env: NodeJS.ProcessEnv = process.env,
  home: string = homedir(),
): boolean {
  if (platform === 'win32') {
    try {
      const runOut = execFileSync('reg.exe', ['query', WIN_RUN_KEY, '/v', WIN_VALUE_NAME], {
        encoding: 'utf8',
        windowsHide: true,
        stdio: ['ignore', 'pipe', 'ignore'],
      });
      if (!parseWindowsRegQueryValue(runOut)) return false;
    } catch {
      return false;
    }
    try {
      const approvedOut = execFileSync('reg.exe', ['query', WIN_APPROVED_KEY, '/v', WIN_VALUE_NAME], {
        encoding: 'utf8',
        windowsHide: true,
        stdio: ['ignore', 'pipe', 'ignore'],
      });
      if (isWindowsStartupApprovedDisabled(approvedOut)) return false;
    } catch {
      // No StartupApproved override means the Run entry is active.
    }
    return true;
  }
  if (platform === 'darwin') {
    return Boolean(appAdapter.getLoginItemSettings().openAtLogin);
  }
  const desktopFile = linuxDesktopFilePath(env, home);
  if (!existsSync(desktopFile)) return false;
  try {
    return !/^\s*X-GNOME-Autostart-enabled\s*=\s*false\s*$/im.test(readFileSync(desktopFile, 'utf8'));
  } catch {
    return false;
  }
}

export function setLoginItemEnabled(
  enabled: boolean,
  appAdapter: LoginItemAppAdapter,
  execPath: string,
  platform: NodeJS.Platform = process.platform,
  env: NodeJS.ProcessEnv = process.env,
  home: string = homedir(),
): boolean {
  if (platform === 'win32') {
    if (enabled) {
      execFileSync(
        'reg.exe',
        ['add', WIN_RUN_KEY, '/v', WIN_VALUE_NAME, '/t', 'REG_SZ', '/d', windowsAutostartCommand(execPath), '/f'],
        { windowsHide: true, stdio: 'ignore' },
      );
      try {
        execFileSync('reg.exe', ['delete', WIN_APPROVED_KEY, '/v', WIN_VALUE_NAME, '/f'], {
          windowsHide: true,
          stdio: 'ignore',
        });
      } catch {
        // Value absent is expected when not previously disabled.
      }
    } else {
      try {
        execFileSync('reg.exe', ['delete', WIN_RUN_KEY, '/v', WIN_VALUE_NAME, '/f'], {
          windowsHide: true,
          stdio: 'ignore',
        });
      } catch {
        // Already removed.
      }
    }
    return getLoginItemEnabled(appAdapter, platform, env, home);
  }
  if (platform === 'darwin') {
    appAdapter.setLoginItemSettings({ openAtLogin: enabled, args: enabled ? ['--background'] : [] });
    return getLoginItemEnabled(appAdapter, platform, env, home);
  }
  const desktopFile = linuxDesktopFilePath(env, home);
  if (enabled) {
    mkdirSync(path.dirname(desktopFile), { recursive: true });
    writeFileSync(desktopFile, renderLinuxDesktopEntry(execPath), 'utf8');
  } else {
    rmSync(desktopFile, { force: true });
  }
  return getLoginItemEnabled(appAdapter, platform, env, home);
}
