#!/usr/bin/env node
import { execFileSync, spawn } from 'node:child_process';
import fs from 'node:fs';
import { createRequire } from 'node:module';
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const require = createRequire(import.meta.url);
const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, '../..');

function parseArgs(argv) {
  let binary = process.env.GNOMCODE_SMOKE_BIN || '';
  let trayModule = '';
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === '--binary' && argv[i + 1]) {
      binary = argv[++i];
    } else if (arg === '--tray-module' && argv[i + 1]) {
      trayModule = argv[++i];
    } else if (!arg.startsWith('-') && !binary) {
      binary = arg;
    } else {
      throw new Error(`Unknown argument: ${arg}`);
    }
  }
  return { binary, trayModule };
}

function findPackagedBinary(explicitPath) {
  if (explicitPath) {
    const resolved = path.resolve(explicitPath);
    if (!fs.existsSync(resolved)) {
      throw new Error(`Specified binary not found: ${resolved}`);
    }
    return resolved;
  }

  const exe = process.platform === 'win32' ? 'gnomcode.exe' : 'gnomcode';
  const candidates = [];
  const outDir = path.join(repoRoot, 'apps', 'desktop', 'out');
  if (fs.existsSync(outDir)) {
    for (const entry of fs.readdirSync(outDir, { withFileTypes: true })) {
      if (!entry.isDirectory() || entry.name === 'make') continue;
      candidates.push(path.join(outDir, entry.name, 'resources', 'bin', exe));
      candidates.push(path.join(outDir, entry.name, 'Gnomcode.app', 'Contents', 'Resources', 'bin', exe));
    }
  }
  candidates.push(
    path.join(repoRoot, 'apps', 'desktop', 'staging', 'bin', exe),
    path.join(repoRoot, 'target', 'release', exe),
    path.join(repoRoot, 'target', 'debug', exe),
  );

  const found = candidates.find(candidate => {
    try {
      return fs.statSync(candidate).isFile();
    } catch {
      return false;
    }
  });
  if (!found) {
    throw new Error(`No packaged or built gnomcode binary found. Checked:\n${candidates.join('\n')}`);
  }
  return found;
}

function getFreePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.unref();
    server.on('error', reject);
    server.listen(0, '127.0.0.1', () => {
      const address = server.address();
      if (!address || typeof address === 'string') {
        server.close(() => reject(new Error('Failed to allocate loopback port')));
        return;
      }
      const { port } = address;
      server.close(err => (err ? reject(err) : resolve(port)));
    });
  });
}

async function waitForDaemon(homeDir, port, child, stderrChunks, timeoutMs = 15000) {
  const infoPath = path.join(homeDir, 'daemon.json');
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (child.exitCode !== null) {
      throw new Error(`Daemon exited early (${child.exitCode}): ${stderrChunks.join('')}`);
    }
    if (fs.existsSync(infoPath)) {
      try {
        const info = JSON.parse(fs.readFileSync(infoPath, 'utf8'));
        if (info && Number(info.port) === port && typeof info.token === 'string' && info.token.length > 0) {
          const res = await fetch(`http://127.0.0.1:${port}/health`, {
            headers: {
              Authorization: `Bearer ${info.token}`,
              Host: `127.0.0.1:${port}`,
            },
          });
          if (res.ok) {
            return info;
          }
        }
      } catch {
        // Keep polling until daemon finishes writing daemon.json and binding HTTP.
      }
    }
    await new Promise(resolve => setTimeout(resolve, 150));
  }
  throw new Error(`Timed out waiting for daemon on 127.0.0.1:${port}. stderr: ${stderrChunks.join('')}`);
}

async function requestJson(port, token, route) {
  const res = await fetch(`http://127.0.0.1:${port}${route}`, {
    headers: {
      Authorization: `Bearer ${token}`,
      Host: `127.0.0.1:${port}`,
    },
  });
  if (!res.ok) {
    throw new Error(`GET ${route} failed with HTTP ${res.status}`);
  }
  return res.json();
}

async function requestHtml(port, token, route) {
  const res = await fetch(`http://127.0.0.1:${port}${route}`, {
    headers: {
      Authorization: `Bearer ${token}`,
      Host: `127.0.0.1:${port}`,
    },
  });
  if (!res.ok) {
    throw new Error(`GET ${route} failed with HTTP ${res.status}`);
  }
  const contentType = res.headers.get('content-type') || '';
  if (!contentType.includes('text/html')) {
    throw new Error(`GET ${route} expected text/html, got ${contentType}`);
  }
  return res.text();
}

