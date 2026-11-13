import { expect, test } from './fixtures';

test.use({ viewport: { width: 390, height: 844 }, hasTouch: true });

for (const [name, hash] of [
  ['Home', ''],
  ['Workspace', '#sessions'],
  ['Usage', '#usage'],
  ['Settings', '#settings'],
  ['Account', '#auth'],
] as const) {
  test(`${name} fits a 390px phone without sideways scrolling`, async ({ page, openConsole }) => {
    await openConsole(hash);
    await expect(page.locator('#console-title')).toHaveText(name);
    await page.waitForTimeout(800);
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
    expect(overflow).toBeLessThanOrEqual(0);
  });
}

test('rail buttons are at least 44px and navigation works by touch', async ({ page, openConsole }) => {
  await openConsole();
  for (const tab of await page.getByRole('tab').all()) {
    const box = await tab.boundingBox();
    expect(box?.height ?? 0).toBeGreaterThanOrEqual(44);
    expect(box?.width ?? 0).toBeGreaterThanOrEqual(44);
  }
  await page.getByRole('tab', { name: 'Usage' }).tap();
  await expect(page.locator('#console-title')).toHaveText('Usage');
});

test('the conversation list starts collapsed on a phone', async ({ page, openConsole }) => {
  await openConsole('#sessions');
  await expect(page.locator('body')).toHaveClass(/chat-list-collapsed/);
  await expect(page.locator('#chat-list-toggle')).toBeVisible();
  await expect(page.locator('#chat-list-toggle')).toHaveAttribute('aria-expanded', 'false');
});

test('session rows on Home are 44px tall touch targets', async ({ page, openConsole }) => {
  await openConsole();
  for (const row of await page.locator('#home-sessions .sp-session-row').all()) {
    const box = await row.boundingBox();
    expect(box?.height ?? 0).toBeGreaterThanOrEqual(44);
  }
});
