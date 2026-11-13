import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

const root = path.resolve(import.meta.dirname, '..', '..');

test('F2 imported session Continue here UI exists in WorkspaceView and RemoteCompanion', () => {
  const workspaceSrc = fs.readFileSync(path.join(root, 'web', 'local-console', 'WorkspaceView.tsx'), 'utf8');
  assert.match(workspaceSrc, /continue-here/);
  assert.match(workspaceSrc, /Continue here/);
  assert.match(workspaceSrc, /continueHereBusy/);

  const remoteSrc = fs.readFileSync(path.join(root, 'web', 'components', 'RemoteCompanion.tsx'), 'utf8');
  assert.match(remoteSrc, /continue_here/);
  assert.match(remoteSrc, /Continue here/);
  assert.match(remoteSrc, /continue_here_ack/);
});
