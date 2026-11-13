import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';
import test from 'node:test';

const root = path.resolve(import.meta.dirname, '..', '..');

test('React Workspace view exists and owns sessions view in local console bundle', () => {
  const workspacePath = path.join(root, 'web', 'local-console', 'WorkspaceView.tsx');
  const mainPath = path.join(root, 'web', 'local-console', 'main.tsx');
  const bundlePath = path.join(root, 'crates', 'daemon', 'src', 'console-react.js');

  assert.ok(existsSync(workspacePath), 'web/local-console/WorkspaceView.tsx must exist');
  const workspaceSource = readFileSync(workspacePath, 'utf8');
  const mainSource = readFileSync(mainPath, 'utf8');
  const bundle = readFileSync(bundlePath, 'utf8');

  assert.match(workspaceSource, /renderSessionList/);
  assert.match(workspaceSource, /reconcileChat/);
  assert.match(workspaceSource, /updateChat/);
  assert.match(workspaceSource, /renderConversationLimits/);
  assert.match(mainSource, /sessions:\s*true/);
  assert.match(bundle, /New agent session/);
  assert.doesNotMatch(bundle, /sourceMappingURL/i);
});
