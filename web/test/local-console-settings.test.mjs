import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';
import test from 'node:test';

const root = path.resolve(import.meta.dirname, '..', '..');

test('React Settings view exists and owns settings view in local console bundle', () => {
  const settingsPath = path.join(root, 'web', 'local-console', 'SettingsView.tsx');
  const mainPath = path.join(root, 'web', 'local-console', 'main.tsx');
  const bundlePath = path.join(root, 'crates', 'daemon', 'src', 'console-react.js');

  assert.ok(existsSync(settingsPath), 'web/local-console/SettingsView.tsx must exist');
  const settingsSource = readFileSync(settingsPath, 'utf8');
  const mainSource = readFileSync(mainPath, 'utf8');
  const bundle = readFileSync(bundlePath, 'utf8');

  assert.match(settingsSource, /updateProviderSources/);
  assert.match(settingsSource, /\/api\/v1\/settings\/notifications/);
  assert.match(settingsSource, /\/api\/v1\/rules/);
  assert.match(settingsSource, /\/api\/v1\/remote\/devices/);
  assert.match(mainSource, /settings:\s*true/);
  assert.match(bundle, /Devices and Remote/);
  assert.doesNotMatch(bundle, /sourceMappingURL/i);
});
