import { expect, test } from './fixtures';
import { openClaudeConversation, reshapeConversations } from './workspace-helpers';
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
    const tools = page.getByRole('complementary', { name: 'Conversation tools' });
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
    await page.getByRole('complementary', { name: 'Conversation tools' }).getByRole('button', { name: 'Files', exact: true }).click();
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
    await page.getByRole('complementary', { name: 'Conversation tools' }).getByRole('button', { name: 'Changes', exact: true }).click();
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
    await page.getByRole('complementary', { name: 'Conversation tools' }).getByRole('button', { name: 'Changes', exact: true }).click();
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
    await page.getByRole('complementary', { name: 'Conversation tools' }).getByRole('button', { name: 'Artifacts', exact: true }).click();
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
  test('the composer has no suggestion strip or customization control', async ({ page, openConsole }) => {
    await reshapeConversations(page, { source: 'managed', can_prompt: true, hosted: true, state: 'working' });
    await openConsole('#sessions');
    await openClaudeConversation(page);
    await expect(page.getByRole('toolbar', { name: 'Quick prompts' })).toHaveCount(0);
    await expect(page.getByRole('button', { name: 'Manage quick prompts' })).toHaveCount(0);
    await expect(page.getByRole('textbox', { name: 'Message the agent', exact: true })).toBeVisible();
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
