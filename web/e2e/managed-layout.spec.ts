import { expect, test } from './fixtures';
import { openClaudeConversation, reshapeConversations } from './workspace-helpers';

for (const size of [{ width: 1440, height: 900 }, { width: 640, height: 850 }, { width: 844, height: 390 }]) {
  test(`managed work and notices ${size.width}x${size.height}`, async ({ page, openConsole }, info) => {
    await page.setViewportSize(size);
    await page.addInitScript(() => localStorage.setItem('gnomcode-theme', 'dark'));
    await reshapeConversations(page, { source: 'managed', hosted: true, repo_path: '/work/gnomcode', model: 'gpt-6.1-sol', title: 'Review workspace navigation, permissions and responsive layouts across the entire console' });
    await page.route('**/api/v1/model-catalog', route => route.fulfill({ json: { providers: [{ name: 'OpenAI', models: [{ agent: 'codex', model: 'gpt-6.1-sol', display_name: 'GPT 6.1 Sol' }, { agent: 'codex', model: 'o3', display_name: 'o3' }] }] } }));
    await page.route('**/api/v1/sessions/*/chat?*', route => route.fulfill({ json: [
      { event_id: 'user', role: 'user', content: 'Review the navigation and permission flows. Keep my unsent work intact when I switch between the conversation and tools.', turn_index: 1, created_at: '2026-11-20T12:00:00Z' },
      { event_id: 'assistant', role: 'assistant', content: 'I checked the workspace layout.\n\n- Conversations have a dedicated history drawer.\n- Review tools use the available work area.\n- Permission requests stay below the page toolbar.\n\n```ts\nconst layout = { navigation: "compact", tools: "available" };\n```', turn_index: 2, created_at: '2026-11-20T12:01:00Z' },
    ] }));
    await page.route('**/api/v1/sessions/*/setup', route => route.fulfill({ json: {} }));
    const switches: Record<string, unknown>[] = [];
    await page.route('**/api/v1/sessions/*/switch', route => {
      switches.push(route.request().postDataJSON());
      return route.fulfill({ status: 502, json: { message: 'Model switch unavailable in this fixture.' } });
    });
    let notices = false;
    await page.route('**/api/v1/approvals', route => route.fulfill({ json: notices ? {
      items: [{ id: 'review', agent: 'claude-code', tool: 'Bash', summary: 'git status --short' }],
      projects: ['/work/gnomcode'], grants: [{ session_id: 'managed-review', agent: 'claude-code', tool: 'Read' }],
    } : { items: [], grants: [], projects: [] } }));
    await page.route('**/api/v1/sessions/*/terminal', route => route.fulfill({ json: { session_id: 'test-terminal', repo_path: '/work/gnomcode' } }));
    await page.routeWebSocket('**/api/v1/sessions/*/pty?*', socket => {
      socket.send('Gnomcode workspace terminal\r\n/work/gnomcode > git status --short\r\n M web/app/gnomcode-console.css\r\n M web/local-console/ConsoleShell.tsx\r\n/work/gnomcode > ');
    });
    await openConsole('#sessions');
    const toggle = page.locator('#chat-list-toggle');
    if (await toggle.getAttribute('aria-expanded') === 'false') await toggle.click();
    await openClaudeConversation(page);
    const composer = page.getByRole('textbox', { name: 'Message the agent' });
    await composer.fill('Keep this draft while I review the changed files.');
    await expect(composer).toBeVisible();
    expect(await composer.evaluate(node => getComputedStyle(node).outlineStyle)).toBe('none');
    if (size.width < 900) {
      for (const control of [page.getByRole('button', { name: 'Overview', exact: true }), page.getByRole('button', { name: 'Remaining limits — View Usage' }), page.getByLabel('Conversation actions', { exact: true }), page.getByLabel('Model', { exact: true }), page.getByRole('button', { name: 'Attach', exact: true }), page.getByRole('button', { name: 'Send', exact: true })]) {
        expect((await control.boundingBox())!.height).toBeGreaterThanOrEqual(44);
      }
    }
    const send = page.getByRole('button', { name: 'Send', exact: true });
    const sendBox = await send.boundingBox();
    expect(sendBox!.y + sendBox!.height).toBeLessThanOrEqual(size.height - (size.width < 900 ? 56 : 0));
    await page.screenshot({ path: info.outputPath('managed-composer.png') });
    await page.getByLabel('Model', { exact: true }).click();
    await page.screenshot({ path: info.outputPath('model-picker.png') });
    await page.keyboard.press('Escape');
    notices = true;
    await expect(page.locator('#approvals')).toBeVisible();
    const approvalSendBox = await send.boundingBox();
    const layout = await page.locator('.sp-workspace-main-col').evaluate(node => [node, ...node.children].map(el => ({ name: el.className, height: el.getBoundingClientRect().height, padding: getComputedStyle(el).padding, minHeight: getComputedStyle(el).minHeight })));
    expect(approvalSendBox!.y + approvalSendBox!.height, JSON.stringify(layout)).toBeLessThanOrEqual(size.height - (size.width < 900 ? 56 : 0));
    await page.screenshot({ path: info.outputPath('approval.png') });
    await page.locator('#approvals').evaluate(node => { node.scrollTop = node.scrollHeight; });
    await page.screenshot({ path: info.outputPath('trust-grants.png') });
    notices = false;
    await expect(page.locator('#approvals')).toBeHidden();
    await page.getByRole('button', { name: 'Overview', exact: true }).click();
    await page.getByRole('complementary', { name: 'Conversation tools' }).getByRole('button', { name: 'Agent terminal', exact: true }).click();
    await expect(page.locator('.sp-chat-terminal-box .xterm')).toBeVisible();
    await page.screenshot({ path: info.outputPath('agent-terminal.png') });
    await page.getByRole('tab', { name: 'Terminal', exact: true }).click();
    await expect(page.locator('.sp-workspace-terminal-screen .xterm')).toBeVisible();
    await page.screenshot({ path: info.outputPath('workspace-terminal.png') });
    await page.getByRole('button', { name: 'Close workspace inspector' }).click();
    await expect(composer).toHaveValue('Keep this draft while I review the changed files.');
    const box = await composer.boundingBox();
    expect(box!.y + box!.height).toBeLessThan(size.height - (size.width < 900 ? 56 : 0));
    // The platform popup can extend beyond a renderer capture; its lower options remain keyboard reachable.
    const picker = page.getByLabel('Model', { exact: true });
    await picker.focus();
    await picker.press('End');
    await expect.poll(() => switches.length).toBe(1);
    expect(switches[0]).toMatchObject({ target_model: 'o3' });
    await expect(composer).toHaveValue('Keep this draft while I review the changed files.');
  });
}
