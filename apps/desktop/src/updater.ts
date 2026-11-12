import { spawn, type SpawnOptions } from 'node:child_process';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';

export interface DesktopUpdateStatus {
  state: string;
  available_version: string | null;
  downloaded_path: string | null;
  sha256: string | null;
  auto_update_enabled: boolean;
}

const ALLOWED_INSTALLER_EXTENSIONS = new Set([
  '.exe',
  '.msi',
  '.zip',
  '.dmg',
  '.deb',
  '.rpm',
  '.appimage',
]);

export type InstallerSpawner = (
  command: string,
  args: readonly string[],
  options: SpawnOptions,
) => { unref(): void };

export function parseUpdateStatus(raw: unknown): DesktopUpdateStatus | null {
  if (!raw || typeof raw !== 'object') return null;
  const record = raw as Record<string, unknown>;
  if (typeof record.state !== 'string') return null;
  return {
    state: record.state,
    available_version: typeof record.available_version === 'string' ? record.available_version : null,
    downloaded_path: typeof record.downloaded_path === 'string' ? record.downloaded_path : null,
    sha256: typeof record.sha256 === 'string' ? record.sha256 : null,
    auto_update_enabled: record.auto_update_enabled !== false,
  };
}

export function verifyStagedInstaller(
  status: DesktopUpdateStatus | null,
  updatesRoot: string,
): string | null {
  if (!status || status.state !== 'ready' || !status.downloaded_path || !status.sha256) {
    return null;
  }
  const expectedHash = status.sha256.trim().toLowerCase();
  if (!/^[0-9a-f]{64}$/.test(expectedHash)) return null;

  try {
    const rootReal = fs.realpathSync(updatesRoot);
    const fileReal = fs.realpathSync(status.downloaded_path);
    const rel = path.relative(rootReal, fileReal);
    if (!rel || rel.startsWith('..') || path.isAbsolute(rel)) return null;
    if (!ALLOWED_INSTALLER_EXTENSIONS.has(path.extname(fileReal).toLowerCase())) return null;
    if (!fs.statSync(fileReal).isFile()) return null;

    const actualHash = crypto.createHash('sha256').update(fs.readFileSync(fileReal)).digest('hex');
    if (actualHash !== expectedHash) return null;
    return fileReal;
  } catch {
    return null;
  }
}

export function launchVerifiedInstaller(
  stagedPath: string,
  spawner: InstallerSpawner = (cmd, args, opts) => spawn(cmd, [...args], opts),
): boolean {
  try {
    const child = spawner(stagedPath, [], {
      detached: true,
      stdio: 'ignore',
      windowsHide: false,
    });
    child.unref();
    return true;
  } catch {
    return false;
  }
}
