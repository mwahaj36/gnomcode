import type { Page } from '@playwright/test';
import { expect, test } from './fixtures';
import { reshapeConversations } from './workspace-helpers';

const PNG = 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==';
const SVG = `data:image/svg+xml;base64,${Buffer.from('<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><script>window.__svgRan=1</script><rect width="10" height="10"/></svg>').toString('base64')}`;
const previews: Record<string, Record<string, unknown>> = {
  'shot.png': { kind: 'image', mime_type: 'image/png', size: 70, data_url: PNG },
  'logo.svg': { kind: 'svg', mime_type: 'image/svg+xml', size: 120, data_url: SVG },
  'README.md': { kind: 'markdown', size: 20, content: '# Title\n\n- item' },
  'data.json': { kind: 'json', size: 10, content: '{"a":[1,2]}' },
  'rows.csv': { kind: 'csv', size: 20, content: 'name,count\n"a,b",3\n' },
  'main.rs': { kind: 'code', language: 'rust', size: 12, content: 'fn main() {}' },
  'doc.pdf': { kind: 'pdf', mime_type: 'application/pdf', size: 8, data_url: 'data:application/pdf;base64,JVBERi0xLjQ=' },
  'blob.bin': { kind: 'binary', size: 4096, data_url: 'data:application/octet-stream;base64,AAAA' },
  'huge.log': { kind: 'text', size: 9_000_000, truncated: true, content: 'first line' },
};

async function openFiles(page: Page, openConsole: (hash: string) => Promise<void>) {
  await reshapeConversations(page, { repo_path: '/work/console-demo' });
  await page.route('**/api/v1/repos/files?*', (route) =>
    route.fulfill({ json: { path: '', entries: Object.entries(previews).map(([name, p]) => ({ name, path: name, kind: 'file', size: p.size })) } }),
  );
  await page.route('**/api/v1/repos/file?*', (route) => {
    const name = new URL(route.request().url()).searchParams.get('path') || '';
    return route.fulfill({ json: { path: name, name, truncated: false, ...previews[name] } });
  });
  await openConsole('#sessions');
  const toggle = page.locator('#chat-list-toggle');
  if (await toggle.isVisible() && (await toggle.getAttribute('aria-expanded')) === 'false') await toggle.click();
  await page.locator('.ide-session', { hasText: 'claude-code' }).first().click();
  const tools = page.getByRole('complementary', { name: 'Conversation tools' });
  // Narrow windows start with the Overview closed; open it from the conversation header.
  if (!(await tools.isVisible())) await page.getByRole('button', { name: 'Tools', exact: true }).click();
  await tools.getByRole('button', { name: 'Files', exact: true }).click();
  return page.locator('.sp-files-panel');
}

for (const [name, viewport] of [['desktop', { width: 1280, height: 900 }], ['phone', { width: 390, height: 844 }]] as const) {
  test.describe(`file previews on ${name}`, () => {
    test.use({ viewport });

    test('each preview type renders safely', async ({ page, openConsole }) => {
      const panel = await openFiles(page, openConsole);
      const open = async (file: string) => panel.getByRole('button', { name: new RegExp(file.replace('.', '\.')) }).click();

      await open('shot.png');
      await expect(panel.locator('.sp-preview-image img')).toHaveAttribute('src', PNG);
      await panel.locator('.sp-preview-image').click();
      await expect(page.locator('dialog.sp-image-viewer img')).toBeVisible();
      await page.locator('dialog.sp-image-viewer').getByRole('button', { name: 'Close' }).click();

      await open('logo.svg');
      await expect(panel.locator('.sp-preview-image img')).toHaveAttribute('src', /^data:image\/svg\+xml/);
      await expect(panel.locator('svg script')).toHaveCount(0);
      expect(await page.evaluate(() => (window as unknown as { __svgRan?: number }).__svgRan)).toBeUndefined();

      await open('README.md');
      await expect(panel.locator('.sp-preview-markdown h1')).toHaveText('Title');
      await open('data.json');
      await expect(panel.locator('.sp-code-lang')).toHaveText('json');
      await expect(panel.locator('.sp-code pre')).toContainText('"a": [');
      await open('rows.csv');
      await expect(panel.locator('.sp-preview-csv td').first()).toHaveText('a,b');
      await open('main.rs');
      await expect(panel.locator('.sp-hl-keyword').first()).toHaveText('fn');
      await open('doc.pdf');
      await expect(panel.getByRole('button', { name: 'Open PDF' })).toBeVisible();
      await open('blob.bin');
      await expect(panel.getByText('4.0 KB')).toBeVisible();
      await expect(panel.getByRole('link', { name: 'Download' })).toHaveAttribute('download', 'blob.bin');
      await open('huge.log');
      await expect(panel.getByText('Preview truncated at 512 KiB')).toBeVisible();
      await expect(panel.getByRole('link', { name: 'Download' })).toHaveCount(0);
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    });

    test('chat attachment references load as image thumbnails', async ({ page, openConsole }) => {
      await page.route('**/api/v1/sessions/*/chat?*', (route) =>
        route.fulfill({ json: [{ event_id: 'a1', turn_index: 1, role: 'user', content: 'Look\n[Attachment: shot.png]', created_at: '2026-11-14T10:00:00Z' }] }),
      );
      await openFiles(page, openConsole);
      await page.getByRole('button', { name: 'Close workspace inspector' }).click();
      const thumb = page.locator('.sp-chat-thumb[data-attachment-path="shot.png"] img');
      await expect(thumb).toHaveAttribute('src', PNG);
      await thumb.click();
      await expect(page.locator('dialog.sp-image-viewer img')).toBeVisible();
    });
  });
}
