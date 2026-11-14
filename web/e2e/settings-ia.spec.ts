import type { Page } from '@playwright/test';
import { expect, test } from './fixtures';

async function signedIn(page: Page) {
  let paired = true;
  await page.route('**/api/v1/auth/session', async (route) => {
    if (route.request().method() === 'DELETE') {
      paired = false;
      return route.fulfill({ json: { ok: true } });
    }
    return route.fulfill({ json: paired ? { paired: true, email: 'person@example.test' } : { paired: false } });
  });
}

for (const scheme of ['light', 'dark'] as const) {
  test(`Sign out is one click from the rail and confirmed (${scheme})`, async ({ page, openConsole }) => {
    await page.emulateMedia({ colorScheme: scheme });
    await signedIn(page);
    await openConsole('#usage');
    await page.locator('#account-sign-out').click();
    await expect(page.locator('#confirm-heading')).toHaveText('Sign out');
    await page.locator('#confirm-accept').click();
    await expect(page.locator('#app-notice')).toContainText('Signed out');
    await expect(page.locator('#account-sign-out')).toBeHidden();
    await expect(page.locator('#account-entry')).toBeVisible();
  });
}

test('Account is first in Settings and shows who is signed in with Sign out', async ({ page, openConsole }) => {
  await signedIn(page);
  await openConsole('#settings');
  const first = page.locator('#local-settings-root .sp-settings-root > section').first();
  await expect(first).toHaveId('settings-account');
  await expect(first).toContainText('person@example.test');
  await first.getByRole('button', { name: 'Sign out' }).click();
  await page.locator('#confirm-accept').click();
  await expect(first).toContainText('Not signed in');
});

test('Settings search filters sections and finds Sign out', async ({ page, openConsole }) => {
  await signedIn(page);
  await openConsole('#settings');
  const search = page.getByRole('searchbox', { name: 'Search settings' });
  await search.fill('log out');
  await expect(page.locator('#settings-account')).toBeVisible();
  await expect(page.locator('#settings-appearance')).toBeHidden();
  await expect(page.locator('#settings-sign-out')).toBeVisible();
  await search.fill('notifications');
  await expect(page.locator('#settings-notifications')).toBeVisible();
  await expect(page.locator('#settings-account')).toBeHidden();
  await search.fill('zzzz');
  await expect(page.getByText('No settings match.')).toBeVisible();
});

test('section list jumps to a section', async ({ page, openConsole }) => {
  await openConsole('#settings');
  await page.locator('.sp-settings-nav').getByRole('link', { name: 'Updates' }).click();
  await expect(page.locator('#settings-updates h2')).toBeFocused();
});

test('command palette reaches Sign out and Notifications', async ({ page, openConsole }) => {
  await signedIn(page);
  await openConsole('#overview');
  await page.keyboard.press('Control+K');
  const palette = page.locator('#command-palette');
  await expect(palette).toBeVisible();
  await page.keyboard.type('notif');
  await page.keyboard.press('Enter');
  await expect(page.locator('#console-title')).toHaveText('Settings');
  await expect(page.locator('#settings-notifications h2')).toBeFocused();
  await page.locator('#command-palette-entry').click();
  await page.keyboard.type('sign out');
  await expect(palette.getByRole('option').first()).toHaveText('Sign out');
  await page.keyboard.press('Enter');
  await expect(page.locator('#confirm-heading')).toHaveText('Sign out');
});

test('phone width: Sign out within two taps, no horizontal scroll', async ({ page, openConsole }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await signedIn(page);
  await openConsole('#usage');
  await page.getByRole('tab', { name: 'Settings' }).click();
  const signOut = page.locator('#settings-sign-out');
  await expect(signOut).toBeVisible();
  expect((await signOut.boundingBox())?.height ?? 0).toBeGreaterThanOrEqual(44);
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  expect(overflow).toBeLessThanOrEqual(0);
});
