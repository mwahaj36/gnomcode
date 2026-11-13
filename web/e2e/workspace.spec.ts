import type { Page } from '@playwright/test';
import { expect, test } from './fixtures';

type Row = Record<string, unknown>;

/**
 * Rewrites the daemon's real conversation list so the Claude fixture session looks like one Gnomcode
 * launched (managed), optionally running and with a workspace folder. Everything else is the real daemon.
 */
async function reshapeConversations(page: Page, patch: Row) {
  await page.route('**/api/v1/conversations?*', async (route) => {
    try {
      const response = await route.fetch();
      const body = await response.json();
      const rows: Row[] = Array.isArray(body) ? body : body.sessions;
      for (const row of rows) if (String(row.id).startsWith('claude-code:')) Object.assign(row, patch);
      await route.fulfill({ response, json: body });
    } catch {
      /* the test ended while a poll was in flight */
    }
  });
}

async function openClaudeConversation(page: Page) {
  await page.locator('.ide-session', { hasText: 'claude-code' }).click();
  await expect(page.locator('.ide-session[aria-current="true"]')).toHaveCount(1);
}

test.describe('read-only workspace panels on an observed session', () => {
  test('Files lists a workspace folder and opens a file read-only', async ({ page, openConsole }) => {
    await reshapeConversations(page, { repo_path: '/work/console-demo' });
    const requests: string[] = [];
    await page.route('**/api/v1/repos/files?*', async (route) => {
      requests.push(route.request().url());
      await route.fulfill({
        json: { path: '', entries: [{ name: 'src', path: 'src', kind: 'dir' }, { name: 'README.md', path: 'README.md', kind: 'file', size: 2048 }] },
      });
    });
    await page.route('**/api/v1/repos/file?*', (route) =>
      route.fulfill({ json: { path: 'README.md', content: '# Demo\n<b>not markup</b>\n', truncated: false, lossy: false } }),
    );
    await openConsole('#sessions');
    await openClaudeConversation(page);
    const tools = page.getByRole('toolbar', { name: 'Conversation tools' });
    await tools.getByRole('button', { name: 'Files' }).click();
    const panel = page.locator('.sp-files-panel');
    await expect(panel.getByRole('heading', { name: 'Workspace files' })).toBeVisible();
    await expect(panel.getByRole('button', { name: /README\.md/ })).toBeVisible();
    expect(requests[0]).toContain('repo=%2Fwork%2Fconsole-demo');
    await panel.getByRole('button', { name: /README\.md/ }).click();
    await expect(panel.locator('.sp-file-code')).toContainText('# Demo');
    await expect(panel.locator('.sp-file-code')).toContainText('<b>not markup</b>');
    await expect(panel.locator('.sp-file-code b')).toHaveCount(0);
    await expect(panel.getByText('Read-only')).toBeVisible();
    await expect(panel.locator('textarea, [contenteditable]')).toHaveCount(0);
    // A path outside the workspace never reaches the daemon.
    let escaped = false;
    page.on('request', (request) => {
      if (request.url().includes('/api/v1/repos/file?') && request.url().includes('..')) escaped = true;
    });
    await panel.getByLabel('Relative workspace file path').fill('../secret.txt');
    await panel.getByRole('button', { name: 'Open' }).click();
    await expect(panel.getByRole('alert')).toBeVisible();
    expect(escaped).toBe(false);
    await panel.getByRole('button', { name: 'Close' }).click();
    await expect(panel).toHaveCount(0);
  });

  test('Files explains a session without a workspace folder', async ({ page, openConsole }) => {
    await openConsole('#sessions');
    await page.locator('.ide-session').first().click();
    await page.getByRole('toolbar', { name: 'Conversation tools' }).getByRole('button', { name: 'Files' }).click();
    await expect(page.locator('.sp-files-panel [role="alert"]')).toContainText('Workspace repository path is unavailable for this session.');
  });

  test('Changes shows the status and diff without any revert control', async ({ page, openConsole }) => {
    await page.route('**/api/v1/sessions/*/changes', (route) =>
      route.fulfill({
        json: { repo: '/work/console-demo', status: ' M src/app.ts\n?? notes.md\n', diff: 'diff --git a/src/app.ts b/src/app.ts\n-old\n+new\n', lossy: false },
      }),
    );
    await openConsole('#sessions');
    await openClaudeConversation(page);
    await page.getByRole('toolbar', { name: 'Conversation tools' }).getByRole('button', { name: 'Changes' }).click();
    const panel = page.locator('.sp-changes-panel');
    await expect(panel.getByRole('heading', { name: 'Workspace changes' })).toBeVisible();
    await expect(panel.getByRole('list', { name: 'Changed files' })).toContainText('src/app.ts');
    await expect(panel.getByRole('list', { name: 'Changed files' })).toContainText('notes.md');
    await expect(panel.locator('.sp-diff-code')).toContainText('+new');
    await expect(panel.getByRole('button', { name: /revert|discard|checkout/i })).toHaveCount(0);
    await expect(panel.getByRole('button', { name: 'Refresh' })).toBeVisible();
  });

  test('Changes reports a workspace that is not a git repository', async ({ page, openConsole }) => {
    await page.route('**/api/v1/sessions/*/changes', (route) => route.fulfill({ status: 400, json: { message: 'not a repo' } }));
    await openConsole('#sessions');
    await openClaudeConversation(page);
    await page.getByRole('toolbar', { name: 'Conversation tools' }).getByRole('button', { name: 'Changes' }).click();
    await expect(page.locator('.sp-changes-panel [role="alert"]')).toContainText('This workspace is not a git repository.');
  });

  test('Artifacts previews HTML inside an empty sandbox and never runs its script', async ({ page, openConsole }) => {
    await page.route('**/api/v1/sessions/*/artifacts', (route) =>
      route.fulfill({
        json: {
          changes_count: 2,
          artifacts: [
            { id: 'a1', path: 'out/preview.html', kind: 'html', status: 'M', content: '<h1>Hi</h1><script>window.parent.hacked=1</script>' },
            { id: 'a2', path: 'docs/notes.md', kind: 'markdown', status: 'A', content: '# Notes\n\n<img src=x onerror=window.hacked=1>' },
          ],
        },
      }),
    );
    await openConsole('#sessions');
    await openClaudeConversation(page);
    await page.getByRole('toolbar', { name: 'Conversation tools' }).getByRole('button', { name: 'Artifacts' }).click();
    const panel = page.locator('.sp-artifacts-panel');
    await expect(panel.getByRole('status')).toHaveText('2 workspace changes');
    await expect(panel.locator('iframe')).toHaveAttribute('sandbox', '');
    await panel.getByRole('button', { name: /notes\.md/ }).click();
    await expect(panel.locator('.sp-artifact-preview')).toContainText('Notes');
    await expect(panel.locator('.sp-artifact-preview img')).toHaveCount(0);
    expect(await page.evaluate(() => (window as unknown as { hacked?: number }).hacked)).toBeUndefined();
  });
});

