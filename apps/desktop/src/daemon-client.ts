import { existsSync } from 'node:fs';
import { homedir } from 'node:os';
import { readFile } from 'node:fs/promises';
import { get, request as httpRequest } from 'node:http';
import * as path from 'node:path';
import { daemonUrl } from './security';

export function dataHome(env: NodeJS.ProcessEnv = process.env, home: string = homedir()): string {
  const override = env.GNOMCODE_HOME?.trim() || env.SPAWNT_HOME?.trim();
  if (override) return override;
  const legacy = path.join(home, '.spawnt');
  return existsSync(legacy) ? legacy : path.join(home, '.gnomcode');
}

export async function readDaemon(home: string): Promise<string | null> {
  try {
    const url = daemonUrl(JSON.parse(await readFile(path.join(home, 'daemon.json'), 'utf8')));
    if (!url) return null;
    const parsed = new URL(url);
    const healthy = await new Promise<boolean>(resolve => {
      const request = get({ hostname: '127.0.0.1', port: parsed.port, path: '/health', headers: { Authorization: `Bearer ${parsed.searchParams.get('token')}`, Host: `127.0.0.1:${parsed.port || '80'}` }, timeout: 1000 }, response => { response.resume(); resolve(response.statusCode === 200); });
      request.on('timeout', () => request.destroy());
      request.on('error', () => resolve(false));
    });
    return healthy ? url : null;
  } catch {
    return null;
  }
}

/** Authenticated request to the local daemon; resolves parsed JSON, or null on any failure (fail open). */
export function daemonRequest(baseUrl: string, method: 'GET' | 'POST', route: string, timeout = 3000): Promise<unknown> {
  return new Promise(resolve => {
    try {
      const parsed = new URL(baseUrl);
      const port = parsed.port || '80';
      const request = httpRequest({ hostname: '127.0.0.1', port, path: route, method, headers: { Authorization: `Bearer ${parsed.searchParams.get('token')}`, Host: `127.0.0.1:${port}` }, timeout }, response => {
        if (response.statusCode !== 200) { response.resume(); resolve(null); return; }
        let body = '';
        response.setEncoding('utf8');
        response.on('data', chunk => { body += chunk; if (body.length > 2 * 1024 * 1024) request.destroy(); });
        response.on('end', () => { try { resolve(JSON.parse(body)); } catch { resolve(null); } });
        response.on('error', () => resolve(null));
      });
      request.on('timeout', () => request.destroy());
      request.on('error', () => resolve(null));
      request.end();
    } catch {
      resolve(null);
    }
  });
}
