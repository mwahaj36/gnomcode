import { expect, test } from './fixtures';

test.describe('Home', () => {
  test('shows limits, sessions, today tiles and the activity grid in that order', async ({ page, openConsole }) => {
    await openConsole();
    await expect(page.locator('#console-title')).toHaveText('Home');
    await expect(page.locator('#home-limits .sp-provider-card')).toHaveCount(3);
    for (const name of ['Claude Code', 'Codex', 'Antigravity']) {
      await expect(page.locator('#home-limits')).toContainText(name);
    }
    await expect(page.locator('#home-sessions')).toContainText('Recent');
    await expect(page.locator('#home-sessions .sp-session-row')).toHaveCount(2);
    await expect(page.locator('#home-today')).toContainText('Tokens today');
    await expect(page.locator('#home-today')).toContainText('Active sessions');
    await expect(page.locator('#home-activity')).toContainText('Last 12 weeks');
    const order = await page
      .locator('#home-limits, #home-sessions, #home-today, #home-activity')
      .evaluateAll((nodes) => nodes.map((node) => node.id));
    expect(order).toEqual(['home-limits', 'home-sessions', 'home-today', 'home-activity']);
  });

  test('a session row opens that conversation in Workspace', async ({ page, openConsole }) => {
    await openConsole();
    await page.locator('#home-sessions .sp-session-row', { hasText: 'sample-project' }).click();
    await expect(page.locator('#console-title')).toHaveText('Workspace');
    expect(new URL(page.url()).hash).toBe('#sessions');
    await expect(page.locator('.ide-session[aria-current="true"]')).toHaveCount(1);
  });

  test('View all opens the Workspace list', async ({ page, openConsole }) => {
    await openConsole();
    await page.getByRole('button', { name: /View all/ }).click();
    await expect(page.locator('#console-title')).toHaveText('Workspace');
    await expect(page.locator('.ide-session')).toHaveCount(2);
  });

  test('a provider with limits off offers a command that copies', async ({ page, context, openConsole }) => {
    await context.grantPermissions(['clipboard-read', 'clipboard-write']);
    await openConsole();
    const copy = page.locator('#home-limits .sp-provider-card[data-provider="claude-code"] [data-copy]');
    await copy.click();
    await expect(copy).toContainText('Copied');
    expect(await page.evaluate(() => navigator.clipboard.readText())).toBe('gnomcode limits enable claude');
  });
});

test.describe('Workspace', () => {
  test('lists conversations and opens one', async ({ page, openConsole }) => {
    await openConsole('#sessions');
    const rows = page.locator('.ide-session');
    await expect(rows).toHaveCount(2);
    await expect(page.getByText('1 / 1 · 2 conversations')).toBeVisible();
    await expect(page.locator('.sp-workspace-empty')).toContainText('Pick a conversation or start a new one.');
    await rows.first().click();
    await expect(page.locator('.ide-session[aria-current="true"]')).toHaveCount(1);
    await expect(page.locator('.sp-workspace-empty')).toHaveCount(0);
  });

  test('New opens the launcher with the agent, folder and model fields', async ({ page, openConsole }) => {
    await openConsole('#sessions');
    const root = page.locator('#local-workspace-root');
    await root.getByRole('button', { name: 'New', exact: true }).click();
    await expect(root.getByRole('heading', { name: 'New agent session' })).toBeVisible();
    await expect(root.locator('.ide-launcher').getByLabel('Agent')).toBeVisible();
    await expect(root.getByLabel('Workspace directory')).toBeVisible();
    await expect(root.getByLabel('Model (optional)')).toBeVisible();
    await expect(root.getByRole('button', { name: 'Launch Agent' })).toBeVisible();
  });

  test('the conversation list collapses and expands and remembers the choice', async ({ page, openConsole }) => {
    await openConsole('#sessions');
    const toggle = page.locator('#chat-list-toggle');
    await expect(page.locator('body')).not.toHaveClass(/chat-list-collapsed/);
    await expect(toggle).toHaveAttribute('aria-expanded', 'true');
    await toggle.click();
    await expect(page.locator('body')).toHaveClass(/chat-list-collapsed/);
    await expect(toggle).toHaveAttribute('aria-expanded', 'false');
    await page.reload();
    await expect(page.locator('body')).toHaveClass(/chat-list-collapsed/);
    await page.locator('#chat-list-toggle').click();
    await expect(page.locator('body')).not.toHaveClass(/chat-list-collapsed/);
  });
});

