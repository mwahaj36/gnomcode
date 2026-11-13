import { expect, test } from './fixtures';

const PLACES = [
  { name: 'Home', hash: '', title: 'Home' },
  { name: 'Workspace', hash: '#sessions', title: 'Workspace' },
  { name: 'Usage', hash: '#usage', title: 'Usage' },
  { name: 'Settings', hash: '#settings', title: 'Settings' },
] as const;

test('rail has exactly four places plus the account entry and selects the active one', async ({ page, openConsole }) => {
  await openConsole();
  await expect(page.getByRole('tab')).toHaveCount(4);
  for (const place of PLACES) {
    await page.getByRole('tab', { name: place.name }).click();
    await expect(page.locator('#console-title')).toHaveText(place.title);
    await expect(page.getByRole('tab', { name: place.name })).toHaveAttribute('aria-selected', 'true');
    await expect(page.getByRole('tab', { selected: true })).toHaveCount(1);
    expect(new URL(page.url()).hash).toBe(place.hash);
  }
});

test('old addresses redirect in place without a history entry', async ({ page, openConsole }) => {
  for (const [old, title, hash] of [
    ['#limits', 'Usage', '#usage'],
    ['#projects', 'Usage', '#usage'],
    ['#automations', 'Settings', '#settings'],
  ] as const) {
    await openConsole(old);
    await expect(page.locator('#console-title')).toHaveText(title);
    await expect.poll(() => new URL(page.url()).hash).toBe(hash);
    await expect(page.locator('body')).toHaveAttribute('data-console-view', hash.slice(1));
  }
  // Following an old link inside the page also rewrites it.
  await openConsole();
  await page.evaluate(() => (window.location.hash = 'limits'));
  await expect(page.locator('#console-title')).toHaveText('Usage');
  await expect.poll(() => new URL(page.url()).hash).toBe('#usage');
});

test('browser back and forward walk the views', async ({ page, openConsole }) => {
  await openConsole();
  await page.getByRole('tab', { name: 'Usage' }).click();
  await page.getByRole('tab', { name: 'Settings' }).click();
  await page.goBack();
  await expect(page.locator('#console-title')).toHaveText('Usage');
  await page.goBack();
  await expect(page.locator('#console-title')).toHaveText('Home');
  await page.goForward();
  await expect(page.locator('#console-title')).toHaveText('Usage');
});

test('the rail is a roving-focus tablist driven by arrow, Home and End keys', async ({ page, openConsole }) => {
  await openConsole();
  const home = page.getByRole('tab', { name: 'Home' });
  await home.focus();
  await page.keyboard.press('ArrowDown');
  await expect(page.locator('#console-title')).toHaveText('Workspace');
  await expect(page.getByRole('tab', { name: 'Workspace' })).toBeFocused();
  await page.keyboard.press('End');
  await expect(page.locator('#console-title')).toHaveText('Settings');
  await page.keyboard.press('ArrowRight');
  await expect(page.locator('#console-title')).toHaveText('Home');
  await page.keyboard.press('ArrowUp');
  await expect(page.locator('#console-title')).toHaveText('Settings');
  await expect(page.getByRole('tab', { name: 'Home' })).toHaveAttribute('tabindex', '-1');
  await expect(page.getByRole('tab', { name: 'Settings' })).toHaveAttribute('tabindex', '0');
});

test('the brand link returns to Home and the rail only expands on request', async ({ page, openConsole }) => {
  await openConsole('#usage');
  await page.getByRole('link', { name: 'Gnomcode home' }).click();
  await expect(page.locator('#console-title')).toHaveText('Home');
  expect(new URL(page.url()).hash).toBe('');
  const toggle = page.locator('#navigation-toggle');
  await expect(toggle).toHaveAttribute('aria-expanded', 'false');
  await page.getByRole('tab', { name: 'Usage' }).hover();
  await expect(toggle).toHaveAttribute('aria-expanded', 'false');
  await toggle.click();
  await expect(toggle).toHaveAttribute('aria-expanded', 'true');
  await expect(page.locator('body')).toHaveAttribute('data-rail', 'expanded');
  await page.reload();
  await expect(toggle).toHaveAttribute('aria-expanded', 'true');
  await toggle.click();
  await expect(toggle).toHaveAttribute('aria-expanded', 'false');
});

test('the last view is restored for a bare address', async ({ page, daemon, openConsole }) => {
  await openConsole('#usage');
  await expect(page.locator('#console-title')).toHaveText('Usage');
  await page.goto(daemon.consoleUrl);
  await expect(page.locator('#console-title')).toHaveText('Usage');
});

