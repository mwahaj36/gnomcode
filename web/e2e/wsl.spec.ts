import { expect, test } from './fixtures';
import { reshapeConversations } from './workspace-helpers';

const BS = String.fromCharCode(92);
const LONG = [BS + BS + 'wsl.localhost', 'Ubuntu-22.04', 'home', 'wahaj', 'src', 'very-long-organisation-name', 'monorepo', 'packages', 'web-console'].join(BS);

for (const scheme of ['light', 'dark'] as const) {
  for (const width of [1440, 390]) {
    test.describe(`WSL ${scheme} ${width}px`, () => {
      test.use({ viewport: { width, height: 900 } });

      test('Run in picker posts a wsl.localhost folder and shows the starting state', async ({ page, openConsole }) => {
        await page.emulateMedia({ colorScheme: scheme });
        await page.route('**/api/v1/wsl/distros', (route) =>
          route.fulfill({ json: { distros: [{ name: 'Ubuntu-22.04', label: 'WSL: Ubuntu-22.04', running: false }] } }),
        );
        await page.route('**/api/v1/agents/installed', (route) =>
          route.fulfill({ json: { agents: [{ id: 'codex', name: 'OpenAI Codex', installed: true }] } }),
        );
        const launches: Record<string, unknown>[] = [];
        await page.route('**/api/v1/sessions/launch', async (route) => {
          launches.push(route.request().postDataJSON());
          await route.fulfill({ status: 500, json: { message: 'No terminal in this test.' } });
        });
        await openConsole('#sessions');
        await page.getByRole('button', { name: width < 768 ? 'New session' : 'New', exact: true }).first().click();
        const form = page.locator('.ide-launcher');
        const runIn = form.getByLabel('Run in');
        await expect(runIn).toHaveValue('');
        await expect(runIn.locator('option').first()).toHaveText('Windows');
        await runIn.selectOption('Ubuntu-22.04');
        await form.getByLabel('Linux folder in Ubuntu-22.04').fill('/home/wahaj/src/very-long-organisation-name/monorepo/packages/web-console');
        const code = form.locator('.sp-path-line code');
        await expect(code).toHaveAttribute('title', LONG);
        await expect(code).toContainText('...');
        await expect(form.getByRole('button', { name: 'Copy full path' })).toBeVisible();
        await page.getByRole('button', { name: 'Launch Agent' }).click();
        await expect.poll(() => launches.length).toBe(1);
        expect(launches[0].repo_path).toBe(LONG);
        const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
        expect(overflow).toBeLessThanOrEqual(0);
      });

      test('WSL session rows carry a distro badge', async ({ page, openConsole }) => {
        await page.emulateMedia({ colorScheme: scheme });
        await reshapeConversations(page, { repo_path: LONG });
        await openConsole('#sessions');
        await expect(page.locator('.ide-session .sp-wsl-badge').first()).toHaveText('WSL: Ubuntu-22.04');
        const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
        expect(overflow).toBeLessThanOrEqual(0);
      });
    });
  }
}

test('All chats filters by where a chat runs', async ({ page, openConsole }) => {
  await reshapeConversations(page, { repo_path: LONG });
  await openConsole('#sessions');
  await page.locator('.sp-all-chats-trigger').first().click();
  const pane = page.locator('.sp-all-chats-pane');
  const chips = pane.getByRole('group', { name: 'Filter by where it runs' });
  await expect(chips.getByRole('button', { name: 'WSL: Ubuntu-22.04' })).toBeVisible();
  const before = await pane.locator('.sp-all-chat-row').count();
  await chips.getByRole('button', { name: 'WSL: Ubuntu-22.04' }).click();
  await expect(pane.locator('.sp-all-chat-row .sp-wsl-badge').first()).toHaveText('WSL: Ubuntu-22.04');
  const wslRows = await pane.locator('.sp-all-chat-row').count();
  await chips.getByRole('button', { name: 'Windows' }).click();
  await expect(pane.locator('.sp-all-chat-row .sp-wsl-badge')).toHaveCount(0);
  expect(wslRows).toBeLessThanOrEqual(before);
});

test('provider card source line names WSL sources', async ({ page, openConsole }) => {
  await page.route('**/api/v1/providers/overview', async (route) => {
    const response = await route.fetch();
    const body = await response.json();
    for (const card of body) if (card.provider === 'claude-code') card.wsl_sources = ['WSL: Ubuntu-22.04'];
    await route.fulfill({ response, json: body });
  });
  await openConsole('#usage');
  const card = page.locator('.sp-provider-card[data-provider="claude-code"]').first();
  await expect(card).toBeAttached();
  if (await card.locator('.sp-provider-foot').count()) await expect(card.locator('.sp-provider-foot .sp-wsl-badge')).toHaveText('WSL: Ubuntu-22.04');
});

test('Integrate in WSL appears when the Claude hook is missing and posts the distro', async ({ page, openConsole }) => {
  await page.route('**/api/v1/wsl/distros', (route) =>
    route.fulfill({ json: { distros: [{ name: 'Ubuntu-22.04', label: 'WSL: Ubuntu-22.04', running: true, claude_hook: 'missing' }] } }),
  );
  const posts: Record<string, unknown>[] = [];
  await page.route('**/api/v1/wsl/integrate', async (route) => {
    posts.push(route.request().postDataJSON());
    await route.fulfill({ json: { ok: true, message: 'Claude hook installed in Ubuntu-22.04 (1 profile(s)).' } });
  });
  await openConsole('#sessions');
  await page.getByRole('button', { name: 'New', exact: true }).first().click();
  const form = page.locator('.ide-launcher');
  await form.getByLabel('Agent').selectOption('claude');
  await form.getByLabel('Run in').selectOption('Ubuntu-22.04');
  await form.getByRole('button', { name: 'Integrate in WSL' }).click();
  await expect(form.getByRole('status').filter({ hasText: 'hook installed' })).toBeVisible();
  expect(posts[0]).toEqual({ distro: 'Ubuntu-22.04' });
  await expect(form.getByRole('button', { name: 'Integrate in WSL' })).toHaveCount(0);
});
