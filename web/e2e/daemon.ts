import { type ChildProcess, spawn } from 'node:child_process';
import { cp, mkdir, mkdtemp, readFile, rm } from 'node:fs/promises';
import { createServer } from 'node:net';
import { tmpdir } from 'node:os';
import path from 'node:path';

const repoRoot = path.resolve(__dirname, '..', '..');

export interface Daemon {
  /** `http://127.0.0.1:<port>` of the daemon under test. */
  base: string;
  /** Bearer token the console receives through its launch URL. */
  token: string;
  /** Console URL with the token, as the tray and desktop shell open it. */
  consoleUrl: string;
  home: string;
  stop(): Promise<void>;
}

function binaryPath(): string {
  if (process.env.GNOMCODE_BINARY) return path.resolve(process.env.GNOMCODE_BINARY);
  const target = process.env.CARGO_TARGET_DIR ?? path.join(repoRoot, 'target');
  return path.join(target, 'debug', process.platform === 'win32' ? 'gnomcode.exe' : 'gnomcode');
}

async function freePort(): Promise<number> {
  const probe = createServer();
  await new Promise<void>((resolve) => probe.listen(0, '127.0.0.1', resolve));
  const { port } = probe.address() as { port: number };
  await new Promise((resolve) => probe.close(resolve));
  return port;
}

/** Copies anonymized agent logs from `fixtures/` into the temp agent homes the daemon scans. */
async function seed(home: string, desktopClaude: boolean): Promise<void> {
  const fixtures = path.join(repoRoot, 'fixtures');
  const claudeProject = path.join(home, 'claude', 'projects', '-work-console-demo');
  await mkdir(claudeProject, { recursive: true });
  await cp(
    path.join(fixtures, 'claude-code', 'v2.1', 'session_sample.jsonl'),
    path.join(claudeProject, '11111111-1111-4111-8111-111111111111.jsonl'),
  );
  if (desktopClaude) {
    await cp(
      path.join(fixtures, 'claude-code', 'v2.1.321', 'desktop-main.jsonl'),
      path.join(claudeProject, '33333333-3333-4333-8333-333333333333.jsonl'),
    );
  }
  const codexDay = path.join(home, 'codex', 'sessions', '2026', '11', '06');
  await mkdir(codexDay, { recursive: true });
  await cp(
    path.join(fixtures, 'codex', 'v0.162', 'rollout_sample.jsonl'),
    path.join(codexDay, 'rollout-2026-11-06T10-00-00-22222222-2222-4222-8222-222222222222.jsonl'),
  );
}

/** Starts the real daemon on a free loopback port with an isolated `GNOMCODE_HOME` and fixture data. */
export async function startDaemon(options: { seed?: boolean; desktopClaude?: boolean } = {}): Promise<Daemon> {
  const home = await mkdtemp(path.join(tmpdir(), 'gnomcode-e2e-'));
  if (options.seed !== false) await seed(home, options.desktopClaude === true);
  const env = {
    ...process.env,
    GNOMCODE_HOME: home,
    HOME: home,
    USERPROFILE: home,
    CLAUDE_CONFIG_DIR: path.join(home, 'claude'),
    CODEX_HOME: path.join(home, 'codex'),
    // Never read this machine's WSL distros (ADR 0060); tests mock the picker.
    GNOMCODE_DISABLE_WSL: '1',
  };
  const port = await freePort();
  const child: ChildProcess = spawn(binaryPath(), ['daemon', '--port', String(port)], {
    env,
    windowsHide: true,
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  let log = '';
  child.stdout?.on('data', (chunk) => (log += chunk));
  child.stderr?.on('data', (chunk) => (log += chunk));

  let info: { port: number; token: string } | undefined;
  for (let attempt = 0; attempt < 200 && !info; attempt++) {
    try {
      info = JSON.parse(await readFile(path.join(home, 'daemon.json'), 'utf8'));
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
  }
  if (!info) {
    child.kill();
    throw new Error(`daemon did not publish daemon.json:\n${log}`);
  }
  const base = `http://127.0.0.1:${info.port}`;
  for (let attempt = 0; attempt < 200; attempt++) {
    try {
      const response = await fetch(`${base}/health`, { headers: { Authorization: `Bearer ${info.token}` } });
      if (response.ok) break;
    } catch {
      /* not listening yet */
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }

  return {
    base,
    token: info.token,
    consoleUrl: `${base}/?token=${info.token}`,
    home,
    async stop() {
      child.kill();
      await new Promise((resolve) => setTimeout(resolve, 300));
      await rm(home, { recursive: true, force: true }).catch(() => undefined);
    },
  };
}
