import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import vm from 'node:vm';
import ts from 'typescript';
const route = new URL('../app/downloads/[asset]/route.ts', import.meta.url);
const { version } = JSON.parse(readFileSync(new URL('../package.json', import.meta.url)));
const compiled = ts.transpileModule(readFileSync(route, 'utf8'), { compilerOptions: { module: ts.ModuleKind.CommonJS, esModuleInterop: true } }).outputText;
const exported = {};
vm.runInNewContext(compiled, { exports: exported, require: createRequire(route), Response });
const get = asset => exported.GET(new Request('https://gnomcode.vercel.app/downloads/' + asset), { params: Promise.resolve({ asset }) });
test('current desktop and CLI release downloads resolve to exact versioned assets', async () => {
  for (const name of ['Gnomcode-Setup.exe', `Gnomcode-v${version}-windows-x64.zip`, `Gnomcode-darwin-arm64-${version}.zip`, `Gnomcode-darwin-x64-${version}.zip`, `Gnomcode-linux-x64-${version}.zip`, `gnomcode-desktop_${version}_amd64.deb`, `gnomcode-v${version}-linux-aarch64.tar.gz`, `gnomcode-v${version}-linux-x86_64-musl.tar.gz`, `Gnomcode-v${version}-windows-arm64-cli.zip`, `SHA256SUMS-v${version}.txt`]) {
    const response = await get(name);
    assert.equal(response.status, 302, name);
    assert.equal(response.headers.get('location'), `https://github.com/mwahaj36/gnomcode/releases/download/v${version}/${name}`);
  }
});
test('unknown versions and paths cannot become arbitrary redirects', async () => {
  for (const name of ['../gnomcode.exe', 'https://example.com', 'Gnomcode-v0.0.0-windows-x64.zip']) assert.equal((await get(name)).status, 404);
});
test('automatic updates fail closed until a signed manifest is available', async () => {
  const response = await get('latest.json');
  assert.equal(response.status, 503);
  assert.match(await response.text(), /automatic update/i);
});
