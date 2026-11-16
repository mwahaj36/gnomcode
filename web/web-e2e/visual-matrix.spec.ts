import { expect, test } from '@playwright/test';

for (const size of [{ width: 1440, height: 900 }, { width: 1024, height: 768 }, { width: 768, height: 1024 }, { width: 390, height: 844 }, { width: 375, height: 667 }, { width: 844, height: 390 }]) {
  for (const theme of ['dark', 'light']) {
    test(`web screens ${size.width}x${size.height} ${theme}`, async ({ page }, info) => {
      await page.setViewportSize(size);
      await page.addInitScript(theme => {
        localStorage.setItem('gnomcode-theme', theme);
        localStorage.setItem('sb-jjnuxngvhuvxkxrfmnqx-auth-token', JSON.stringify({ access_token: 'test-token', refresh_token: 'test-refresh', expires_at: Math.floor(Date.now() / 1000) + 3600, token_type: 'bearer', user: { id: '00000000-0000-0000-0000-000000000001', aud: 'authenticated', email: 'test@example.invalid' } }));
      }, theme);
      await page.route('https://*.supabase.co/**', route => {
        const url = new URL(route.request().url());
        return route.fulfill({ json: url.pathname.endsWith('/spawnt_desktops') ? [{ id: '11111111111111111111111111111111', name: 'Offline desktop', last_seen_at: '2026-01-01T00:00:00Z', remote_enabled: false, live_snapshot: { generated_at: new Date().toISOString(), sessions: [], quota: [], timelines: {}, ongoing_count: 0 } }] : [] });
      });
      await page.goto('/console');
      for (const name of ['Home', 'Usage', 'Settings', 'Workspace']) {
        await page.getByRole('navigation', { name: 'Console views', exact: true }).getByRole('button', { name, exact: true }).click();
        await expect(page.getByRole('heading', { name, exact: true }).first()).toBeVisible();
        await page.waitForTimeout(350);
        await page.screenshot({ path: info.outputPath(`${name.toLowerCase()}.png`) });
        expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth), name).toBeLessThanOrEqual(1);
        const rail = await page.getByRole('navigation', { name: 'Console views', exact: true }).boundingBox();
        expect(rail!.y + rail!.height, name).toBeLessThanOrEqual(size.height + 1);
      }
    });
  }
}
