import { expect, test } from './fixtures';
import { openClaudeConversation, reshapeConversations } from './workspace-helpers';

test.describe('prompt composer', () => {
  test('Enter sends, Shift+Enter adds a line and IME composition never submits', async ({ page, openConsole }) => {
    await reshapeConversations(page, { source: 'managed', hosted: true });
    const sent: Record<string, unknown>[] = [];
    await page.route('**/api/v1/sessions/*/prompt', async (route) => {
      sent.push(route.request().postDataJSON());
      await route.fulfill({ json: { ok: true } });
    });
    await openConsole('#sessions');
    await openClaudeConversation(page);
    const box = page.getByRole('textbox', { name: 'Message the agent' });

    await box.fill('Keyboard prompt');
    await box.press('Shift+Enter');
    await expect(box).toHaveValue('Keyboard prompt\n');

    // An Enter that confirms an IME candidate is not a submit.
    await box.evaluate((element) => {
      element.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', isComposing: true, bubbles: true, cancelable: true }));
    });
    expect(sent).toHaveLength(0);

    await box.fill('Review auth');
    await box.press('Shift+Enter');
    await box.type('Then run tests');
    await box.press('Enter');
    await expect.poll(() => sent.length).toBe(1);
    expect(sent[0]).toEqual({ text: 'Review auth\nThen run tests' });
    await expect(box).toHaveValue('');
  });

  test('a failed send keeps the draft, tells the person and leaves Send usable', async ({ page, openConsole }) => {
    await reshapeConversations(page, { source: 'managed', hosted: true });
    await page.route('**/api/v1/sessions/*/prompt', (route) => route.fulfill({ status: 500, json: { message: 'Could not send.' } }));
    await openConsole('#sessions');
    await openClaudeConversation(page);
    const box = page.getByRole('textbox', { name: 'Message the agent' });
    await box.fill('Keep my draft');
    await page.getByRole('button', { name: 'Send' }).click();
    await expect(page.locator('#local-workspace-root')).toContainText('Could not send.');
    await expect(box).toHaveValue('Keep my draft');
    await expect(page.getByRole('button', { name: 'Send' })).toBeEnabled();
  });
});

test.describe('model handoff', () => {
  test('choosing another model posts one switch and a failure restores the confirmed model', async ({ page, openConsole }) => {
    await reshapeConversations(page, { source: 'managed', hosted: true, model: 'gpt-6.1-sol' });
    await page.route('**/api/v1/model-catalog', (route) =>
      route.fulfill({
        json: {
          providers: [
            { name: 'OpenAI', models: [{ agent: 'codex', model: 'gpt-6.1-sol', display_name: 'gpt-6.1-sol' }, { agent: 'codex', model: 'o3', display_name: 'o3' }] },
          ],
        },
      }),
    );
    const switches: Record<string, unknown>[] = [];
    await page.route('**/api/v1/sessions/*/switch', async (route) => {
      switches.push(route.request().postDataJSON());
      await route.fulfill({ status: 502, json: { message: 'Switch failed.' } });
    });
    await openConsole('#sessions');
    await openClaudeConversation(page);
    const picker = page.getByLabel('Model', { exact: true });
    await page.getByRole('textbox', { name: 'Message the agent' }).fill('Keep my draft');
    const before = await picker.inputValue();
    await picker.selectOption('codex:o3');
    await expect.poll(() => switches.length).toBe(1);
    expect(switches[0]).toEqual({ target_model: 'o3', agent: 'codex' });
    await expect(picker).toHaveValue(before);
    await expect(page.getByRole('textbox', { name: 'Message the agent' })).toHaveValue('Keep my draft');
  });
});

test.describe('launcher', () => {
  test('an agent that is not installed cannot be launched and shows how to install it', async ({ page, openConsole }) => {
    await page.route('**/api/v1/agents/installed', (route) =>
      route.fulfill({
        json: {
          agents: [
            { id: 'codex', name: 'OpenAI Codex', installed: true },
            { id: 'claude', name: 'Claude Code', installed: false, install_command: 'npm install -g @anthropic-ai/claude-code' },
          ],
        },
      }),
    );
    await openConsole('#sessions');
    const launcher = page.locator('#local-workspace-root');
    await launcher.getByRole('button', { name: 'New', exact: true }).click();
    const form = launcher.locator('.ide-launcher');
    await form.getByLabel('Agent').selectOption('claude');
    await expect(launcher.getByRole('button', { name: 'Launch Agent' })).toBeDisabled();
    await expect(form).toContainText('npm install');
    await form.getByLabel('Agent').selectOption('codex');
    await form.getByLabel('Workspace directory').fill('C:\work');
    await expect(launcher.getByRole('button', { name: 'Launch Agent' })).toBeEnabled();
  });

  test('launching posts the agent, folder and model and opens the new conversation', async ({ page, openConsole }) => {
    await page.route('**/api/v1/agents/installed', (route) =>
      route.fulfill({ json: { agents: [{ id: 'codex', name: 'OpenAI Codex', installed: true }] } }),
    );
    const launches: Record<string, unknown>[] = [];
    await page.route('**/api/v1/sessions/launch', async (route) => {
      launches.push(route.request().postDataJSON());
      await route.fulfill({ status: 500, json: { message: 'No terminal in this test.' } });
    });
    await openConsole('#sessions');
    const root = page.locator('#local-workspace-root');
    await root.getByRole('button', { name: 'New', exact: true }).click();
    const form = root.locator('.ide-launcher');
    await form.getByLabel('Workspace directory').fill('C:\\workspace\\picked');
    await form.getByLabel('Model (optional)').fill('o3');
    await root.getByRole('button', { name: 'Launch Agent' }).click();
    await expect.poll(() => launches.length).toBe(1);
    expect(launches[0]).toMatchObject({ agent: 'codex', repo_path: 'C:\\workspace\\picked', model: 'o3' });
    await expect(form).toContainText('Could not launch');
    // The folder survives a failed launch.
    await expect(form.getByLabel('Workspace directory')).toHaveValue('C:\\workspace\\picked');
  });

  test('the folder chooser fills the workspace field and cancelling keeps it', async ({ page, openConsole }) => {
    await page.route('**/api/v1/workspaces', (route) =>
      route.fulfill({ json: { type: 'workspaces', path: 'C:\\workspace', parent: null, roots: [], entries: [{ name: 'picked', path: 'C:\\workspace\\picked' }] } }),
    );
    await openConsole('#sessions');
    const root = page.locator('#local-workspace-root');
    await root.getByRole('button', { name: 'New', exact: true }).click();
    await root.getByLabel('Workspace directory').fill('C:\\workspace\\kept');
    await root.getByRole('button', { name: /Browse|Choose/ }).first().click();
    const dialog = root.locator('dialog[open], dialog.sp-dialog[open], [role="dialog"]').first();
    await expect(dialog).toBeVisible();
    await dialog.getByRole('button', { name: 'Cancel' }).click();
    await expect(root.getByLabel('Workspace directory')).toHaveValue('C:\\workspace\\kept');
  });
});
