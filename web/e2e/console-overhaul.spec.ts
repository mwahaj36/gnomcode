import { expect, test } from './fixtures';

test('Home and Usage expose work, capacity and report refresh', async ({ page, openConsole }) => {
  await openConsole();
  await expect(page.locator('.sp-home-grid > #home-sessions')).toBeVisible();
  await expect(page.locator('.sp-home-grid > #home-limits')).toBeVisible();
  await page.getByRole('tab', { name: 'Usage', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Refresh', exact: true }).first()).toBeVisible();
});

test('Workspace history has its own column beside primary navigation', async ({ page, openConsole }) => {
  await openConsole('#sessions');
  const sidebar = page.locator('.gc-console > .sp-chat-column');
  await expect(sidebar.locator('#console-nav-sessions')).toBeAttached();
  const project = sidebar.locator('details.sp-project-conversations').first();
  await expect(project.locator('summary').first()).toBeVisible();
  await expect(project.locator('button.ide-session').first()).toBeVisible();
});
