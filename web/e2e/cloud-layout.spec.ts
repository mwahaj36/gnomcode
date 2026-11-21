import { expect, test } from '@playwright/test';

test.skip(!process.env.GNOMCODE_WEB_URL, 'Set GNOMCODE_WEB_URL to an isolated production web build.');

for (const size of [{ width: 1440, height: 900 }, { width: 390, height: 844 }]) {
  test(`account console shell ${size.width}x${size.height}`, async ({ page }, info) => {
    await page.setViewportSize(size);
    await page.goto(`${process.env.GNOMCODE_WEB_URL}/console`);
    const shell = page.locator('.gc-console');
    await expect(shell).toBeVisible();
    for (const view of ['Home', 'Workspace', 'Usage', 'Settings']) {
      await page.getByRole('navigation', { name: 'Console views' }).getByRole('button', { name: view, exact: true }).click();
      await expect(page.getByRole('heading', { name: view, exact: true }).first()).toBeVisible();
      const box = await shell.boundingBox();
      expect(box!.width).toBe(size.width);
      expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
      await page.screenshot({ path: info.outputPath(`${view.toLowerCase()}.png`) });
    }
  });
}
