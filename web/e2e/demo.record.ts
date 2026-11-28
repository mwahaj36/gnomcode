import { mkdir } from 'node:fs/promises';
import path from 'node:path';
import { test } from './fixtures';

// Captures the README demo frames from the real console seeded with anonymized fixtures.
// Run: npx playwright test -c playwright.demo.config.ts, then scripts/make-demo-gif.ps1.
const out = path.resolve(__dirname, '..', '..', 'target', 'demo-frames');

test('record README demo frames', async ({ page, openConsole }) => {
  await mkdir(out, { recursive: true });
  await page.setViewportSize({ width: 1280, height: 800 });
  let n = 0;
  const shot = async () => {
    await page.waitForTimeout(700);
    await page.screenshot({ path: path.join(out, `frame-${String(n++).padStart(2, '0')}.png`) });
  };
  for (const hash of ['#overview', '#usage', '#sessions']) {
    await openConsole(hash);
    await shot();
  }
  await page.locator('.ide-session', { hasText: 'claude-code' }).click();
  await shot();
  await openConsole('#overview');
  await shot();
});
