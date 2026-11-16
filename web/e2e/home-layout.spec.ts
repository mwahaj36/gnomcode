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


test('activity month labels share the date grid columns at every scroll position', async ({ page, openConsole }) => {
  const days = Array.from({length:365}, (_,i)=>({date:new Date(Date.UTC(2025,10,17+i)).toISOString().slice(0,10),total_tokens:1000,cost_usd:0,cost_basis:'estimated',turns:1,level:1}));
  await page.route('**/api/v1/activity/heatmap',route=>route.fulfill({json:{from:'2025-11-17',to:'2026-11-16',first_recorded:'2025-11-17',days,stats:{total_tokens:365000,active_days:365,current_streak:365,longest_streak:365,busiest:null}}}));
  await openConsole('#overview');
  for (const width of [1440,390]) {
    await page.setViewportSize({width,height:900});
    const grid = page.locator('#home-activity .sp-heatmap-scroll');
    await expect(grid.locator('.sp-heatmap-months span').first()).toBeAttached();
    for (const end of [false,true]) {
      const offsets = await grid.evaluate((el,end)=>{
        el.scrollLeft = end ? el.scrollWidth : 0;
        const cells = el.querySelectorAll('.sp-heatmap-grid>.sp-heat');
        return [...el.querySelectorAll<HTMLElement>('.sp-heatmap-months span')].map(label=>{
          const col=Number(label.style.getPropertyValue('--col'));
          return Math.abs(label.getBoundingClientRect().left-cells[col*7].getBoundingClientRect().left);
        });
      },end);
      for(const offset of offsets)expect(offset).toBeLessThanOrEqual(1);
    }
  }
});
