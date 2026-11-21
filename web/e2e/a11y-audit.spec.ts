import { expect, test } from './fixtures';
import { openClaudeConversation, reshapeConversations } from './workspace-helpers';

test.describe('Accessibility and layout audit across all local console views', () => {
  for (const theme of ['dark', 'light'] as const) {
    test(`all views pass accessible names, WCAG contrast, and overflow checks in ${theme} mode`, async ({ page, openConsole }) => {
      await page.addInitScript((mode) => {
        localStorage.setItem('gnomcode-theme', mode);
      }, theme);
      await reshapeConversations(page, { repo_path: '/work/console-demo' });
      await page.route('**/api/v1/repos/files?*', (route) =>
        route.fulfill({ json: { path: '', entries: [{ name: 'README.md', path: 'README.md', kind: 'file', size: 256 }] } }),
      );

      for (const hash of ['#overview', '#usage', '#sessions', '#settings', '#auth']) {
        await openConsole(hash);
        if (hash === '#sessions') {
          await openClaudeConversation(page);
    await page.getByRole('button', { name: 'Overview', exact: true }).click();
          const tools = page.getByRole('complementary', { name: 'Conversation tools' });
          await tools.getByRole('button', { name: 'Files' }).click();
        }

        const issues = await page.evaluate(() => {
          const problems: string[] = [];
          const visible = (el: Element): boolean => {
            const style = getComputedStyle(el);
            if (style.display === 'none' || style.visibility === 'hidden' || style.opacity === '0') return false;
            if (el.closest('[hidden]')) return false;
            const rect = el.getBoundingClientRect();
            return rect.width > 0 && rect.height > 0;
          };

          const parseRgb = (color: string): [number, number, number, number] | null => {
            const m = color.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)(?:,\s*([\d.]+))?\)/);
            if (!m) return null;
            return [Number(m[1]), Number(m[2]), Number(m[3]), m[4] !== undefined ? Number(m[4]) : 1];
          };

          const luminance = (r: number, g: number, b: number): number => {
            const toLin = (c: number) => {
              const s = c / 255;
              return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
            };
            return 0.2126 * toLin(r) + 0.7152 * toLin(g) + 0.0722 * toLin(b);
          };

          const contrast = (fg: [number, number, number], bg: [number, number, number]): number => {
            const l1 = luminance(...fg);
            const l2 = luminance(...bg);
            return (Math.max(l1, l2) + 0.05) / (Math.min(l1, l2) + 0.05);
          };

          const effectiveBg = (el: Element): [number, number, number] => {
            let cur: Element | null = el;
            while (cur) {
              const parsed = parseRgb(getComputedStyle(cur).backgroundColor);
              if (parsed && parsed[3] >= 0.9) return [parsed[0], parsed[1], parsed[2]];
              cur = cur.parentElement;
            }
            const bodyBg = parseRgb(getComputedStyle(document.body).backgroundColor);
            return bodyBg ? [bodyBg[0], bodyBg[1], bodyBg[2]] : [0, 0, 0];
          };

          // 1. Accessible names for interactive controls
          for (const el of document.querySelectorAll('button, a[href], input:not([type="hidden"]), select, textarea')) {
            if (!visible(el)) continue;
            const ariaLabel = el.getAttribute('aria-label')?.trim();
            const labelledBy = el.getAttribute('aria-labelledby')?.trim();
            const title = el.getAttribute('title')?.trim();
            const text = el.textContent?.trim();
            const id = el.getAttribute('id');
            const hasForLabel = id ? Boolean(document.querySelector(`label[for="${CSS.escape(id)}"]`)) : false;
            const insideLabel = Boolean(el.closest('label'));
            if (!ariaLabel && !labelledBy && !text && !hasForLabel && !insideLabel && !title) {
              problems.push(`Control missing accessible name: <${el.tagName.toLowerCase()} class="${el.className}">`);
            }
          }

          // 2. Contrast check on primary CTA and interactive buttons
          for (const btn of document.querySelectorAll('.btn-primary, .btn-control, .tab-btn.active')) {
            if (!visible(btn)) continue;
            const style = getComputedStyle(btn);
            const fg = parseRgb(style.color);
            const bg = effectiveBg(btn);
            if (fg && fg[3] >= 0.9) {
              const ratio = contrast([fg[0], fg[1], fg[2]], bg);
              if (ratio < 4.5) {
                problems.push(`Low contrast (${ratio.toFixed(2)}:1) on <${btn.tagName.toLowerCase()} class="${btn.className}"> "${btn.textContent?.trim()}"`);
              }
            }
          }

          // 3. Horizontal overflow check
          if (document.documentElement.scrollWidth > window.innerWidth) {
            problems.push(`Horizontal overflow: scrollWidth=${document.documentElement.scrollWidth} > innerWidth=${window.innerWidth}`);
          }

          return problems;
        });

        expect(issues, `Accessibility/contrast/layout issues on ${hash} (${theme})`).toEqual([]);
      }
    });
  }

  test('mobile inspector, Command Palette edge cases, and Terminal tab stay clean and responsive', async ({ page, openConsole }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await reshapeConversations(page, { repo_path: '/work/console-demo' });
    await openConsole('#sessions');
    await page.getByRole('button', { name: 'Toggle conversations' }).click();
    await openClaudeConversation(page);
    await page.getByRole('button', { name: 'Overview', exact: true }).click();

    // Open inspector on mobile via Actions menu -> Terminal
    await page.locator('details.conversation-menu > summary').click();
    await page.locator('details.conversation-menu').getByRole('button', { name: 'Terminal', exact: true }).click();

    const inspector = page.locator('aside.sp-inspector');
    await expect(inspector).toBeVisible();
    const tablist = inspector.getByRole('tablist', { name: 'Inspector views' });
    const tabs = tablist.getByRole('tab');
    const tabCount = await tabs.count();
    for (let i = 0; i < tabCount; i++) {
      const box = await tabs.nth(i).boundingBox();
      expect(box).not.toBeNull();
      expect(box!.height).toBeLessThanOrEqual(44);
    }
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    expect(overflow).toBeLessThanOrEqual(0);
    await inspector.getByRole('button', { name: 'Close workspace inspector' }).click();
    await expect(inspector).toHaveCount(0);

    // Command Palette keyboard flow: empty-query guard + navigation
    await page.keyboard.press('Control+k');
    const palette = page.locator('#command-palette');
    await expect(palette).toBeVisible();
    const input = palette.getByRole('combobox', { name: 'Search commands and settings' });
    await input.fill('zzzzzz-no-match');
    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('ArrowUp');
    await page.keyboard.press('Enter');
    await expect(palette).toBeVisible();
    await input.fill('Go to Usage');
    await page.keyboard.press('Enter');
    await expect(palette).toBeHidden();
    await expect(page.locator('#local-usage-root')).toBeVisible();
  });
});