test.describe('composer on a conversation Gnomcode manages', () => {
  test('quick prompts send plain text through the prompt endpoint', async ({ page, openConsole }) => {
    await reshapeConversations(page, { source: 'managed' });
    const sent: { url: string; body: string | null }[] = [];
    await page.route('**/api/v1/sessions/*/prompt', async (route) => {
      sent.push({ url: route.request().url(), body: route.request().postData() });
      await route.fulfill({ json: { ok: true } });
    });
    await openConsole('#sessions');
    await openClaudeConversation(page);
    const bar = page.getByRole('toolbar', { name: 'Quick prompts' });
    await expect(bar.getByRole('button', { name: 'Run tests' })).toBeVisible();
    await bar.getByRole('button', { name: 'Run tests' }).click();
    await expect.poll(() => sent.length).toBe(1);
    expect(sent[0].url).toMatch(/\/api\/v1\/sessions\/claude-code%3A[^/]+\/prompt$/);
    const payload = JSON.parse(sent[0].body ?? '{}');
    expect(Object.keys(payload)).toEqual(['text']);
    expect(String(payload.text)).toMatch(/test/i);
  });

  test('quick prompts can be added, removed and reset in the management dialog', async ({ page, openConsole }) => {
    await reshapeConversations(page, { source: 'managed' });
    await openConsole('#sessions');
    await openClaudeConversation(page);
    await page.getByRole('button', { name: 'Manage quick prompts' }).click();
    const dialog = page.locator('dialog.sp-dialog');
    await expect(dialog).toBeVisible();
    await dialog.getByLabel('Button label').fill('Lint');
    await dialog.getByLabel('Prompt message').fill('Run the linter and fix what it reports.');
    await dialog.getByRole('button', { name: 'Add prompt' }).click();
    await expect(dialog).toContainText('Lint');
    await dialog.getByRole('button', { name: 'Reset defaults' }).click();
    await expect(dialog).not.toContainText('Lint');
    await dialog.getByRole('button', { name: 'Close' }).first().click();
    await expect(dialog).not.toBeVisible();
  });

  test('Attach uploads an allowed file and rejects an unsupported one before any request', async ({ page, openConsole }) => {
    await reshapeConversations(page, { source: 'managed' });
    const uploads: Record<string, unknown>[] = [];
    await page.route('**/api/v1/sessions/*/attachments', async (route) => {
      uploads.push(route.request().postDataJSON());
      await route.fulfill({ json: { workspace_path: '.gnomcode/attachments/notes.txt' } });
    });
    await openConsole('#sessions');
    await openClaudeConversation(page);
    const input = page.locator('#local-workspace-root input[type="file"]');
    await input.setInputFiles({ name: 'notes.txt', mimeType: 'text/plain', buffer: Buffer.from('hello') });
    await expect(page.locator('#chat-attachments-bar')).toContainText('notes.txt');
    await expect(page.locator('#chat-attachments-bar')).toContainText(/Saved|Uploaded/);
    expect(uploads).toHaveLength(1);
    expect(uploads[0]).toMatchObject({ filename: 'notes.txt', mime_type: 'text/plain', size: 5 });
    expect(typeof uploads[0].data_base64).toBe('string');
    await input.setInputFiles({ name: 'tool.exe', mimeType: 'application/octet-stream', buffer: Buffer.from('MZ') });
    await expect(page.locator('#chat-attach-status')).not.toBeEmpty();
    expect(uploads).toHaveLength(1);
  });

  test('Interrupt appears only while a turn runs and sends one idempotent request', async ({ page, openConsole }) => {
    await reshapeConversations(page, { source: 'managed', hosted: true, state: 'thinking' });
    const requests: Record<string, unknown>[] = [];
    await page.route('**/api/v1/sessions/*/interrupt', async (route) => {
      requests.push(route.request().postDataJSON());
      await route.fulfill({ json: { type: 'interrupt_ack', sent: true } });
    });
    await openConsole('#sessions');
    await openClaudeConversation(page);
    const interrupt = page.getByRole('button', { name: 'Interrupt running turn' });
    await expect(interrupt).toBeVisible();
    await interrupt.click();
    await expect.poll(() => requests.length).toBe(1);
    expect(String(requests[0].request_id)).toMatch(/\S{8,}/);
  });

  test('Interrupt is absent when the turn is idle', async ({ page, openConsole }) => {
    await reshapeConversations(page, { source: 'managed', state: 'idle' });
    await openConsole('#sessions');
    await openClaudeConversation(page);
    await expect(page.getByRole('button', { name: 'Interrupt running turn' })).toHaveCount(0);
  });
});
