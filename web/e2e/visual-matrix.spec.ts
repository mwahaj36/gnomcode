import { expect, test } from './fixtures';
import { openClaudeConversation, reshapeConversations } from './workspace-helpers';

for (const size of [{ width: 1440, height: 900 }, { width: 1024, height: 768 }, { width: 768, height: 1024 }, { width: 390, height: 844 }, { width: 375, height: 667 }, { width: 844, height: 390 }]) {
  for (const theme of ['dark', 'light']) {
    test(`visual audit ${size.width}x${size.height} ${theme}`, async ({ page, openConsole }, info) => {
      test.setTimeout(90_000);
      await page.setViewportSize(size);
      await page.addInitScript(mode => localStorage.setItem('gnomcode-theme', mode), theme);
      await reshapeConversations(page, { repo_path: '/work/demo' });
      await page.route('**/api/v1/repos/files?*', route => route.fulfill({ json: { path: '', entries: [{ name: 'README.md', path: 'README.md', kind: 'file' }] } }));
      await page.route('**/api/v1/repos/file?*', route => route.fulfill({ json: { path: 'README.md', kind: 'markdown', content: '# Workspace preview\n\nA readable document with enough room to inspect its contents.\n\n- Browse folders\n- Download the file\n- Expand the preview' } }));
      await page.route('**/api/v1/sessions/*/changes', route => route.fulfill({ json: { status: ' M README.md', diff: 'diff --git a/README.md b/README.md\n--- a/README.md\n+++ b/README.md\n@@ -1 +1 @@\n-old\n+new' } }));
      await page.route('**/api/v1/sessions/*/artifacts', route => route.fulfill({ json: { artifacts: [{ id: 'preview', path: 'preview.html', kind: 'html', content: '<h1>Artifact preview</h1><p>Sandboxed document content.</p>' }], changes_count: 1 } }));
      for (const [name, hash] of [['home', ''], ['usage', '#usage'], ['settings', '#settings'], ['account', '#auth'], ['workspace', '#sessions']]) {
        await openConsole(hash);
        if (name === 'workspace') {
          const toggle = page.locator('#chat-list-toggle');
          if (await toggle.isVisible() && await toggle.getAttribute('aria-expanded') === 'false') await toggle.click();
          await openClaudeConversation(page);
        }
        await page.waitForTimeout(300);
        await page.screenshot({ path: info.outputPath(`${name}.png`) });
        expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth), name).toBeLessThanOrEqual(1);
      }
      for (const name of ['Files', 'Changes', 'Artifacts']) {
        const tools = page.getByRole('complementary', { name: 'Conversation tools' });
        if (!await tools.isVisible()) await page.getByRole('button', { name: 'Overview', exact: true }).click();
        await tools.getByRole('button', { name, exact: true }).click();
        const panel = page.locator(name === 'Files' ? '.sp-files-panel' : name === 'Changes' ? '.sp-changes-panel' : '.sp-artifacts-panel');
        await expect(panel).toBeVisible();
        if (name === 'Files') {
          await panel.getByRole('button', { name: 'README.md', exact: true }).click();
          await expect(panel.locator('.sp-preview-markdown')).toContainText('Workspace preview');
        }
        await page.screenshot({ path: info.outputPath(`${name.toLowerCase()}.png`) });
        const close = page.getByRole('button', { name: 'Close workspace inspector', exact: true });
        await close.click();
      }
    });
  }
}
