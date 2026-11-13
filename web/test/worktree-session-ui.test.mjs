import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

const root = path.resolve(import.meta.dirname, '..', '..');

test('worktree presentation helper and WorkspaceView / RemoteCompanion expose F1 worktree launch, diff review, and merge/discard/PR actions', async () => {
  const helperPath = path.join(root, 'web', 'lib', 'worktree-presentation.mjs');
  assert.equal(fs.existsSync(helperPath), true, 'web/lib/worktree-presentation.mjs must exist');
  const mod = await import(`file://${helperPath}`);
  assert.equal(typeof mod.renderWorktreeDiffMarkup, 'function');
  assert.equal(typeof mod.formatWorktreeBadge, 'function');

  const emptyMarkup = mod.renderWorktreeDiffMarkup({ diff: '', status: '', repo: 'C:\\repo\\demo' });
  assert.match(emptyMarkup, /No uncommitted changes/i);

  const diffMarkup = mod.renderWorktreeDiffMarkup({
    repo: 'C:\\repo\\demo',
    status: ' M src/main.rs\n?? new.txt',
    diff: 'diff --git a/src/main.rs b/src/main.rs\n@@ -1 +1 @@\n-old\n+new',
  });
  assert.match(diffMarkup, /src\/main\.rs/);
  assert.match(diffMarkup, /sp-diff-line--add/);
  assert.match(diffMarkup, /sp-diff-line--del/);

  const xssMarkup = mod.renderWorktreeDiffMarkup({
    repo: '<script>alert(1)</script>',
    status: ' M <img/src=x>',
    diff: '+<script>xss</script>',
  });
  assert.equal(xssMarkup.includes('<script>'), false);

  const workspaceSrc = fs.readFileSync(path.join(root, 'web', 'local-console', 'WorkspaceView.tsx'), 'utf8');
  assert.match(workspaceSrc, /worktree_branch/);
  assert.match(workspaceSrc, /Isolate in git worktree/);
  assert.match(workspaceSrc, /worktree\/\$\{action\}/);
  assert.match(workspaceSrc, /'merge' \| 'discard' \| 'open-pr'/);

  const remoteSrc = fs.readFileSync(path.join(root, 'web', 'components', 'RemoteCompanion.tsx'), 'utf8');
  assert.match(remoteSrc, /worktree_merge/);
  assert.match(remoteSrc, /worktree_discard/);
  assert.match(remoteSrc, /worktree_open_pr/);
});
