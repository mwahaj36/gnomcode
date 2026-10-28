#!/usr/bin/env node
'use strict';
const { spawnSync } = require('node:child_process');
const path = require('node:path');
const { baseUrl } = require('./branding.json');
const report = process.platform === 'linux' && process.report ? process.report.getReport().header : null;
const musl = report && !report.glibcVersionRuntime;
const suffix = `${process.platform}-${process.arch}${musl ? '-musl' : ''}`;
const supported = new Set(['win32-x64', 'win32-arm64', 'darwin-x64', 'darwin-arm64', 'linux-x64', 'linux-arm64', 'linux-x64-musl']);
if (!supported.has(suffix)) {
  console.error(`Spawnt does not provide a binary for ${suffix}. See ${baseUrl} for installation options.`);
  process.exit(1);
}
const name = `@spawnt/cli-${suffix}`;
let packageFile;
try {
  packageFile = require.resolve(`${name}/package.json`);
} catch {
  console.error(`Missing ${name}. npm may have skipped optional dependencies. Reinstall without --no-optional or --omit=optional, or see ${baseUrl} for installation options.`);
  process.exit(1);
}
const binary = path.join(path.dirname(packageFile), 'bin', process.platform === 'win32' ? 'spawnt.exe' : 'spawnt');
const result = spawnSync(binary, process.argv.slice(2), { stdio: 'inherit' });
if (result.error) {
  console.error(`Could not start ${binary}: ${result.error.message}`);
  process.exit(1);
}
if (result.signal) process.kill(process.pid, result.signal);
process.exit(result.status ?? 1);
