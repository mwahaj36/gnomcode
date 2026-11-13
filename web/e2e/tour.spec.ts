import { expect, test } from './fixtures';
import { startDaemon } from './daemon';

test.describe('first-run tour with existing history', () => {
  test.beforeEach(async ({ page }) => {
    // Opt back in: the shared fixture marks the tour as seen.
    await page.addInitScript(() => {
      try {
        if (sessionStorage.getItem('e2e-tour-reset')) return;
        sessionStorage.setItem('e2e-tour-reset', '1');
        localStorage.removeItem('gnomcode-onboarding-v1');
      } catch {
        /* storage unavailable */
      }
    });
  });

  test('offers a tip instead of a modal and the tip starts and dismisses the tour', async ({ page, daemon }) => {
    await page.goto(daemon.consoleUrl);
    const tip = page.locator('#tour-tip');
    await expect(tip).toBeVisible();
    await expect(page.locator('#onboarding-dialog')).not.toBeVisible();
    await tip.getByRole('button', { name: /dismiss|not now|skip/i }).click();
    await expect(tip).toBeHidden();
    expect(await page.evaluate(() => localStorage.getItem('gnomcode-onboarding-v1'))).toBe('dismissed');
    await page.reload();
    await expect(page.locator('#tour-tip')).toBeHidden();
  });

  // Known bug in the legacy shell: the last step focuses a launcher element that no longer exists and
  // never opens the React launcher. The shell port fixes it and removes this marker.
  test.fixme('walks four steps, can go back and ends at the session launcher', async ({ page, daemon }) => {
    await page.goto(daemon.consoleUrl);
    await page.locator('#tour-tip').getByRole('button').first().click();
    const dialog = page.locator('#onboarding-dialog');
    await expect(dialog).toBeVisible();
    await expect(page.locator('#tour-tip')).toBeHidden();
    const progress = page.locator('#onboarding-progress');
    const back = page.locator('#onboarding-back');
    const next = page.locator('#onboarding-next');
    await expect(progress).toHaveText('Step 1 of 4');
    await expect(back).toBeDisabled();
    await expect(dialog.getByRole('heading', { name: 'Four places, one local console' })).toBeFocused();
    await next.click();
    await expect(progress).toHaveText('Step 2 of 4');
    await expect(back).toBeEnabled();
    await back.click();
    await expect(progress).toHaveText('Step 1 of 4');
    await next.click();
    await next.click();
    await next.click();
    await expect(progress).toHaveText('Step 4 of 4');
    await expect(next).toHaveText('Open session launcher');
    await next.click();
    await expect(dialog).not.toBeVisible();
    await expect(page.locator('#console-title')).toHaveText('Workspace');
    await expect(page.getByRole('heading', { name: 'New agent session' })).toBeVisible();
    await expect(page.locator('.ide-launcher').getByLabel('Agent')).toBeFocused();
    expect(await page.evaluate(() => localStorage.getItem('gnomcode-onboarding-v1'))).toBe('dismissed');
  });

  test('Skip guide and Escape both close it and return focus', async ({ page, daemon }) => {
    await page.goto(daemon.consoleUrl);
    const start = page.locator('#tour-tip').getByRole('button').first();
    await start.click();
    await page.getByRole('button', { name: 'Skip guide' }).click();
    await expect(page.locator('#onboarding-dialog')).not.toBeVisible();
    await expect(page.locator('#console-title')).toHaveText('Home');

    await page.evaluate(() => localStorage.removeItem('gnomcode-onboarding-v1'));
    await page.goto(`${daemon.consoleUrl}#settings`);
    await page.locator('#local-settings-root').getByRole('button', { name: 'Getting started' }).click();
    await expect(page.locator('#onboarding-dialog')).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(page.locator('#onboarding-dialog')).not.toBeVisible();
    await expect(page.locator('#local-settings-root').getByRole('button', { name: 'Getting started' })).toBeFocused();
  });
});

test('Getting started in Settings reopens the tour after it was dismissed', async ({ page, openConsole }) => {
  await openConsole('#settings');
  await expect(page.locator('#tour-tip')).toBeHidden();
  await page.locator('#local-settings-root').getByRole('button', { name: 'Getting started' }).click();
  await expect(page.locator('#onboarding-dialog')).toBeVisible();
  await expect(page.locator('#onboarding-progress')).toHaveText('Step 1 of 4');
});

test('an empty history opens the tour by itself', async ({ page }) => {
  const empty = await startDaemon({ seed: false });
  try {
    await page.goto(empty.consoleUrl);
    await expect(page.locator('#onboarding-dialog')).toBeVisible();
    await expect(page.locator('#tour-tip')).toBeHidden();
    await expect(page.locator('#onboarding-progress')).toHaveText('Step 1 of 4');
  } finally {
    await empty.stop();
  }
});
