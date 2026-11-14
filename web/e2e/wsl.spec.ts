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
