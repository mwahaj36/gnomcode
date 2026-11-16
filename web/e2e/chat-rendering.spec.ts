import type { Page } from '@playwright/test';
import { expect, test } from './fixtures';
import { reshapeConversations } from './workspace-helpers';

const at = '2026-11-14T10:00:00Z';
const PNG = 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==';
const events = [
  { event_id: 'e1', turn_index: 1, role: 'user', content: 'Show the change', created_at: at },
  { event_id: 'e2', turn_index: 2, role: 'system', status: 'task_notification', content: 'Background task finished: Sample task\n\nAll checks passed', created_at: at },
  { event_id: 'e3', turn_index: 3, role: 'system', status: 'command', content: 'Ran /compact', created_at: at },
  {
    event_id: 'e4', turn_index: 4, role: 'assistant', created_at: at,
    content: `## Result\n\n\`\`\`rust\nfn main() { let long_line = "${'x'.repeat(300)}"; }\n\`\`\`\n\n\`\`\`diff\n-old\n+new\n\`\`\`\n\n![shot](${PNG}) ![remote](https://example.com/a.png)`,
  },
];

async function openChat(page: Page, openConsole: (hash: string) => Promise<void>) {
  await reshapeConversations(page, { source: 'managed', hosted: true });
  await page.route('**/api/v1/sessions/*/chat?*', (route) => route.fulfill({ json: events }));
  await page.context().grantPermissions(['clipboard-read', 'clipboard-write']);
  await openConsole('#sessions');
  const toggle = page.locator('#chat-list-toggle');
  if (await toggle.isVisible() && (await toggle.getAttribute('aria-expanded')) === 'false') await toggle.click();
  await page.locator('.ide-session', { hasText: 'claude-code' }).first().click();
  await expect(page.locator('.sp-code').first()).toBeVisible();
}

for (const [name, viewport] of [['desktop', { width: 1280, height: 900 }], ['phone', { width: 390, height: 844 }]] as const) {
  test.describe(`chat rendering on ${name}`, () => {
    test.use({ viewport });

    test('code blocks copy, scroll sideways and wrap on demand without page overflow', async ({ page, openConsole }) => {
      await openChat(page, openConsole);
      const block = page.locator('.sp-code').first();
      await expect(block.locator('.sp-code-lang')).toHaveText('rust');
      const pre = block.locator('pre');
      expect(await pre.evaluate((el) => el.scrollWidth > el.clientWidth)).toBe(true);
      await block.getByRole('button', { name: 'Copy' }).click();
      await expect(block.getByRole('button', { name: 'Copied' })).toBeVisible();
      expect(await page.evaluate(() => navigator.clipboard.readText())).toContain('fn main()');
      await block.getByRole('button', { name: 'Wrap' }).click();
      expect(await pre.evaluate((el) => el.scrollWidth <= el.clientWidth + 1)).toBe(true);
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
      await expect(page.locator('.sp-diff-add')).toHaveText('+new');
    });

    test('harness entries are chips and no raw tags show', async ({ page, openConsole }) => {
      await openChat(page, openConsole);
      await expect(page.locator('.sp-chat-system[data-kind="command"]')).toHaveText('Ran /compact');
      const task = page.locator('details.sp-chat-system[data-kind="task_notification"]');
      await expect(task).not.toHaveAttribute('open', '');
      await task.locator('summary').click();
      await expect(task).toContainText('All checks passed');
      const text = await page.locator('.sp-chat-stream').innerText();
      expect(text).not.toContain('<task-notification>');
      expect(text).not.toContain('<system-reminder>');
      await expect(page.locator('.sp-chat-user')).toHaveCount(1);
    });

    test('message footer tools are compact and user tools reveal on hover or tap', async ({ page, openConsole }) => {
      await openChat(page, openConsole);
      const user = page.locator('.sp-chat-user');
      const assistant = page.locator('.sp-chat-assistant').first();
      await expect(assistant.locator('.sp-chat-copy')).toHaveCSS('opacity', '1');
      await expect(user.locator('.sp-chat-copy')).toHaveCSS('opacity', '0');
      if (name === 'desktop') await user.hover();
      else await user.locator('.sp-chat-body').click();
      await expect(user.locator('.sp-chat-copy')).toHaveCSS('opacity', '1');
      const body = await assistant.locator('.sp-chat-body').boundingBox();
      const footer = await assistant.locator('header').boundingBox();
      expect(footer!.y).toBeGreaterThanOrEqual(body!.y + body!.height);
      await expect(assistant.locator('header .sp-copy-label')).toBeHidden();
    });

    test('image thumbnails open full size and external images stay unloaded', async ({ page, openConsole }) => {
      await openChat(page, openConsole);
      await expect(page.locator('img[src^="https://"]')).toHaveCount(0);
      await expect(page.getByText('External image not loaded: remote (opens example.com)')).toBeVisible();
      await page.getByRole('button', { name: 'Open shot full size' }).click();
      await expect(page.locator('dialog.sp-image-viewer img')).toBeVisible();
      await page.locator('dialog.sp-image-viewer').getByRole('button', { name: 'Close' }).click();
      await expect(page.locator('dialog.sp-image-viewer')).toHaveCount(0);
    });
  });
}
