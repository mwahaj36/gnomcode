import { expect, test as base } from '@playwright/test';
import { type Daemon, startDaemon } from './daemon';

// A Claude session that came from the desktop app next to the CLI one: the daemon names the surface.
const test = base.extend<object, { desktop: Daemon }>({
  desktop: [
    async ({}, use) => {
      const daemon = await startDaemon({ desktopClaude: true });
      await use(daemon);
      await daemon.stop();
    },
    { scope: 'worker' },
  ],
});

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem('gnomcode-onboarding-v1', 'dismissed'));
});

test('the surface label is secondary text on Home session rows', async ({ page, desktop }) => {
  await page.goto(desktop.consoleUrl);
  const rows = page.locator('#home-sessions .sp-session-row');
  await expect(rows).toHaveCount(3);
  await expect(rows.filter({ hasText: 'Claude desktop app' })).toHaveCount(1);
  await expect(rows.filter({ hasText: 'Claude Code · claude-opus-5' })).toHaveCount(1);
});

test('the surface label is secondary text in the Workspace list', async ({ page, desktop }) => {
  await page.goto(`${desktop.consoleUrl}#sessions`);
  const rows = page.locator('.ide-session');
  await expect(rows).toHaveCount(3);
  await expect(rows.filter({ hasText: 'Claude desktop app' })).toHaveCount(1);
  await expect(rows.filter({ hasText: 'claude-code' })).toHaveCount(1);
});

test('the API carries surface_label for desktop apps and not for CLI sessions', async ({ desktop }) => {
  const response = await fetch(`${desktop.base}/api/v1/conversations`, { headers: { Authorization: `Bearer ${desktop.token}` } });
  const rows = (await response.json()) as { agent: string; surface_label?: string }[];
  expect(rows.flatMap((row) => (row.surface_label ? [row.surface_label] : [])).sort()).toEqual(['Claude desktop app', 'Codex desktop app']);
});
