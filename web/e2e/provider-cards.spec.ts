import type { Page } from '@playwright/test';
import { expect, test } from './fixtures';

// Every surface renders the same provider card markup, so one broken summary rule breaks all of
// them at once. 1.0.4 shipped exactly that: a global disclosure marker was injected into the
// card's fixed four-column summary grid, which spilled a second chevron onto a new row, overlapped
// the provider logo with the summary text and pushed each card past its column.
const provider = (slug: string, name: string, plan: string | null) => ({
  provider: slug,
  name,
  plan,
  state: 'low',
  basis: 'api',
  observed_at: new Date().toISOString(),
  windows: [
    {
      limit_id: 'weekly',
      label: 'Weekly',
      used_pct: 87,
      resets_at: new Date(Date.now() + 2 * 86_400_000).toISOString(),
      basis: 'api',
    },
  ],
  models: [],
  source: 'API',
});

async function withProviders(page: Page) {
  await page.route('**/api/v1/providers/overview', (route: import('@playwright/test').Route) =>
    route.fulfill({
      json: [
        provider('claude-code', 'Claude Code', null),
        provider('codex', 'Codex', 'plus'),
        provider('antigravity', 'Antigravity', null),
      ],
    }),
  );
}

async function assertCardsAreIntact(page: Page, scope: string) {
  const cards = page.locator(`${scope} details.sp-provider-card`);
  await expect(cards.first()).toBeVisible();

  const report = await page.evaluate((sel: string) => {
    const found = Array.from(document.querySelectorAll<HTMLElement>(`${sel} details.sp-provider-card`));
    return found.map((card) => {
      const summary = card.querySelector<HTMLElement>(':scope > summary')!;
      const box = card.getBoundingClientRect();
      return {
        injected: getComputedStyle(summary, '::before').content,
        summaryHeight: summary.getBoundingClientRect().height,
        right: box.right,
        left: box.left,
        top: box.top,
        // A spilled grid item makes the summary wider than the card it sits in.
        overflows: summary.scrollWidth > summary.clientWidth + 1,
      };
    });
  }, scope);

  expect(report.length, `${scope}: provider cards rendered`).toBeGreaterThanOrEqual(3);
  for (const [i, card] of report.entries()) {
    expect(card.injected, `${scope} card ${i}: no injected ::before marker`).toBe('none');
    expect(card.summaryHeight, `${scope} card ${i}: summary stays one row`).toBeLessThanOrEqual(90);
    expect(card.overflows, `${scope} card ${i}: summary fits its card`).toBe(false);
  }
  // Cards sharing a row must not overlap each other.
  for (let i = 1; i < report.length; i++) {
    if (Math.abs(report[i].top - report[i - 1].top) > 5) continue;
    expect(report[i].left, `${scope}: card ${i} overlaps card ${i - 1}`).toBeGreaterThanOrEqual(
      report[i - 1].right - 1,
    );
  }

  // Exactly one chevron per summary, the card's own.
  const chevrons = await page.evaluate(
    (sel: string) =>
      Array.from(document.querySelectorAll(`${sel} details.sp-provider-card > summary`)).map(
        (s) => s.querySelectorAll('.sp-chevron').length,
      ),
    scope,
  );
  for (const [i, count] of chevrons.entries()) {
    expect(count, `${scope} card ${i}: exactly one chevron`).toBe(1);
  }
}

test.describe('provider limit cards render intact on every surface', () => {
  test.use({ viewport: { width: 1440, height: 900 } });

  test('Usage limits row', async ({ page, openConsole }) => {
    await withProviders(page);
    await openConsole('#usage');
    await assertCardsAreIntact(page, '.sp-usage-view');
  });

  test('Home limits', async ({ page, openConsole }) => {
    await withProviders(page);
    await openConsole('#overview');
    await assertCardsAreIntact(page, '.sp-home');
  });

  test('Workspace provider usage popover', async ({ page, openConsole }) => {
    await withProviders(page);
    await openConsole('#sessions');
    await page.locator('.ide-session').first().click();
    const trigger = page.locator('.sp-limits-popover-trigger').first();
    await expect(trigger).toBeVisible();
    await trigger.click();
    await assertCardsAreIntact(page, '.sp-usage-popover');
  });
});
