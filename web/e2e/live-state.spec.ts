import type { Page } from '@playwright/test';
import { expect, test } from './fixtures';
import { reshapeConversations } from './workspace-helpers';

const STATES = [
  { state: 'working', label: 'Working', step: 'Running a command', basis: 'exact' },
  { state: 'waiting_for_you', label: 'Waiting for you', step: '', basis: 'estimated' },
  { state: 'idle', label: 'Idle', step: '', basis: 'exact' },
  { state: 'stopped', label: 'Stopped', step: '', basis: 'exact' },
] as const;

async function open(page: Page, openConsole: (hash: string) => Promise<void>, live: (typeof STATES)[number]) {
  const startedAt = new Date(Date.now() - 65_000).toISOString();
  await reshapeConversations(page, {
    source: 'managed',
    hosted: true,
    state: live.state === 'working' ? 'tool' : live.state === 'waiting_for_you' ? 'awaiting_approval' : live.state === 'stopped' ? 'ended' : 'idle',
    live: { state: live.state, basis: live.basis, step: live.step },
    ...(live.state === 'working' ? { turn_started_at: startedAt } : {}),
  });
  await openConsole('#sessions');
  const toggle = page.locator('#chat-list-toggle');
  if (await toggle.isVisible() && (await toggle.getAttribute('aria-expanded')) === 'false') await toggle.click();
  await page.locator('.ide-session', { hasText: 'claude-code' }).first().click();
}

for (const [name, viewport] of [['desktop', { width: 1280, height: 900 }], ['phone', { width: 390, height: 844 }]] as const) {
  test.describe(`live state on ${name}`, () => {
    test.use({ viewport });
    for (const live of STATES) {
      test(`${live.label} shows text and shape in header, list and transcript`, async ({ page, openConsole }) => {
        await open(page, openConsole, live);
        const header = page.locator('.sp-conversation-live .sp-live');
        await expect(header).toHaveAttribute('data-live', live.state);
        await expect(header.locator('.sp-live-label')).toHaveText(live.label);
        await expect(header.locator('.sp-live-shape')).toHaveCount(1);
        await expect(header.locator('.sp-live-basis')).toHaveCount(0);
        await expect(page.locator('.ide-session[aria-current="true"] .sp-live')).toHaveAttribute('data-live', live.state);
        const activity = page.getByRole('status', { name: 'Agent activity' });
        const box = page.getByRole('textbox', { name: 'Message the agent' });
        if (live.state === 'working') {
          await expect(activity).toContainText('Running a command');
          await expect(activity.locator('.sp-activity-elapsed')).toHaveText(/^1m \d\ds$/);
          await expect(activity.getByRole('button', { name: 'Stop' })).toBeVisible();
          await expect(box).toHaveAttribute('placeholder', /Queue a message/);
          await expect(page.getByRole('button', { name: 'Queue' })).toBeVisible();
        } else {
          await expect(activity).toHaveCount(0);
          await expect(box).toHaveAttribute('placeholder', 'Message the agent...');
        }
        expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
      });
    }

    test('reduced motion stops the working spinner', async ({ page, openConsole }) => {
      await page.emulateMedia({ reducedMotion: 'reduce' });
      await open(page, openConsole, STATES[0]);
      const shape = page.locator('.sp-conversation-live .sp-live-shape');
      expect(await shape.evaluate((el) => getComputedStyle(el).animationName)).toBe('none');
    });
  });
}
