import { readFile } from 'node:fs/promises';
import { get } from 'node:http';
import * as path from 'node:path';
import { daemonUrl } from './security';

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
