import { readFile } from 'node:fs/promises';
import { expect, test } from './fixtures';
import { openClaudeConversation, reshapeConversations } from './workspace-helpers';

test('compact bubbles keep icon-only metadata outside the bubble and reveal it on hover', async ({ page, openConsole }) => {
  await page.route('**/api/v1/sessions/*/chat?*', route => route.fulfill({ json: [{ event_id: 'compact', turn_index: 1, role: 'user', content: 'hi', created_at: '2026-11-16T00:00:00Z' }] }));
  await openConsole('#sessions');
  await openClaudeConversation(page);
  const bubble = page.locator('[data-chat-id="compact"]');
  await expect(bubble).toBeVisible();
  expect((await bubble.locator('.sp-chat-body').boundingBox())!.height).toBeLessThan(60);
  const copy = bubble.getByRole('button', { name: 'Copy your message' });
  await bubble.hover();
  await expect(copy).toHaveCSS('opacity', '1');
  await expect(copy).toHaveCSS('opacity', '1');
  expect((await copy.boundingBox())!.width).toBeGreaterThanOrEqual(32);
  const body = await bubble.locator('.sp-chat-body').boundingBox();
  const footer = await bubble.locator('header').boundingBox();
  expect(footer!.y).toBeGreaterThanOrEqual(body!.y + body!.height);
  await expect(copy.locator('.sp-copy-label')).toBeHidden();
});

test('Files keeps folders visible, downloads exact content, and expands its viewer', async ({ page, openConsole }, testInfo) => {
  await reshapeConversations(page, { repo_path: '/work/demo' });
  await page.route('**/api/v1/repos/files?*', route => {
    const path = new URL(route.request().url()).searchParams.get('path') || '';
    return route.fulfill({ json: { path, entries: path === ''
      ? [{ name: 'src', path: 'src', kind: 'dir' }, { name: 'README.md', path: 'README.md', kind: 'file' }]
      : [{ name: 'main.rs', path: 'src/main.rs', kind: 'file' }] } });
  });
  const content = 'fn main() { println!("hello"); }\n';
  await page.route('**/api/v1/repos/file?*', route => route.fulfill({ json: { path: 'src/main.rs', kind: 'code', language: 'rust', content } }));
  await openConsole('#sessions');
  await openClaudeConversation(page);
  await page.getByRole('complementary', { name: 'Conversation tools' }).getByRole('button', { name: 'Files', exact: true }).click();
  const panel = page.locator('.sp-files-panel');
  await panel.getByRole('button', { name: /^src(?: Folder)?$/ }).click();
  await expect(panel.getByRole('button', { name: /^README\.md(?: File)?$/ })).toBeVisible();
  await panel.getByRole('button', { name: /^main\.rs(?: File)?$/ }).click();
  const download = page.waitForEvent('download');
  await panel.getByRole('link', { name: 'Download', exact: true }).click();
  const file = await download;
  expect(file.suggestedFilename()).toBe('main.rs');
  expect(await readFile((await file.path())!, 'utf8')).toBe(content);
  await panel.getByRole('button', { name: 'Expand preview' }).click();
  const viewer = page.getByRole('dialog', { name: 'File preview' });
  await expect(viewer).toBeVisible();
  expect((await viewer.boundingBox())!.width).toBeGreaterThan(1100);
  await page.screenshot({ path: testInfo.outputPath('file-viewer-desktop.png') });
  await page.keyboard.press('Escape');
  await expect(viewer).toHaveCount(0);
  await expect(panel.getByRole('button', { name: 'Expand preview' })).toBeFocused();
});

test('standing grants stay in Workspace while pending decisions remain global', async ({ page, openConsole }) => {
  await page.route('**/api/v1/approvals', route => route.fulfill({ json: {
    items: [{ id: 'pending', agent: 'codex', tool: 'Edit' }],
    grants: [{ agent: 'antigravity', session_id: 'held-session', tool: null }],
  } }));
  await openConsole();
  await expect(page.locator('[data-approval-id="pending"]')).toBeVisible();
  await expect(page.locator('[data-grant-session]')).toHaveCount(0);
  await page.getByRole('tab', { name: 'Workspace', exact: true }).click();
  await expect(page.locator('[data-grant-session="held-session"]')).toBeVisible();
  await expect(page.getByRole('button', { name: /Stop allowing/ })).toBeVisible();
});



test('a linked handoff opens its document directly in Artifacts', async ({ page, openConsole }) => {
  await reshapeConversations(page, { repo_path: '/work/demo' });
  await page.route('**/api/v1/sessions/*/chat?*', route => route.fulfill({ json: [{ event_id: 'handoff', turn_index: 1, role: 'assistant', content: '[Handoff note](localdocs/handoff.md)', created_at: '2026-11-16T00:00:00Z' }] }));
  await page.route('**/api/v1/sessions/*/artifacts', route => route.fulfill({ json: { artifacts: [
    { id: 'other', path: 'other.md', kind: 'markdown', content: '# Other document' },
    { id: 'handoff', path: 'localdocs/handoff.md', kind: 'markdown', content: '# Handoff document', status: 'referenced' },
  ], changes_count: 0 } }));
  await openConsole('#sessions');
  await openClaudeConversation(page);
  await page.getByRole('link', { name: 'Handoff note', exact: true }).click();
  await expect(page.locator('.sp-artifact-preview')).toContainText('Handoff document');
  await expect(page.locator('.sp-artifact-preview').getByRole('link', { name: 'Download', exact: true })).toBeVisible();
});
