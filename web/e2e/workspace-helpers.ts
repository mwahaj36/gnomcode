import type { Page } from '@playwright/test';
import { expect } from './fixtures';

type Row = Record<string, unknown>;

/**
 * Rewrites the daemon's real conversation list so the Claude fixture session looks like one Gnomcode
 * launched (managed), optionally running and with a workspace folder. Everything else is the real daemon.
 */
export async function reshapeConversations(page: Page, patch: Row) {
  await page.route('**/api/v1/conversations?*', async (route) => {
    try {
      const response = await route.fetch();
      const body = await response.json();
      const rows: Row[] = Array.isArray(body) ? body : body.sessions;
      for (const row of rows) if (String(row.id).startsWith('claude-code:')) Object.assign(row, patch);
      await route.fulfill({ response, json: body });
    } catch {
      /* the test ended while a poll was in flight */
    }
  });
}

export async function openClaudeConversation(page: Page) {
  await page.locator('.ide-session', { hasText: 'claude-code' }).click();
  await expect(page.locator('.ide-session[aria-current="true"]')).toHaveCount(1);
}
