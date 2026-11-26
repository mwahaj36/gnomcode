import { expect, test } from './fixtures';

test('the rail offers Sign in and the account view starts local-first', async ({ page, openConsole }) => {
  await openConsole();
  await page.getByRole('button', { name: 'Sign in to Gnomcode' }).click();
  await expect(page.locator('#console-title')).toHaveText('Account');
  expect(new URL(page.url()).hash).toBe('#auth');
  await expect(page.locator('#auth-cloud-status')).toContainText('Not connected. Local monitoring is ready.');
  await expect(page.locator('#auth-unpaired-card')).toBeVisible();
  await expect(page.locator('#auth-paired-card')).toBeHidden();
  // Settings stays the highlighted place while the account view is open.
  await expect(page.getByRole('tab', { name: 'Settings' })).toHaveAttribute('tabindex', '0');
  await expect(page.getByRole('tab', { selected: true })).toHaveCount(0);
});

test('sign-in asks for account access first and cancelling sends nothing', async ({ page, openConsole }) => {
  await openConsole('#auth');
  const signins: string[] = [];
  page.on('request', (request) => {
    if (request.url().includes('/api/v1/auth/signin')) signins.push(request.url());
  });
  await page.getByLabel('Email').fill('person@example.test');
  await page.getByLabel('Password').fill('not-a-real-password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  const dialog = page.locator('#confirm-dialog');
  await expect(dialog).toBeVisible();
  await expect(dialog.getByRole('heading')).toHaveText('Link and allow account access');
  await expect(dialog.locator('#confirm-message')).toContainText('accessible from any device signed in to your account');
  await dialog.getByRole('button', { name: 'Cancel' }).click();
  await expect(dialog).not.toBeVisible();
  await expect(page.locator('#auth-pair-status')).toBeHidden();
  await expect(page.getByRole('button', { name: 'Sign in', exact: true })).toBeEnabled();
  expect(signins).toEqual([]);
});

test('a confirmed sign-in posts the credentials with the consent flag and shows the connected account', async ({ page, openConsole }) => {
  let paired = false;
  let body: Record<string, unknown> = {};
  await page.route('**/api/v1/auth/signin', async (route) => {
    body = route.request().postDataJSON();
    paired = true;
    await route.fulfill({ json: { ok: true } });
  });
  await page.route('**/api/v1/auth/session', async (route) => {
    if (route.request().method() !== 'GET') return route.fallback();
    await route.fulfill({ json: { paired, email: paired ? 'person@example.test' : null } });
  });
  await openConsole('#auth');
  await page.getByLabel('Email').fill('person@example.test');
  await page.getByLabel('Password').fill('not-a-real-password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.locator('#confirm-accept').click();
  await expect(page.locator('#auth-paired-card')).toBeVisible();
  await expect(page.locator('#auth-paired-email')).toHaveText('person@example.test');
  await expect(page.locator('#auth-cloud-status')).toHaveText('Connected to your account');
  expect(body).toMatchObject({ email: 'person@example.test', remote_access_confirmed: true });
  // Signed in: the rail entry for signing in goes away.
  await expect(page.getByRole('button', { name: 'Sign in to Gnomcode' })).toBeHidden();
});

test('a failed sign-in explains itself and re-enables the form', async ({ page, openConsole }) => {
  await page.route('**/api/v1/auth/signin', (route) => route.fulfill({ status: 401, json: { message: 'no' } }));
  await openConsole('#auth');
  await page.getByLabel('Email').fill('person@example.test');
  await page.getByLabel('Password').fill('wrong');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.locator('#confirm-accept').click();
  await expect(page.locator('#auth-pair-status')).toContainText('Sign-in failed. Check your email, password and connection.');
  await expect(page.getByRole('button', { name: 'Sign in', exact: true })).toBeEnabled();
});

test('the pairing token field is masked until revealed and connects with consent', async ({ page, openConsole }) => {
  let posted: Record<string, unknown> = {};
  let paired = false;
  await page.route('**/api/v1/auth/session', async (route) => {
    const method = route.request().method();
    if (method === 'POST') {
      posted = route.request().postDataJSON();
      paired = true;
      return route.fulfill({ json: { ok: true } });
    }
    if (method === 'DELETE') {
      paired = false;
      return route.fulfill({ json: { ok: true } });
    }
    return route.fulfill({ json: { paired, email: paired ? 'browser@example.test' : null } });
  });
  await openConsole('#auth');
  await page.locator('summary', { hasText: 'Connect an existing browser account' }).click();
  const token = page.locator('#auth-token-input');
  await expect(token).toHaveAttribute('type', 'password');
  await page.getByRole('button', { name: 'Show token' }).click();
  await expect(token).toHaveAttribute('type', 'text');
  await expect(page.getByRole('button', { name: 'Hide token' })).toHaveAttribute('aria-pressed', 'true');
  await token.fill('{"access_token":"abc","token_type":"bearer"}');
  await page.getByRole('button', { name: 'Connect desktop' }).click();
  await page.locator('#confirm-accept').click();
  await expect(page.locator('#auth-paired-card')).toBeVisible();
  expect(posted).toMatchObject({ access_token: 'abc', remote_access_confirmed: true });

  // Disconnect confirms, then returns to the sign-in card.
  await page.getByRole('button', { name: 'Sign out', exact: true }).click();
  await expect(page.locator('#confirm-heading')).toHaveText('Sign out');
  await page.locator('#confirm-accept').click();
  await expect(page.locator('#auth-unpaired-card')).toBeVisible();
});

test('the web console link uses the configured base address and opens safely', async ({ page, openConsole }) => {
  await openConsole('#auth');
  const link = page.getByRole('link', { name: 'Open web console' });
  await expect(link).toHaveAttribute('target', '_blank');
  await expect(link).toHaveAttribute('rel', /noopener/);
  expect(await link.getAttribute('href')).toMatch(/\/console$/);
});

test('a failed disconnect raises a dismissible notice and Escape cancels the confirmation', async ({ page, openConsole }) => {
  await page.route('**/api/v1/auth/session', async (route) => {
    const method = route.request().method();
    if (method === 'DELETE') return route.fulfill({ status: 500, json: { message: 'no' } });
    return route.fulfill({ json: { paired: true, email: 'person@example.test' } });
  });
  await openConsole('#auth');
  const dialog = page.locator('#confirm-dialog');
  await page.getByRole('button', { name: 'Sign out', exact: true }).click();
  await expect(dialog).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(dialog).not.toBeVisible();
  await expect(page.locator('#app-notice')).toBeHidden();

  await page.getByRole('button', { name: 'Sign out', exact: true }).click();
  await page.locator('#confirm-accept').click();
  const notice = page.locator('#app-notice');
  await expect(notice).toBeVisible();
  await expect(notice).toContainText('Could not sign out. Reload the account status and retry.');
  await notice.getByRole('button', { name: 'Dismiss' }).click();
  await expect(notice).toBeHidden();
});

for (const [name, pasted, expected] of [
  ['a bare token', 'abc.def.ghi', { access_token: 'abc.def.ghi', token_type: 'bearer' }],
  ['base64 JSON', Buffer.from('{"access_token":"b64","refresh_token":"r"}').toString('base64'), { access_token: 'b64', refresh_token: 'r' }],
] as const) {
  test(`pairing accepts ${name} and sends the consent flag`, async ({ page, openConsole }) => {
    let posted: Record<string, unknown> = {};
    await page.route('**/api/v1/auth/session', async (route) => {
      if (route.request().method() === 'POST') {
        posted = route.request().postDataJSON();
        return route.fulfill({ json: { ok: true } });
      }
      return route.fulfill({ json: { paired: false } });
    });
    await openConsole('#auth');
    await page.locator('summary', { hasText: 'Connect an existing browser account' }).click();
    await page.locator('#auth-token-input').fill(pasted);
    await page.getByRole('button', { name: 'Connect desktop' }).click();
    await page.locator('#confirm-accept').click();
    await expect(page.locator('#auth-pair-status')).toContainText('Desktop connected.');
    expect(posted).toEqual({ ...expected, remote_access_confirmed: true });
    await expect(page.locator('#auth-token-input')).toHaveValue('');
  });
}

test('a refused pairing says so in red and keeps the token for another try', async ({ page, openConsole }) => {
  await page.route('**/api/v1/auth/session', async (route) => {
    if (route.request().method() === 'POST') return route.fulfill({ status: 400, json: { message: 'bad token' } });
    return route.fulfill({ json: { paired: false } });
  });
  await openConsole('#auth');
  await page.locator('summary', { hasText: 'Connect an existing browser account' }).click();
  await page.locator('#auth-token-input').fill('nope');
  await page.getByRole('button', { name: 'Connect desktop' }).click();
  await page.locator('#confirm-accept').click();
  const status = page.locator('#auth-pair-status');
  await expect(status).toContainText('Failed to save session. Check token and try again.');
  await expect(status).toHaveClass(/sp-tone-red/);
  await expect(page.locator('#auth-token-input')).toHaveValue('nope');
  await expect(page.getByRole('button', { name: 'Connect desktop' })).toBeEnabled();
});

test('sign-in fields fill their column and the button sits under them', async ({ page, openConsole }) => {
  await openConsole('#auth');
  // Measure after fonts and the auth column settle; a first measurement can land mid-layout on slower runners.
  await page.evaluate(() => document.fonts.ready);
  await expect.poll(async () => (await page.getByLabel('Email').boundingBox())?.width ?? 0).toBeGreaterThanOrEqual(300);
  const email = await page.getByLabel('Email').boundingBox();
  const password = await page.getByLabel('Password').boundingBox();
  const button = await page.getByRole('button', { name: 'Sign in', exact: true }).boundingBox();
  expect(email?.width ?? 0).toBeGreaterThanOrEqual(300);
  expect(password?.width ?? 0).toBeGreaterThanOrEqual(300);
  expect((button?.y ?? 0) >= (password?.y ?? 0) + (password?.height ?? 0)).toBe(true);
});
