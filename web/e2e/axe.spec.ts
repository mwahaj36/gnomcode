import AxeBuilder from '@axe-core/playwright';
import { expect, test } from './fixtures';
import { openClaudeConversation, reshapeConversations } from './workspace-helpers';

// WCAG 2.2 AA through axe-core: every console view, both themes, desktop and phone. Serious and
// critical findings fail; the report names the rule and the first offending nodes.
for (const size of [{ width: 1440, height: 900 }, { width: 390, height: 844 }]) {
  for (const theme of ['dark', 'light'] as const) {
    test(`axe finds no serious issues at ${size.width}px in ${theme} mode`, async ({ page, openConsole }) => {
      test.setTimeout(120_000);
      await page.setViewportSize(size);
      await page.addInitScript((mode) => localStorage.setItem('gnomcode-theme', mode), theme);
      await reshapeConversations(page, { repo_path: '/work/console-demo' });
      for (const hash of ['', '#usage', '#settings', '#auth', '#sessions']) {
        await openConsole(hash);
        if (hash === '#sessions') {
          const toggle = page.locator('#chat-list-toggle');
          if ((await toggle.isVisible()) && (await toggle.getAttribute('aria-expanded')) === 'false') await toggle.click();
          await openClaudeConversation(page);
        }
        await page.waitForTimeout(300);
        const { violations } = await new AxeBuilder({ page })
          .withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa'])
          .exclude('.xterm')
          .analyze();
        const serious = violations
          .filter((v) => v.impact === 'serious' || v.impact === 'critical')
          .map((v) => `${v.id}: ${v.nodes.slice(0, 3).map((n) => n.target.join(' ')).join(' | ')}`);
        expect(serious, `view ${hash || '#overview'}`).toEqual([]);
      }
    });
  }
}