function verifyTrayTemplate(trayModulePath, providers, sessions) {
  const resolved = trayModulePath
    ? path.resolve(trayModulePath)
    : path.join(repoRoot, 'apps', 'desktop', 'dist', 'tray-menu.js');
  if (!fs.existsSync(resolved)) {
    console.log(`Tray module not built at ${resolved}; skipping tray menu template check.`);
    return;
  }
  const { formatTrayMenu } = require(resolved);
  if (typeof formatTrayMenu !== 'function') {
    throw new Error(`Expected formatTrayMenu export in ${resolved}`);
  }
  const menu = formatTrayMenu(providers, sessions, new Date());
  if (!Array.isArray(menu) || menu.length === 0) {
    throw new Error('formatTrayMenu returned an empty menu');
  }
  const labels = menu.map(item => (item.type === 'separator' ? '---' : item.label));
  for (const required of ['New session', 'Open Gnomcode', 'Quit Gnomcode']) {
    if (!labels.includes(required)) {
      throw new Error(`Tray menu missing required item "${required}". Got: ${JSON.stringify(labels)}`);
    }
  }
  console.log(`Verified tray menu template (${menu.length} items).`);
}

function stopChild(child) {
  if (!child || child.exitCode !== null) return;
  try {
    if (process.platform === 'win32' && child.pid) {
      execFileSync('taskkill', ['/F', '/T', '/PID', String(child.pid)], { stdio: 'ignore' });
    } else {
      child.kill('SIGTERM');
    }
  } catch {
    // Best-effort cleanup.
  }
}

async function main() {
  const { binary: explicitBinary, trayModule } = parseArgs(process.argv.slice(2));
  const binary = findPackagedBinary(explicitBinary);
  const port = await getFreePort();
  const tempHome = fs.mkdtempSync(path.join(os.tmpdir(), 'gnomcode-smoke-'));
  const stderrChunks = [];
  let child = null;

  console.log(`Starting smoke test with binary: ${binary}`);
  console.log(`Temp GNOMCODE_HOME: ${tempHome}, port: ${port}`);

  try {
    child = spawn(binary, ['daemon', '--port', String(port)], {
      env: {
        ...process.env,
        GNOMCODE_HOME: tempHome,
        GNOMCODE_AUTO_UPDATE: '0',
      },
      stdio: ['ignore', 'ignore', 'pipe'],
      windowsHide: true,
    });
    child.stderr.on('data', chunk => stderrChunks.push(chunk.toString('utf8')));

    const info = await waitForDaemon(tempHome, port, child, stderrChunks);
    const doctor = await requestJson(port, info.token, '/api/v1/doctor');
    if (!doctor || !Array.isArray(doctor.checks)) {
      throw new Error('GET /api/v1/doctor did not return checks[]');
    }
    console.log(`/api/v1/doctor ok (${doctor.checks.length} checks)`);

    const overview = await requestJson(port, info.token, '/api/v1/providers/overview');
    if (!Array.isArray(overview)) {
      throw new Error('GET /api/v1/providers/overview did not return a JSON array');
    }
    const providerIds = overview.map(p => p.provider).sort();
    const expectedIds = ['antigravity', 'claude-code', 'codex'];
    if (JSON.stringify(providerIds) !== JSON.stringify(expectedIds)) {
      throw new Error(`Unexpected providers in overview: ${JSON.stringify(providerIds)}`);
    }
    console.log(`/api/v1/providers/overview ok (${providerIds.join(', ')})`);

    const sessions = await requestJson(port, info.token, '/api/v1/sessions');
    if (!sessions || !Array.isArray(sessions.sessions)) {
      throw new Error('GET /api/v1/sessions did not return sessions[]');
    }
    console.log(`/api/v1/sessions ok (${sessions.sessions.length} sessions)`);

    const html = await requestHtml(port, info.token, '/');
    if (!html.toLowerCase().includes('gnomcode') || !html.includes('gnomcode-console-root')) {
      throw new Error('GET / console HTML does not include expected console root');
    }
    console.log('GET / console HTML ok');

    verifyTrayTemplate(trayModule, overview, sessions.sessions);
    console.log('Packaged smoke test PASSED.');
  } finally {
    stopChild(child);
    await new Promise(resolve => setTimeout(resolve, 200));
    fs.rmSync(tempHome, { recursive: true, force: true });
  }
}

main().catch(err => {
  console.error(`Packaged smoke test FAILED: ${err instanceof Error ? err.message : String(err)}`);
  process.exitCode = 1;
});
