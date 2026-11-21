import { expect, test } from './fixtures';
import { openClaudeConversation, reshapeConversations } from './workspace-helpers';

for (const size of [{ width: 640, height: 1100 }, { width: 430, height: 760 }, { width: 853, height: 480 }, { width: 1440, height: 900 }]) {
  test(`rebuilt workspace navigation ${size.width}x${size.height}`, async ({ page, openConsole }, info) => {
    await page.setViewportSize(size);
    await openConsole('#sessions');
    const toggle = page.locator('#chat-list-toggle');
    if (await toggle.getAttribute('aria-expanded') === 'false') await toggle.click();
    const history = page.getByRole('complementary', { name: 'Session history' });
    await expect(history).toBeVisible();
    const newButton = history.getByRole('button', { name: 'New', exact: true });
    const button = await newButton.boundingBox();
    expect(button!.width).toBeGreaterThan(54);
    expect(button!.height).toBeLessThanOrEqual(44);
    const list = history.locator('.sp-workspace-list');
    const historyBox = await history.boundingBox();
    const listBox = await list.boundingBox();
    expect(listBox!.height / historyBox!.height).toBeGreaterThan(0.4);
    await page.screenshot({ path: info.outputPath('history.png') });
    await openClaudeConversation(page);
    if (size.width < 900) await expect(history).toBeHidden();
    const heading = page.locator('.conversation-heading');
    const headingBox = await heading.boundingBox();
    const toolbarBox = await page.locator('.console-context').boundingBox();
    expect(headingBox!.y).toBeGreaterThanOrEqual(toolbarBox!.y + toolbarBox!.height - 1);
    await expect(page.locator('.sp-chat-stream-box')).toBeVisible();
    await page.screenshot({ path: info.outputPath('conversation.png') });
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
  });
}

test('resizing a conversation into a compact window keeps the conversation visible', async ({ page, openConsole }) => {
  await openConsole('#sessions');
  await openClaudeConversation(page);
  await page.setViewportSize({ width: 640, height: 850 });
  await expect(page.getByRole('complementary', { name: 'Session history' })).toBeHidden();
  await page.locator('#chat-list-toggle').click();
  await expect(page.getByRole('complementary', { name: 'Session history' })).toBeVisible();
  await page.getByRole('button', { name: 'New', exact: true }).click();
  await expect(page.getByRole('complementary', { name: 'Session history' })).toBeHidden();
  await expect(page.getByRole('heading', { name: 'New agent session', exact: true })).toBeVisible();
});

test('a large review stays navigable and loads the selected file', async ({ page, openConsole }) => {
  await reshapeConversations(page, { repo_path: '/work/demo' });
  await page.route('**/api/v1/sessions/*/changes*', route => {
    const selected = new URL(route.request().url()).searchParams.get('path');
    return route.fulfill({ json: { status: ' M small.txt\n M large.txt\n', truncated: !selected, diff: selected ? 'diff --git a/small.txt b/small.txt\n-old\n+review this file\n' : '' } });
  });
  await openConsole('#sessions');
  await openClaudeConversation(page);
  await page.getByRole('button', { name: 'Overview', exact: true }).click();
  await page.getByRole('complementary', { name: 'Conversation tools' }).getByRole('button', { name: 'Changes', exact: true }).click();
  const panel = page.locator('.sp-changes-panel');
  await expect(panel).toContainText('Select a file to review');
  await panel.locator('[data-change-file="small.txt"]').click();
  await expect(panel.locator('.sp-diff-code')).toContainText('+review this file');
  await expect(panel.getByRole('button', { name: 'Copy diff', exact: true })).toBeEnabled();
});

test('history and docked tools retain keyboard resizing without overflowing compact layouts', async ({ page, openConsole }) => {
  await reshapeConversations(page, { repo_path: '/work/demo' });
  await page.route('**/api/v1/repos/files?*', route => route.fulfill({ json: { path: '', entries: [] } }));
  await openConsole('#sessions');
  await openClaudeConversation(page);
  const history = page.locator('.gc-console > .sp-chat-column');
  const before = (await history.boundingBox())!.width;
  const historyHandle = page.getByRole('separator', { name: 'Resize conversations', exact: true });
  await historyHandle.focus();
  await historyHandle.press('ArrowRight');
  await expect.poll(async () => (await history.boundingBox())!.width).toBe(before + 8);
  await page.getByRole('button', { name: 'Overview', exact: true }).click();
  await page.getByRole('complementary', { name: 'Conversation tools' }).getByRole('button', { name: 'Files', exact: true }).click();
  const tools = page.getByRole('complementary', { name: 'Workspace tools' });
  const width = (await tools.boundingBox())!.width;
  const handle = page.getByRole('separator', { name: 'Resize tools panel', exact: true });
  await handle.focus();
  await handle.press('ArrowRight');
  await expect.poll(async () => (await tools.boundingBox())!.width).toBe(width - 8);
  await page.setViewportSize({ width: 1000, height: 720 });
  await expect(handle).toBeHidden();
  const pane = await tools.boundingBox();
  expect(pane!.x + pane!.width).toBeLessThanOrEqual(1000);
});
