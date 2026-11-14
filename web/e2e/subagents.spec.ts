import { expect, test } from './fixtures';

const child = (id: string, name: string, state: string, ongoing: boolean, minute: number) => ({
  id, title: name, name, state, ongoing, model: 'Unspecified',
  started_at: `2026-11-14T10:0${minute}:00Z`, last_event_at: `2026-11-14T10:1${minute}:00Z`,
});

/** Serves one parent with three subagents and mimics the daemon: unchecked drops the nested list. */
async function mockSubagentConversations(page: import('@playwright/test').Page) {
  await page.route('**/api/v1/conversations?*', async (route) => {
    const include = new URL(route.request().url()).searchParams.get('include_subagents') === 'true';
    const parent: Record<string, unknown> = {
      id: 'claude-code:parent', agent: 'claude-code', title: 'Parent chat', model: 'opus',
      state: 'thinking', state_label: 'Working · 2 subagents running', source: 'imported',
      project_name: 'demo', last_event_at: new Date().toISOString(), started_at: '2026-11-14T09:00:00Z',
      subagent_count: 3, subagents_ongoing: 2,
      subagents: [
        child('claude-code:a', 'Wire watcher into daemon', 'tool', true, 3),
        child('claude-code:b', 'Subagent 2', 'thinking', true, 2),
        child('claude-code:c', 'Subagent 1', 'idle', false, 1),
      ],
    };
    if (!include) delete parent.subagents;
    await route.fulfill({ json: { sessions: [parent], page: 1, total_pages: 1, total_count: 1 } });
  });
}

test.describe('subagents in the sessions sidebar', () => {
  test('parent reflects running children and subagents carry their real names', async ({ page, openConsole }) => {
    await mockSubagentConversations(page);
    await openConsole('#sessions');
    const sidebar = page.locator('#ide-session-sidebar');
    const parent = sidebar.locator('.ide-session', { hasText: 'Parent chat' });
    await expect(parent).toContainText('Working · 2 subagents running');
    await expect(parent).not.toContainText('Idle');
    const group = sidebar.locator('.sp-subagents');
    await expect(group.locator('summary')).toHaveText('2 running · 3 subagents');
    await expect(group).not.toHaveAttribute('open', '');
    await group.locator('summary').click();
    const rows = group.locator('.sp-subagent-row strong');
    await expect(rows).toHaveText(['Wire watcher into daemon', 'Subagent 2', 'Subagent 1']);
  });

  test('Include subagents toggles nested rows everywhere and the choice persists', async ({ page, openConsole }) => {
    await mockSubagentConversations(page);
    await openConsole('#sessions');
    const sidebar = page.locator('#ide-session-sidebar');
    await expect(sidebar.locator('.sp-subagents')).toHaveCount(1);
    await sidebar.locator('.conversation-filters summary').click();
    const toggle = sidebar.getByLabel('Include subagents');
    await expect(toggle).toBeChecked();
    await toggle.uncheck();
    await expect(sidebar.locator('.sp-subagents')).toHaveCount(0);
    await expect(sidebar.locator('.sp-subagent-count')).toHaveText('2 running · 3 subagents');
    await page.reload();
    await sidebar.locator('.conversation-filters summary').click();
    await expect(sidebar.getByLabel('Include subagents')).not.toBeChecked();
    await expect(sidebar.locator('.sp-subagents')).toHaveCount(0);
    await sidebar.getByLabel('Include subagents').check();
    await expect(sidebar.locator('.sp-subagents')).toHaveCount(1);
  });
});
