import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';

test('deployment excludes engineering docs while retaining nested website guide routes', () => {
  const root = fileURLToPath(new URL('../../', import.meta.url));
  const ignore = fileURLToPath(new URL('../../.vercelignore', import.meta.url));
  const options = { cwd: root, encoding: 'utf8' };
  const paths = ['docs/SPEC.md', 'web/app/docs/page.tsx', 'web/app/docs/getting-started/page.tsx'];
  const result = spawnSync('git', ['-c', `core.excludesFile=${ignore}`, 'check-ignore', '--no-index', '--stdin'], { ...options, input: paths.join('\n') });
  assert.ok([0, 1].includes(result.status), result.stderr);
  assert.deepEqual(result.stdout.trim().split(/\r?\n/), ['docs/SPEC.md']);
});