test.describe('Usage', () => {
  test('shows limits, the activity grid, spend by agent, projects and eval report', async ({ page, openConsole }) => {
    await openConsole('#usage');
    const root = page.locator('#local-usage-root');
    for (const text of ['Limits', 'Activity', 'Agent and model usage', 'Subscription value', 'Projects', 'Handoff quality evals']) {
      await expect(root).toContainText(text);
    }
    await expect(root).toContainText('sample-project');
  });

  test('the reporting period changes the report', async ({ page, openConsole }) => {
    await openConsole('#usage');
    const root = page.locator('#local-usage-root');
    const period = root.getByLabel('Reporting period');
    await expect(root).toContainText('Showing: All Time');
    await expect(period).toHaveValue('all');
    await period.selectOption('today');
    await expect(period).toHaveValue('today');
    await expect(root).toContainText('No recorded agent usage in this period.');
    await period.selectOption('all');
    await expect(root).toContainText('Showing: All Time');
  });

  test('export downloads a CSV and a JSON report', async ({ page, openConsole }) => {
    await openConsole('#usage');
    const root = page.locator('#local-usage-root');
    for (const extension of ['csv', 'json']) {
      await root.locator('summary', { hasText: 'Export' }).click();
      const [download] = await Promise.all([
        page.waitForEvent('download'),
        root.getByRole('button', { name: extension.toUpperCase(), exact: true }).click(),
      ]);
      expect(download.suggestedFilename()).toMatch(new RegExp(`\\.${extension}$`));
    }
  });

  test('selecting a project opens Workspace', async ({ page, openConsole }) => {
    await openConsole('#usage');
    await page.locator('#local-usage-root [data-project-id]').first().click();
    await expect(page.locator('#console-title')).toHaveText('Workspace');
  });
});

test.describe('Settings', () => {
  test('lists providers, appearance, devices, automations, notifications, updates and guide', async ({ page, openConsole }) => {
    await openConsole('#settings');
    const root = page.locator('#local-settings-root');
    for (const heading of ['Providers', 'Appearance', 'Devices and Remote', 'Automations', 'Notifications', 'Desktop updates', 'Guide', 'Account', 'Data']) {
      await expect(root.getByRole('heading', { name: heading })).toBeVisible();
    }
    await expect(root.getByText('Sign in to your account to make this desktop available.')).toBeVisible();
    await expect(root.getByText(/^Gnomcode \d/)).toBeVisible();
  });

  test('theme cycles system, light, dark and persists', async ({ page, openConsole }) => {
    await openConsole('#settings');
    const theme = page.locator('#local-settings-root').getByRole('button', { name: /^Theme/ });
    const html = page.locator('html');
    await expect(theme).toContainText('system');
    await theme.click();
    await expect(theme).toContainText('light');
    await expect(html).toHaveAttribute('data-theme', 'light');
    await theme.click();
    await expect(html).toHaveAttribute('data-theme', 'dark');
    await page.reload();
    await expect(html).toHaveAttribute('data-theme', 'dark');
    expect(await page.evaluate(() => localStorage.getItem('gnomcode-theme'))).toBe('dark');
    await theme.click();
    await expect(html).not.toHaveAttribute('data-theme', /.+/);
    expect(await page.evaluate(() => localStorage.getItem('gnomcode-theme'))).toBeNull();
  });

  test('notification preferences save and survive a reload', async ({ page, openConsole }) => {
    await openConsole('#settings');
    const root = page.locator('#local-settings-root');
    await root.getByLabel(/Limit low/).setChecked(false);
    await root.getByRole('button', { name: 'Save notifications' }).click();
    await expect(root.getByText('Saved notification preferences.')).toBeVisible();
    await page.reload();
    await expect(root.getByLabel(/Limit low/)).not.toBeChecked();
    await root.getByLabel(/Limit low/).setChecked(true);
    await root.getByRole('button', { name: 'Save notifications' }).click();
  });

  test('usage rules can be added and removed', async ({ page, openConsole }) => {
    await openConsole('#settings');
    const root = page.locator('#local-settings-root');
    await root.getByLabel('Spend limit (USD)').fill('7.5');
    await root.getByRole('button', { name: 'Add rule' }).click();
    const row = root.locator('tr', { hasText: '$7.50' });
    await expect(row).toBeVisible();
    await row.getByRole('button', { name: 'Delete' }).click();
    await expect(root.locator('tr', { hasText: '$7.50' })).toHaveCount(0);
  });

  test('Getting started opens the tour', async ({ page, openConsole }) => {
    await openConsole('#settings');
    await page.locator('#local-settings-root').getByRole('button', { name: 'Getting started' }).click();
    await expect(page.locator('#onboarding-dialog')).toBeVisible();
  });

  test('Manage account opens the account view', async ({ page, openConsole }) => {
    await openConsole('#settings');
    await page.locator('#local-settings-root').getByRole('button', { name: 'Manage account' }).click();
    await expect(page.locator('#console-title')).toHaveText('Account');
    expect(new URL(page.url()).hash).toBe('#auth');
  });
});

