import { expect, test } from '@playwright/test';

test('removing an old desktop confirms first and deletes only the selected account row', async ({ page }) => {
  const owner = '00000000-0000-0000-0000-000000000001';
  const host = { id: '11111111111111111111111111111111', name: 'Old desktop', last_seen_at: '2026-01-01T00:00:00Z', remote_enabled: false };
  await page.addInitScript(({ owner }) => localStorage.setItem('sb-jjnuxngvhuvxkxrfmnqx-auth-token', JSON.stringify({ access_token: 'test-token', refresh_token: 'test-refresh', expires_at: Math.floor(Date.now() / 1000) + 3600, token_type: 'bearer', user: { id: owner, aud: 'authenticated', email: 'test@example.invalid' } })), { owner });
  const deletions: string[] = [];
  let removed = false;
  await page.route('https://*.supabase.co/**', route => {
    const request = route.request();
    const url = new URL(request.url());
    if (request.method() === 'DELETE') { deletions.push(request.url()); removed = true; return route.fulfill({ json: [host] }); }
    return route.fulfill({ json: url.pathname.endsWith('/spawnt_desktops') ? (removed ? [] : [host]) : url.pathname.endsWith('/user') ? { id: owner } : [] });
  });
  await page.goto('/console?view=settings');
  const card = page.getByRole('article', { name: 'Old desktop', exact: true });
  await expect(card).toBeVisible();
  await card.getByRole('button', { name: 'Remove desktop', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Remove desktop?', exact: true });
  await expect(dialog).toContainText('Old desktop');
  expect(deletions).toHaveLength(0);
  await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(card).toBeVisible();
  await card.getByRole('button', { name: 'Remove desktop', exact: true }).click();
  await dialog.getByRole('button', { name: 'Remove desktop', exact: true }).click();
  await expect(card).toHaveCount(0);
  expect(deletions).toHaveLength(1);
  expect(new URL(deletions[0]).searchParams.get('user_id')).toBe(`eq.${owner}`);
  expect(new URL(deletions[0]).searchParams.get('id')).toBe(`eq.${host.id}`);
});
