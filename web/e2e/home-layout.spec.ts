import path from 'node:path';
import { expect, test } from './fixtures';

for (const theme of ['light', 'dark']) {
  for (const width of [1440, 1024, 390]) {
    test(`Home geometry ${theme} ${width}`, async ({ page, openConsole }) => {
      await page.setViewportSize({ width, height: 900 });
      // Exercise current source styles even when using an already packaged baseline engine.
      await page.route('**/gnomcode-console.css', route => route.fulfill({
        contentType: 'text/css',
        path: path.resolve(__dirname, '../app/gnomcode-console.css'),
      }));
      await openConsole();
      await page.evaluate(theme => document.documentElement.dataset.theme = theme, theme);
      const swatches = page.locator('#home-activity .sp-heatmap-legend .sp-heat');
      await expect(swatches).toHaveCount(5);
      for (const swatch of await swatches.all()) {
        const rect = await swatch.boundingBox();
        expect(rect!.width).toBeLessThanOrEqual(12);
        expect(rect!.height).toBeLessThanOrEqual(12);
      }
      if (width >= 1024) {
        const activity = await page.locator('#home-activity .sp-section-head').boundingBox();
        const today = await page.locator('#home-today .sp-section-head').boundingBox();
        expect(Math.abs(activity!.y - today!.y)).toBeLessThanOrEqual(1);
      }
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
      await page.screenshot({ path: test.info().outputPath('home.png'), fullPage: true });
    });
  }
}
