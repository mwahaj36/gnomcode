import { expect, test } from './fixtures';

test('Home section headings share a baseline and the conversation list can fill its sidebar', async ({ page, openConsole }) => {
  await openConsole();
  const sessions = page.locator('#home-sessions .sp-section-head');
  const limits = page.locator('#home-limits .sp-section-head');
  await expect(sessions).toBeVisible();
  await expect(limits).toBeVisible();
  const first = await sessions.boundingBox();
  const second = await limits.boundingBox();
  expect(Math.abs(first!.y - second!.y)).toBeLessThanOrEqual(1);
  expect(Math.abs(first!.height - second!.height)).toBeLessThanOrEqual(1);
  await page.getByRole('tab', { name: 'Workspace', exact: true }).click();
  const list = page.locator('.sp-chat-column .sp-workspace-list');
  await expect(list).toBeVisible();
  expect(await list.evaluate(node => getComputedStyle(node).maxHeight)).toBe('none');
});
