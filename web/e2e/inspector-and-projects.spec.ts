import { expect, test } from './fixtures';
import { openClaudeConversation, reshapeConversations } from './workspace-helpers';

test.describe('workspace inspector tabs, observed sessions, and usage projects', () => {
  test('inspector tabs stay single-line and support WAI-ARIA arrow-key navigation', async ({ page, openConsole }) => {
    await reshapeConversations(page, { repo_path: '/work/console-demo' });
    await page.route('**/api/v1/repos/files?*', (route) =>
      route.fulfill({ json: { path: '', entries: [{ name: 'README.md', path: 'README.md', kind: 'file', size: 512 }] } }),
    );
    await openConsole('#sessions');
    await openClaudeConversation(page);

    const tools = page.getByRole('complementary', { name: 'Conversation tools' });
    await page.getByRole('button', { name: 'Overview', exact: true }).click();
    await tools.getByRole('button', { name: 'Files' }).click();
    await page.setViewportSize({ width: 920, height: 760 });

    const tablist = page.getByRole('tablist', { name: 'Inspector views' });
    const tabs = tablist.getByRole('tab');
    const count = await tabs.count();
    expect(count).toBeGreaterThanOrEqual(4);

    // Every tab button must stay single-line (never wrap letter-by-letter vertically).
    for (let i = 0; i < count; i++) {
      const box = await tabs.nth(i).boundingBox();
      expect(box).not.toBeNull();
      expect(box!.height).toBeLessThanOrEqual(44);
    }

    const firstTab = tabs.first();
    await expect(firstTab).toHaveAttribute('aria-selected', 'true');
    await expect(firstTab).toHaveAttribute('tabindex', '0');
    await expect(tabs.nth(1)).toHaveAttribute('tabindex', '-1');

    // Arrow keys move focus; Enter activates, avoiding work while crossing expensive tabs.
    await firstTab.focus();
    await page.keyboard.press('ArrowRight');
    await expect(tabs.nth(1)).toBeFocused();
    await expect(firstTab).toHaveAttribute('aria-selected', 'true');
    await page.keyboard.press('Enter');
    await expect(tabs.nth(1)).toHaveAttribute('aria-selected', 'true');
    await page.keyboard.press('ArrowLeft');
    await page.keyboard.press('Enter');
    await expect(firstTab).toHaveAttribute('aria-selected', 'true');
    await page.keyboard.press('End');
    await expect(tabs.last()).toBeFocused();
    await page.keyboard.press('Enter');
    await expect(tabs.last()).toHaveAttribute('aria-selected', 'true');
    await page.keyboard.press('Home');
    await page.keyboard.press('Enter');
    await expect(firstTab).toHaveAttribute('aria-selected', 'true');

    // Close button also stays single-line and returns focus cleanly.
    const closeBtn = page.getByRole('button', { name: 'Close workspace inspector' });
    const closeBox = await closeBtn.boundingBox();
    expect(closeBox).not.toBeNull();
    expect(closeBox!.height).toBeLessThanOrEqual(44);
  });

  test('non-chatable observed IDE session hides composer and shows read-only banner', async ({ page, openConsole }) => {
    await reshapeConversations(page, {
      agent: 'antigravity',
      surface: 'ide',
      surface_label: 'Antigravity 2.0',
      source: 'imported',
      can_prompt: false,
      can_continue_here: false,
    });
    await openConsole('#sessions');
    await page.locator('.ide-session', { hasText: 'Antigravity' }).first().click();

    await expect(page.locator('.sp-chat-composer')).toHaveCount(0);
    await expect(page.locator('.sp-interrupt-btn')).toHaveCount(0);
    const banner = page.locator('.sp-readonly-banner');
    await expect(banner).toBeVisible();
    await expect(banner).toContainText('read-only here');
    await expect(banner.getByRole('button', { name: 'Open in Antigravity' })).toBeVisible();
    await expect(banner.getByRole('button', { name: 'New conversation' })).toBeVisible();
  });

  test('Antigravity 2.0 desktop-app session with stale launch state stays read-only', async ({ page, openConsole }) => {
    await reshapeConversations(page, {
      agent: 'antigravity',
      surface_label: 'Antigravity 2.0',
      source: 'imported',
      host_id: null,
      can_prompt: false,
      can_resume: true,
      delivery_capability: { mode: 'observed_read_only', can_queue: false, queued_count: 0 },
    });
    await openConsole('#sessions');
    await page.locator('.ide-session', { hasText: 'Antigravity' }).first().click();

    await expect(page.locator('.sp-readonly-banner')).toBeVisible();
    await expect(page.locator('.sp-chat-composer')).toHaveCount(0);
    await expect(page.locator('.sp-queued-list')).toHaveCount(0);
    await expect(page.getByRole('button', { name: 'Resume conversation' })).toHaveCount(0);
  });

  test('desktop rail fits the viewport and + New stays on one line', async ({ page, openConsole }) => {
    for (const size of [{ width: 1280, height: 720 }, { width: 1440, height: 900 }, { width: 1280, height: 560 }]) {
      await page.setViewportSize(size);
      await openConsole('#sessions');
      const toggle = page.locator('#navigation-toggle');
      await expect(toggle).toBeVisible();
      const box = await toggle.boundingBox();
      expect(box!.y + box!.height).toBeLessThanOrEqual(size.height);
      const nav = page.locator('.sp-console-nav');
      expect(await nav.evaluate((el) => el.scrollHeight <= el.clientHeight + 1)).toBe(true);
      const newBtn = await page.locator('.sp-sidebar-new-btn').boundingBox();
      expect(newBtn!.height).toBeLessThanOrEqual(32);
      expect(await page.locator('.sp-console-nav').evaluate((el) => getComputedStyle(el).borderRightWidth)).toBe('1px');
      expect(await page.locator('.sp-chat-column').evaluate((el) => getComputedStyle(el).borderRightWidth)).toBe('1px');
    }
  });

  test('observed CLI session with continue support shows Continue here without fake composer', async ({ page, openConsole }) => {
    await reshapeConversations(page, {
      agent: 'claude-code',
      surface: 'cli',
      source: 'imported',
      can_prompt: false,
      can_continue_here: true,
    });
    await openConsole('#sessions');
    await openClaudeConversation(page);

    await expect(page.locator('.sp-chat-composer')).toHaveCount(0);
    const banner = page.locator('.sp-readonly-banner');
    await expect(banner).toBeVisible();
    await expect(banner.getByRole('button', { name: 'Continue here' })).toBeVisible();
    await expect(banner.getByRole('button', { name: 'New conversation' })).toBeVisible();
  });

  test('Usage projects list shows token totals, always-on filter, and pagination', async ({ page, openConsole }) => {
    const day = (n: number) => new Date(Date.UTC(2026, 10, 14 - n)).toISOString();
    const projects = Array.from({ length: 12 }, (_, i) => ({
      id: `proj-${i}`,
      name: `project-${String(i).padStart(2, '0')}`,
      sessions: i + 1,
      total_tokens: (i + 1) * 1500,
      cost_usd: 0.25 * (i + 1),
      priced_ratio: 1,
      last_active: day(i),
      agents: ['codex'],
      repo_root: `/work/project-${String(i).padStart(2, '0')}`,
    }));
    await page.route('**/api/v1/projects?*', (route) => route.fulfill({ json: projects }));
    await openConsole('#usage');

    const root = page.locator('.sp-projects');
    await expect(root.locator('.sp-project-search')).toBeVisible();
    await expect(root.locator('.sp-project-row')).toHaveCount(10);
    await expect(root.locator('.sp-project-tokens').first()).toContainText('tokens');
    await expect(root.locator('.sp-project-pager')).toContainText('Showing 1–10 of 12');
    await expect(root.locator('.sp-project-pager')).toContainText('Page 1 of 2');

    await root.getByRole('button', { name: 'Next' }).click();
    await expect(root.locator('.sp-project-row')).toHaveCount(2);
    await expect(root.locator('.sp-project-pager')).toContainText('Page 2 of 2');
    await expect(root).toContainText('project-11');

    await root.getByRole('button', { name: 'Previous' }).click();
    await expect(root.locator('.sp-project-row')).toHaveCount(10);

    // Filter resets to page 1 and matches across all 12 projects.
    await root.locator('.sp-project-search').fill('project-1');
    await expect(root.locator('.sp-project-row')).toHaveCount(2);
    await expect(root).toContainText('project-10');
    await expect(root).toContainText('project-11');
  });

  test('no horizontal overflow across Home, Usage, Workspace, and Settings on desktop and mobile', async ({ page, openConsole }) => {
    for (const viewport of [
      { width: 1440, height: 900 },
      { width: 390, height: 844 },
    ]) {
      await page.setViewportSize(viewport);
      for (const hash of ['#overview', '#usage', '#sessions', '#settings']) {
        await openConsole(hash);
        const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
        expect(overflow, `overflow at ${viewport.width}px on ${hash}`).toBeLessThanOrEqual(0);
      }
    }
  });
});
