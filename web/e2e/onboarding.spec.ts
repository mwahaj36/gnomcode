import type { Page, Route } from '@playwright/test';
import { expect, test } from './fixtures';
import { startDaemon } from './daemon';

type State = 'done' | 'todo' | 'waiting' | 'unknown';
type Steps = Record<string, State>;

const LABELS: Record<string, string> = {
  installed: 'CLI installed',
  signed_in: 'Signed in',
  integrated: 'Gnomcode hook',
  protocol_approvals: 'Phone and console approvals',
  limits: 'Exact limits',
  first_data: 'First session seen',
};
const COMMANDS = (id: string): Record<string, string> => ({
  installed: id === 'codex' ? 'npm install -g @openai/codex' : 'curl -fsSL https://claude.ai/install.sh | bash',
  signed_in: id === 'codex' ? 'codex login' : 'claude  (then type /login)',
  integrated: 'gnomcode integrate claude',
  protocol_approvals: `gnomcode protocol-approvals enable ${id}`,
  limits: 'gnomcode limits enable claude',
  first_data: id,
});
const OPTIONAL = new Set(['protocol_approvals', 'limits']);
const NAMES: Record<string, string> = { claude: 'Claude Code', codex: 'Codex', antigravity: 'Antigravity' };

function provider(id: string, steps: Steps, distro?: string) {
  const list = Object.entries(steps).map(([step, state]) => ({
    id: step,
    label: LABELS[step],
    state,
    optional: OPTIONAL.has(step),
    trust: 'exact',
    note: state === 'done' ? 'Verified' : 'Needs action',
    command: state === 'done' ? undefined : COMMANDS(id)[step],
  }));
  return {
    provider: id,
    name: NAMES[id],
    distro,
    ready: list.every((s) => s.optional || s.state === 'done'),
    steps: list,
  };
}

/** Mocks the Rust status endpoint; tests flip `world` and the next 2 s poll sees it. */
async function mockStatus(page: Page, world: Record<string, Steps>, seen: string[] = []) {
  await page.route('**/api/v1/onboarding/status**', async (route: Route) => {
    const url = new URL(route.request().url());
    seen.push(url.search);
    const only = url.searchParams.get('provider');
    const distro = url.searchParams.get('distro') ?? undefined;
    const ids = only ? [only] : Object.keys(world);
    await route.fulfill({ json: { providers: ids.map((id) => provider(id, world[id], distro)) } });
  });
}

const fresh = () => ({
  claude: { installed: 'done', signed_in: 'todo', integrated: 'todo', limits: 'todo', first_data: 'waiting' } as Steps,
  codex: { installed: 'todo', signed_in: 'todo', protocol_approvals: 'todo', first_data: 'waiting' } as Steps,
  antigravity: { installed: 'todo', signed_in: 'todo', protocol_approvals: 'todo', first_data: 'waiting' } as Steps,
});

async function openSetup(page: Page, url: string) {
  await page.goto(`${url}#settings`);
  const mobileCat = page.locator('#settings-mobile-category');
  if (await mobileCat.isVisible()) {
    await mobileCat.selectOption('settings-about');
  } else {
    await page.locator('.sp-settings-nav').getByRole('link', { name: 'About' }).click();
  }
  await page.locator('#local-settings-root').getByRole('button', { name: 'Getting started' }).click();
  await expect(page.locator('#onboarding-dialog')).toBeVisible();
}

test('detected providers are pre-checked and each step ticks as status flips', async ({ page, openConsole, daemon }) => {
  const world = fresh();
  await mockStatus(page, world);
  await openConsole('#settings');
  await openSetup(page, daemon.consoleUrl);
  const dialog = page.locator('#onboarding-dialog');
  await expect(dialog.getByRole('heading', { name: 'Which agents do you use?' })).toBeFocused();
  await expect(dialog.getByLabel('Claude Code')).toBeChecked();
  await expect(dialog.getByLabel('Codex')).not.toBeChecked();
  await page.locator('#onboarding-next').click();
  await expect(page.locator('#onboarding-progress')).toHaveText('Claude Code: provider 1 of 1');

  const step = (id: string) => dialog.locator(`[data-step="${id}"]`);
  await expect(step('signed_in')).toHaveAttribute('aria-current', 'step');
  await expect(step('signed_in')).toContainText('Waiting for sign-in...');
  await expect(step('signed_in').getByRole('button', { name: /Copy/ })).toBeVisible();
  await expect(page.locator('#onboarding-next')).toBeDisabled();

  world.claude.signed_in = 'done';
  await expect(step('signed_in')).toHaveAttribute('data-state', 'done');
  await expect(step('integrated')).toHaveAttribute('aria-current', 'step');
  await expect(step('integrated')).toContainText('gnomcode integrate claude');
  world.claude.integrated = 'done';
  await expect(step('integrated')).toHaveAttribute('data-state', 'done');
  world.claude.limits = 'waiting';
  world.claude.first_data = 'done';
  await expect(step('first_data')).toHaveAttribute('data-state', 'done');
  await expect(page.locator('#onboarding-next')).toBeEnabled();
  await page.locator('#onboarding-next').click();
  await expect(dialog.getByRole('heading', { name: 'Take it with you' })).toBeVisible();
  await page.locator('#onboarding-next').click();
  await expect(dialog.getByRole('heading', { name: 'You are set up' })).toBeVisible();
  await expect(dialog.locator('.sp-onb-summary')).toContainText('Ready');
  await page.locator('#onboarding-next').click();
  await expect(dialog).not.toBeVisible();
});