test('the Home activity grid is separated from the Today tiles', async ({ page, openConsole }) => {
  await openConsole();
  await expect(page.locator('#home-activity')).toContainText('Last 12 weeks');
  const today = await page.locator('#home-today').boundingBox();
  const activity = await page.locator('#home-activity').boundingBox();
  expect((activity?.y ?? 0) - ((today?.y ?? 0) + (today?.height ?? 0))).toBeGreaterThanOrEqual(16);
});

test.describe('Stale and reset quota states', () => {
  test('ticks countdowns every 30s, re-fetches at reset boundary, and renders reset_unconfirmed and stale states honestly', async ({
    page,
    openConsole,
  }) => {
    const t0 = new Date('2026-11-13T12:00:00Z').getTime();
    await page.clock.install({ time: t0 });

    let overviewCalls = 0;
    let crossedReset = false;
    await page.route('**/api/v1/providers/overview*', async (route) => {
      overviewCalls += 1;
      const isAfterReset = crossedReset;
      const codexWindow = isAfterReset
        ? {
            provider: 'codex',
            limit_id: 'codex',
            window: 'five_hour',
            window_label: 'Session',
            label: 'Session',
            state: 'reset_unconfirmed',
            used_pct: 0,
            remaining_pct: 100,
            resets_at: '2026-11-13T12:02:00Z',
            reset_passed: true,
            basis: 'estimated',
            stale: true,
            observed_at: '2026-11-13T10:00:00Z',
            source_label: 'Local logs',
            pace: {
              stale: true,
              stale_reason: 'based on data from 2h ago',
              runs_out_at: null,
            },
          }
        : {
            provider: 'codex',
            limit_id: 'codex',
            window: 'five_hour',
            window_label: 'Session',
            label: 'Session',
            state: 'low',
            used_pct: 88,
            remaining_pct: 12,
            resets_at: '2026-11-13T12:02:00Z',
            reset_passed: false,
            basis: 'exact',
            stale: false,
            observed_at: '2026-11-13T11:59:30Z',
            source_label: 'Local logs',
            pace: {
              stale: false,
              runs_out_at: '2026-11-13T12:02:10Z',
            },
          };

      const staleClaudeWindow = {
        provider: 'claude-code',
        limit_id: 'claude-code',
        window: 'five_hour',
        window_label: 'Session',
        label: 'Session',
        state: 'low',
        used_pct: 99,
        remaining_pct: 1,
        resets_at: '2026-11-13T13:06:00Z',
        reset_passed: false,
        basis: 'exact',
        stale: true,
        observed_at: '2026-11-13T10:00:00Z',
        source_label: 'Local logs',
        pace: {
          stale: true,
          stale_reason: 'based on data from 2h ago',
          runs_out_at: null,
        },
      };

      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([
          {
            provider: 'codex',
            display_name: 'Codex',
            plan: 'pro',
            state: codexWindow.state,
            source_label: 'Local logs',
            observed_at: codexWindow.observed_at,
            headline_window: codexWindow,
            windows: [codexWindow],
          },
          {
            provider: 'claude-code',
            display_name: 'Claude Code',
            plan: 'max',
            state: staleClaudeWindow.state,
            source_label: 'Local logs',
            observed_at: staleClaudeWindow.observed_at,
            headline_window: staleClaudeWindow,
            windows: [staleClaudeWindow],
          },
        ]),
      });
    });

    await openConsole();
    const codexCard = page.locator('#home-limits .sp-provider-card[data-provider="codex"]');
    const claudeCard = page.locator('#home-limits .sp-provider-card[data-provider="claude-code"]');

    // Stale card never shows a live "Runs out in" projection and surfaces "Seen 2h ago" near the numbers.
    await claudeCard.locator('summary').click();
    await expect(claudeCard).toContainText('Last seen 2h ago');
    await expect(claudeCard).toContainText('Based on data from 2h ago');
    await expect(claudeCard).not.toContainText(/Runs out in/i);

    await page.screenshot({
      path: 'C:/Users/Wahaj/.gemini/antigravity/brain/5751486b-f1ee-46eb-aac7-95ecc2468411/scratch/stale-quota-after.png',
    });

    // Live card ticks its reset and runs-out countdowns every 30s without waiting for a poll.
    await codexCard.locator('summary').click();
    await expect(codexCard).toContainText('Resets in 2m');
    await expect(codexCard).toContainText('Runs out in 2m');

    await page.clock.runFor(61_000);
    await expect(codexCard).toContainText('Resets in 1m');
    await expect(codexCard).toContainText('Runs out in 1m');

    // Crossing the reset boundary triggers an immediate overview re-fetch and shows Reset, not confirmed + EST.
    const callsBeforeBoundary = overviewCalls;
    crossedReset = true;
    await page.clock.runFor(61_000);
    await expect.poll(() => overviewCalls).toBeGreaterThan(callsBeforeBoundary);
    await expect(codexCard).toContainText('Reset, not confirmed');
    await expect(codexCard).toContainText('EST');

    await page.screenshot({
      path: 'C:/Users/Wahaj/.gemini/antigravity/brain/5751486b-f1ee-46eb-aac7-95ecc2468411/scratch/reset-unconfirmed-after.png',
    });
  });
});

