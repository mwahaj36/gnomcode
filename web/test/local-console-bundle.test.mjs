import assert from 'node:assert/strict';
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import path from 'node:path';
import test from 'node:test';

const root = path.resolve(import.meta.dirname, '..', '..');
const views = path.join(root, 'web', 'local-console');

test('daemon embeds and serves the web/ React console bundle without sourcemaps', () => {
  const api = readFileSync(path.join(root, 'crates', 'daemon', 'src', 'api.rs'), 'utf8');
  const html = readFileSync(path.join(root, 'crates', 'daemon', 'src', 'console.html'), 'utf8');
  const bundlePath = path.join(root, 'crates', 'daemon', 'src', 'console-react.js');

  assert.match(api, /\bmod console_react;/, 'api.rs must register mod console_react');
  assert.match(api, /\.route\("\/console-react\.js"/, 'api.rs must route /console-react.js');
  assert.ok(existsSync(bundlePath), 'crates/daemon/src/console-react.js must exist');
  assert.ok(html.includes('/console-react.js'), 'console.html must load /console-react.js');

  const bundle = readFileSync(bundlePath, 'utf8');
  assert.doesNotMatch(bundle, /sourceMappingURL/i, 'console-react.js must not embed sourcemaps');
  assert.doesNotMatch(bundle, /\.js\.map\b/i, 'console-react.js must not reference .js.map files');
});

test('React views format and display; pricing, cache and quota math stays in Rust', () => {
  const sources = readdirSync(views).filter((name) => /\.tsx?$/.test(name));
  assert.ok(sources.length >= 5);
  for (const name of sources) {
    const source = readFileSync(path.join(views, name), 'utf8');
    assert.doesNotMatch(source, /perMillion|cache_savings_usd\s*\*|remaining_pct\s*\//, `${name} must not compute reporting math`);
  }
});
