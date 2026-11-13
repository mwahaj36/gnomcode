import { test as base, expect } from '@playwright/test';
import { type Daemon, startDaemon } from './daemon';

interface Fixtures {
  /** Opens the console the way the tray does and waits for the shell to be interactive. */
  openConsole: (hash?: string) => Promise<void>;
}

/** One real daemon per worker; every test opens the console in a fresh browser context. */
export const test = base.extend<Fixtures, { daemon: Daemon }>({
  daemon: [
    async ({}, use) => {
      const daemon = await startDaemon();
      await use(daemon);
      await daemon.stop();
    },
    { scope: 'worker' },
  ],
  openConsole: async ({ page, daemon }, use) => {
    // The first-run tour opens by itself on an empty history; tests that cover it opt back in.
    await page.addInitScript(() => {
      try {
        if (!localStorage.getItem('gnomcode-onboarding-v1')) localStorage.setItem('gnomcode-onboarding-v1', 'dismissed');
      } catch {
        /* storage unavailable */
      }
    });
    await use(async (hash = '') => {
      await page.goto(`${daemon.consoleUrl}${hash}`);
      await expect(page.locator('#console-title')).toBeAttached();
    });
  },
});

export { expect };