test('copy buttons put the command on the clipboard', async ({ page, context, openConsole, daemon }) => {
  await context.grantPermissions(['clipboard-read', 'clipboard-write']);
  await mockStatus(page, fresh());
  await openConsole('#settings');
  await openSetup(page, daemon.consoleUrl);
  await page.locator('#onboarding-next').click();
  const copy = page.locator('#onboarding-dialog [data-step="signed_in"]').getByRole('button', { name: /Copy/ });
  await copy.click();
  await expect(copy).toHaveText('Copied');
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe('claude  (then type /login)');
});

test('Skip moves on, Close keeps progress and reopening resumes', async ({ page, openConsole, daemon }) => {
  const world = fresh();
  await mockStatus(page, world);
  await openConsole('#settings');
  await openSetup(page, daemon.consoleUrl);
  const dialog = page.locator('#onboarding-dialog');
  await dialog.getByLabel('Codex').check();
  await page.locator('#onboarding-next').click();
  await expect(page.locator('#onboarding-progress')).toHaveText('Claude Code: provider 1 of 2');
  await dialog.getByRole('button', { name: 'Skip' }).click();
  await expect(page.locator('#onboarding-progress')).toHaveText('Codex: provider 2 of 2');
  await expect(dialog.locator('[data-step="protocol_approvals"]')).toContainText('Optional');
  await page.keyboard.press('Escape');
  await expect(dialog).not.toBeVisible();

  await page.reload();
  await expect(page.locator('#tour-tip')).toBeVisible();
  await page.locator('#tour-tip').getByRole('button', { name: 'Set up' }).click();
  await expect(page.locator('#onboarding-progress')).toHaveText('Codex: provider 2 of 2');
});

test('a WSL distro is passed to the status endpoint', async ({ page, openConsole, daemon }) => {
  const seen: string[] = [];
  await mockStatus(page, fresh(), seen);
  await openConsole('#settings');
  await openSetup(page, daemon.consoleUrl);
  await page.locator('#onboarding-next').click();
  await page.locator('#onboarding-dialog').getByLabel('Run in').fill('Ubuntu');
  await expect.poll(() => seen.some((q) => q.includes('distro=Ubuntu') && q.includes('provider=claude'))).toBe(true);
  await expect(page.locator('#onboarding-dialog .sp-setup-meta')).toContainText('WSL: Ubuntu');
});

test('Settings > Providers uses the same live checklist and the doctor link scrolls to it', async ({ page, openConsole }) => {
  const world = fresh();
  await mockStatus(page, world);
  await openConsole('#settings/providers');
  const panel = page.locator('#provider-setup');
  await expect(panel).toBeFocused();
  await expect(panel.locator('.sp-setup')).toHaveCount(3);
  const claude = panel.locator('[data-provider="claude"]');
  await expect(claude.locator('[data-step="signed_in"]')).toContainText('claude');
  world.claude.signed_in = 'done';
  await expect(claude.locator('[data-step="signed_in"]')).toHaveAttribute('data-state', 'done');
});

test('polling keeps rows in place (no layout shift)', async ({ page, openConsole, daemon }) => {
  await mockStatus(page, fresh());
  await openConsole('#settings');
  await openSetup(page, daemon.consoleUrl);
  await page.locator('#onboarding-next').click();
  const row = page.locator('#onboarding-dialog [data-step="first_data"]');
  await expect(row).toBeVisible();
  const before = await row.boundingBox();
  await page.waitForTimeout(4500);
  const after = await row.boundingBox();
  expect(after?.y).toBe(before?.y);
});

test('phone width: no horizontal scroll and 44px controls', async ({ page, openConsole, daemon }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await mockStatus(page, fresh());
  await openConsole('#settings');
  await openSetup(page, daemon.consoleUrl);
  await page.locator('#onboarding-next').click();
  const dialog = page.locator('#onboarding-dialog');
  const overflow = await dialog.evaluate((d) => d.scrollWidth - d.clientWidth);
  expect(overflow).toBeLessThanOrEqual(0);
  const copy = dialog.locator('[data-step="signed_in"]').getByRole('button', { name: /Copy/ });
  expect((await copy.boundingBox())?.height ?? 0).toBeGreaterThanOrEqual(44);
  const text = await dialog.innerText();
  expect(text).not.toMatch(/[\u{1F300}-\u{1FAFF}\u{2600}-\u{27BF}]/u);
});

test('an empty history opens onboarding by itself', async ({ page }) => {
  const empty = await startDaemon({ seed: false });
  try {
    await page.goto(empty.consoleUrl);
    await expect(page.locator('#onboarding-dialog')).toBeVisible();
    await expect(page.locator('#onboarding-progress')).toHaveText('Choose providers to set up');
  } finally {
    await empty.stop();
  }
});

test('blocked browser storage never breaks onboarding', async ({ page, daemon }) => {
  await page.addInitScript(() => {
    const blocked = () => {
      throw new Error('storage blocked');
    };
    Storage.prototype.getItem = blocked;
    Storage.prototype.setItem = blocked;
    Storage.prototype.removeItem = blocked;
  });
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await mockStatus(page, fresh());
  await openSetup(page, daemon.consoleUrl);
  await page.getByRole('button', { name: 'Close' }).first().click();
  await expect(page.locator('#onboarding-dialog')).not.toBeVisible();
  expect(errors).toEqual([]);
});
