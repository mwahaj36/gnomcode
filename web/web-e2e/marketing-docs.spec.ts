import { expect, test } from '@playwright/test';
const routes = ['/', '/downloads', '/pricing', '/docs', '/docs/getting-started', '/docs/supported-agents', '/docs/model-switching', '/docs/remote-access', '/docs/prompt-from-phone', '/docs/quota-and-cost', '/docs/model-pricing', '/docs/automate-multiple-agents', '/docs/faq', '/docs/quota-limits-explained', '/docs/privacy', '/docs/claude-code-vs-codex', '/privacy', '/terms'];
for (const width of [390, 1440]) {
 for (const theme of ['dark','light']) {
  test(`marketing and docs remain readable at ${width}px ${theme}`, async ({ page }, info) => {
    test.setTimeout(180000);
    await page.setViewportSize({ width, height: 900 });
    await page.addInitScript(theme=>localStorage.setItem('gnomcode-theme',theme),theme);
    for (const route of routes) {
      const response = await page.goto(route);
      expect(response?.status(), route).toBe(200);
      await expect(page.locator('h1').first()).toBeVisible();
      expect(await page.locator('body').innerText(),route).not.toMatch(/\bfree\b|\bpayment\b|\bpaid\b/i);
      expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth), route).toBeLessThanOrEqual(1);
      await page.screenshot({ path: info.outputPath(route.replaceAll('/', '_') + '.png'), fullPage: true });
    }
  });
}

}