test('connection status reports the daemon and stays out of sight', async ({ page, openConsole }) => {
  await openConsole();
  const status = page.locator('#sync-state');
  await expect(status).toHaveText('Local daemon connected');
  await expect(page.locator('.console-status')).toHaveClass(/visually-hidden/);
});

test('an unreachable daemon is reported as stale instead of crashing the console', async ({ page, openConsole }) => {
  await openConsole();
  await expect(page.locator('#sync-state')).toHaveText('Local daemon connected');
  await page.route('**/api/v1/**', (route) => route.abort());
  await expect(page.locator('#sync-state')).toHaveText('Local data stale', { timeout: 15_000 });
  await expect(page.locator('#sync-state')).toHaveClass(/local-status-error/);
});

test('the daemon refuses the console API without the bearer token and never echoes it in the page', async ({ page, daemon, openConsole }) => {
  const anonymous = await fetch(`${daemon.base}/api/v1/sessions?page=1&limit=1`);
  expect(anonymous.status).toBe(401);
  await openConsole();
  // The launch URL carries the token for this tab only; every API call must use the header instead.
  const queries: string[] = [];
  page.on('request', (request) => {
    if (request.url().includes('/api/')) queries.push(request.url());
  });
  await page.getByRole('tab', { name: 'Usage' }).click();
  await expect(page.locator('#console-title')).toHaveText('Usage');
  await expect.poll(() => queries.length).toBeGreaterThan(0);
  expect(queries.filter((url) => url.includes('token='))).toEqual([]);
});

test('a folder deep link opens the launcher on that folder and is consumed', async ({ page, daemon, openConsole }) => {
  await openConsole(`#sessions?folder=${encodeURIComponent("C:\work\space & quote'")}`);
  await expect(page.locator('#console-title')).toHaveText('Workspace');
  const launcher = page.locator('.ide-launcher');
  await expect(launcher.getByRole('heading', { name: 'New agent session' })).toBeVisible();
  await expect(launcher.getByLabel('Workspace directory')).toHaveValue("C:\work\space & quote'");
  const url = new URL(page.url());
  expect(url.hash).toBe('#sessions');
  expect(url.searchParams.get('token')).toBe(daemon.token);
});

test('every navigation keeps the launch address so a reload stays authorized', async ({ page, daemon, openConsole }) => {
  await openConsole();
  for (const name of ['Workspace', 'Settings', 'Home']) {
    await page.getByRole('tab', { name }).click();
    expect(new URL(page.url()).searchParams.get('token')).toBe(daemon.token);
  }
  await page.getByRole('tab', { name: 'Usage' }).click();
  await page.reload();
  await expect(page.locator('#console-title')).toHaveText('Usage');
  await expect(page.locator('#sync-state')).toHaveText('Local daemon connected');
});

test('a missing token is reported with an action instead of an endless spinner', async ({ page, daemon }) => {
  await page.goto(daemon.base + '/');
  await expect(page.locator('#sync-state')).toHaveText('Local data unavailable');
  await expect(page.locator('#sync-state')).toHaveClass(/local-status-error/);
});

test('the Sign in entry stays reachable by keyboard and a hash change navigates', async ({ page, openConsole }) => {
  await openConsole();
  const entry = page.getByRole('button', { name: 'Sign in to Gnomcode' });
  await expect(entry).not.toHaveAttribute('tabindex', '-1');
  await entry.focus();
  await page.keyboard.press('Enter');
  await expect(page.locator('#console-title')).toHaveText('Account');
  await page.evaluate(() => (window.location.hash = 'settings'));
  await expect(page.locator('#console-title')).toHaveText('Settings');
  await expect(page.locator('#view-settings')).toHaveClass(/active/);
  await expect(page.locator('#view-auth')).not.toHaveClass(/active/);
});

test('the console is one React root and the retired script is gone', async ({ page, daemon, openConsole }) => {
  await openConsole();
  await expect(page.locator('#gnomcode-console-root .sp-console-workspace')).toBeVisible();
  const scripts = await page.locator('script[src]').evaluateAll((nodes) => nodes.map((node) => node.getAttribute('src')));
  expect(scripts).not.toContain('/console-app.js');
  expect((await fetch(`${daemon.base}/console-app.js`, { headers: { Authorization: `Bearer ${daemon.token}` } })).status).toBe(404);
});
