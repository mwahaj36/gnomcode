import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';
import test from 'node:test';

const root = path.resolve(import.meta.dirname, '..', '..');

test('React Usage view exists and owns usage view in local console bundle', () => {
  const usagePath = path.join(root, 'web', 'local-console', 'UsageView.tsx');
  const mainPath = path.join(root, 'web', 'local-console', 'main.tsx');
  const bundlePath = path.join(root, 'crates', 'daemon', 'src', 'console-react.js');

  assert.ok(existsSync(usagePath), 'web/local-console/UsageView.tsx must exist');
  const usageSource = readFileSync(usagePath, 'utf8');
  const mainSource = readFileSync(mainPath, 'utf8');
  const bundle = readFileSync(bundlePath, 'utf8');

  assert.match(usageSource, /updateProviderCards/);
  assert.match(usageSource, /updateHeatmap/);
  assert.match(usageSource, /updateActivity/);
  assert.match(usageSource, /updateProviders/);
  assert.match(usageSource, /updateValue/);
  assert.match(usageSource, /mountProjectList/);
  assert.match(mainSource, /usage:\s*true/);
  assert.match(bundle, /Reporting period/);
  assert.doesNotMatch(bundle, /sourceMappingURL/i);
});
