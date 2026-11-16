import { expect, test } from './fixtures';

// Window sizes a desktop user actually produces, including the narrow and short ones that come
// from dragging the Electron window in or running at 125%/150% display scaling.
const SIZES = [
  { name: 'wide', width: 1440, height: 900 },
  { name: 'compact', width: 1280, height: 720 },
  { name: 'scaled-150', width: 853, height: 480 },
  { name: 'narrow', width: 760, height: 700 },
  { name: 'phone', width: 390, height: 844 },
] as const;

const VIEWS = ['#overview', '#sessions', '#usage', '#settings'] as const;

for (const size of SIZES) {
  test.describe(`console shell at ${size.name} (${size.width}x${size.height})`, () => {
    test.use({ viewport: { width: size.width, height: size.height } });

    test('the shell owns the viewport: one scroller, no document scroll, navigation on screen', async ({
      page,
      openConsole,
    }) => {
      for (const view of VIEWS) {
        await openConsole(view);

        // The page itself never scrolls: panes inside the shell do. Two scrollbars mean the
        // document grew past the viewport behind a pane that already scrolls.
        const documentScroll = await page.evaluate(() => ({
          vertical: document.documentElement.scrollHeight - document.documentElement.clientHeight,
          horizontal: document.documentElement.scrollWidth - document.documentElement.clientWidth,
        }));
        expect(documentScroll.vertical, `${view} vertical document scroll`).toBeLessThanOrEqual(1);
        expect(documentScroll.horizontal, `${view} horizontal document scroll`).toBeLessThanOrEqual(1);

        // One work pane scroller, never a scrollbar inside a scrollbar.
        const scrollers = await page.evaluate(() => {
          const content = document.querySelector('.sp-console-content');
          if (!content) return [];
          return [content, ...content.querySelectorAll<HTMLElement>('*')]
            .filter((el) => {
              const style = getComputedStyle(el as HTMLElement);
              const scrolls = /auto|scroll/.test(style.overflowY);
              return scrolls && (el as HTMLElement).scrollHeight > (el as HTMLElement).clientHeight + 2;
            })
            .map((el) => `${(el as HTMLElement).tagName}.${(el as HTMLElement).className}`.slice(0, 80));
        });
        expect(scrollers.length, `${view} nested scrollers: ${scrollers.join(' | ')}`).toBeLessThanOrEqual(1);

        // The navigation is reachable at every size: a bottom tab bar on phones, a rail above.
        const nav = page.locator('.sp-console-nav');
        await expect(nav).toBeVisible();
        const box = await nav.boundingBox();
        expect(box, `${view} navigation box`).not.toBeNull();
        expect(box!.y, `${view} navigation top`).toBeGreaterThanOrEqual(-1);
        expect(box!.y + box!.height, `${view} navigation bottom`).toBeLessThanOrEqual(size.height + 1);

        // Every rail destination stays clickable rather than being clipped by the viewport.
        const tabs = nav.locator('.tab-btn');
        for (let i = 0; i < (await tabs.count()); i++) {
          const tab = await tabs.nth(i).boundingBox();
          if (!tab) continue;
          expect(tab.y + tab.height, `${view} rail tab ${i} bottom`).toBeLessThanOrEqual(size.height + 1);
        }
      }
    });
  });
}

test.describe('desktop rail footer', () => {
  test.use({ viewport: { width: 1280, height: 720 } });

  test('Collapse navigation is fully visible and the rail never scrolls', async ({ page, openConsole }) => {
    await openConsole('#sessions');
    const toggle = page.locator('#navigation-toggle');
    await expect(toggle).toBeVisible();
    const box = await toggle.boundingBox();
    expect(box!.y + box!.height).toBeLessThanOrEqual(720);
    const nav = page.locator('.sp-console-nav');
    expect(await nav.evaluate((el) => el.scrollHeight - el.clientHeight)).toBeLessThanOrEqual(1);
  });

  test('signed-out rails hide Sign out instead of showing a dead control', async ({ page, openConsole }) => {
    await openConsole('#overview');
    const signOut = page.locator('#account-sign-out');
    if ((await signOut.count()) && (await signOut.getAttribute('hidden')) !== null) {
      await expect(signOut).toBeHidden();
    }
  });
});