test.describe('Owner decision: brand-red activity heatmap', () => {
  const ramps = {
    dark: ['#17171a', '#4a121b', '#7d1626', '#ba1c35', '#ff2544'],
    light: ['#e4e9ee', '#fcd1d8', '#f799a8', '#ec526b', '#ff2544'],
  } as const;
  for (const scheme of ['dark', 'light'] as const) {
    test(`heat tokens and rendered cells keep the brand-red ramp in ${scheme} mode`, async ({ page, openConsole }) => {
      await page.emulateMedia({ colorScheme: scheme });
      await openConsole();
      await expect(page.locator('#home-activity .sp-heat[data-level]').first()).toBeVisible();
      const tokens = await page.evaluate(() =>
        [0, 1, 2, 3, 4].map((n) => getComputedStyle(document.documentElement).getPropertyValue(`--sp-heat-${n}`).trim().toLowerCase()),
      );
      expect(tokens).toEqual([...ramps[scheme]]);
      const cells = await page.locator('#home-activity .sp-heat[data-level]').evaluateAll((nodes) =>
        nodes.map((node) => [node.getAttribute('data-level'), getComputedStyle(node).backgroundColor] as const),
      );
      const hex = (value: string) => {
        const [r, g, b] = value.match(/\d+/g)!.map(Number);
        return `#${[r, g, b].map((n) => n.toString(16).padStart(2, '0')).join('')}`;
      };
      for (const [level, color] of cells) expect(hex(color)).toBe(ramps[scheme][Number(level)]);
    });
  }
});
