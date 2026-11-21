import { expect, test } from './fixtures';

for (const width of [1440, 844, 390]) {
  test(`Workspace notices do not overlap navigation at ${width}px`, async ({ page, openConsole }) => {
    await page.setViewportSize({ width, height: width === 844 ? 390 : 844 });
    await page.route('**/api/v1/approvals', route => route.fulfill({ json: {
      projects: ['/work/project'],
      grants: [{ session_id: 'test-session', agent: 'claude-code', tool: 'Read' }],
      items: [{ id: 'test-approval', agent: 'claude-code', tool: 'Bash', summary: 'Read repository status before continuing' }],
    } }));
    await openConsole('#sessions');
    const notices = page.locator('#approvals');
    await expect(notices).toBeVisible();
    const toolbar = page.locator('.console-context');
    const top = await toolbar.boundingBox();
    const bottom = await notices.boundingBox();
    expect(top!.y + top!.height).toBeLessThanOrEqual(bottom!.y + 1);
    const toggle = page.locator('#chat-list-toggle');
    if (await toggle.isVisible()) {
      const button = await toggle.boundingBox();
      expect(button!.y + button!.height).toBeLessThanOrEqual(bottom!.y + 1);
    }
    await expect(notices.getByRole('button', { name: 'Approve Bash once' })).toBeAttached();
    expect(await notices.evaluate(node => node.scrollWidth - node.clientWidth)).toBeLessThanOrEqual(1);
  });
}
