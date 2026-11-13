import { expect, test } from './fixtures';
import type { Daemon } from './daemon';

/** What the Claude Code hook sends when a tool needs a person's decision. */
async function requestApproval(daemon: Daemon, tool: string, summary: string) {
  const response = await fetch(`${daemon.base}/api/v1/approvals`, {
    method: 'POST',
    headers: { Authorization: `Bearer ${daemon.token}`, 'Content-Type': 'application/json' },
    body: JSON.stringify({ agent: 'claude-code', session_id: 'e2e-session', tool, summary }),
  });
  expect(response.status).toBe(200);
  return (await response.json()) as { hold: boolean; id: string };
}

async function outcome(daemon: Daemon, id: string): Promise<string> {
  const response = await fetch(`${daemon.base}/api/v1/approvals/${id}/wait`, {
    headers: { Authorization: `Bearer ${daemon.token}` },
  });
  return ((await response.json()) as { outcome: string }).outcome;
}

test('the card is hidden until a hook asks, then shows the tool and summary', async ({ page, daemon, openConsole }) => {
  await openConsole();
  const region = page.locator('#approvals');
  await expect(region).toBeHidden();
  // The console polling the list is what tells the daemon someone is watching.
  await expect.poll(async () => (await requestApproval(daemon, 'Bash', 'git status')).hold, { timeout: 15_000 }).toBe(true);
  await expect(region).toBeVisible();
  const card = region.locator('.sp-approval').first();
  await expect(card).toContainText('Claude Code wants to run');
  await expect(card).toContainText('Bash');
  await expect(card).toContainText('git status');
  await expect(card.getByRole('button', { name: 'Approve Bash once' })).toBeVisible();
  await expect(card.getByRole('button', { name: 'Deny Bash' })).toBeVisible();
  await card.getByRole('button', { name: 'Deny Bash' }).click();
  await expect(region).toBeHidden();
});

test('Approve once sends exactly one allow decision for that id', async ({ page, daemon, openConsole }) => {
  await openConsole();
  let held: { hold: boolean; id: string } = { hold: false, id: '' };
  await expect
    .poll(async () => {
      held = await requestApproval(daemon, 'Edit', 'src/main.rs');
      return held.hold;
    }, { timeout: 15_000 })
    .toBe(true);
  const decisions: string[] = [];
  page.on('request', (request) => {
    if (request.url().includes('/decision')) decisions.push(`${request.method()} ${request.url()} ${request.postData()}`);
  });
  const card = page.locator(`[data-approval-id="${held.id}"]`);
  await card.getByRole('button', { name: 'Approve Edit once' }).click();
  expect(await outcome(daemon, held.id)).toBe('allow');
  await expect(page.locator('#approvals')).toBeHidden();
  expect(decisions).toHaveLength(1);
  expect(decisions[0]).toContain(`/api/v1/approvals/${held.id}/decision`);
  expect(decisions[0]).toContain('"decision":"allow"');
});

test('Deny records a deny decision', async ({ page, daemon, openConsole }) => {
  await openConsole();
  let held: { hold: boolean; id: string } = { hold: false, id: '' };
  await expect
    .poll(async () => {
      held = await requestApproval(daemon, 'Write', 'notes.md');
      return held.hold;
    }, { timeout: 15_000 })
    .toBe(true);
  await page.locator(`[data-approval-id="${held.id}"]`).getByRole('button', { name: 'Deny Write' }).click();
  expect(await outcome(daemon, held.id)).toBe('deny');
});

test('tool names and summaries are shown as text, never as markup', async ({ page, daemon, openConsole }) => {
  await openConsole();
  await expect
    .poll(async () => (await requestApproval(daemon, 'Bash', '<img src=x onerror=window.__xss=1>')).hold, { timeout: 15_000 })
    .toBe(true);
  const card = page.locator('#approvals .sp-approval').last();
  await expect(card.locator('.sp-approval-summary')).toContainText('<img src=x onerror=window.__xss=1>');
  expect(await page.evaluate(() => (window as unknown as { __xss?: number }).__xss)).toBeUndefined();
  await expect(card.locator('img')).toHaveCount(0);
  await card.getByRole('button', { name: 'Deny Bash' }).click();
});

test('the approval card stays above every view and keeps 44px targets on a phone', async ({ page, daemon, openConsole }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await openConsole('#usage');
  await expect
    .poll(async () => (await requestApproval(daemon, 'Bash', 'ls')).hold, { timeout: 15_000 })
    .toBe(true);
  const card = page.locator('#approvals .sp-approval').last();
  await expect(card).toBeVisible();
  for (const button of await card.getByRole('button').all()) {
    const box = await button.boundingBox();
    expect(box?.height ?? 0).toBeGreaterThanOrEqual(44);
  }
  await card.getByRole('button', { name: 'Deny Bash' }).click();
});
