import { expect, test } from '@playwright/test';
import { createChannel, generateKeyPair } from '../lib/remote-crypto.mjs';

// A phone that opened Remote while the desktop was asleep used to give up after 30 s and never
// retry, even after the desktop came back or the page became visible again.
test('Remote keeps retrying after the desktop stops answering and reconnects once it wakes', async ({ page }) => {
  test.setTimeout(90000);
  const owner = '00000000-0000-0000-0000-000000000001';
  const host = '11111111111111111111111111111111';
  const keys = await generateKeyPair();
  const offer = { owner_id: owner, desktop_id: host, request_id: host, room: host, key: keys.publicHex, relay: 'ws://127.0.0.1:9999/relay', protocol_version: 3 };
  await page.clock.install();
  await page.addInitScript(async ({ owner, host, offer }) => {
    localStorage.setItem('sb-jjnuxngvhuvxkxrfmnqx-auth-token', JSON.stringify({ access_token: 'test-token', refresh_token: 'test-refresh', expires_at: Math.floor(Date.now() / 1000) + 3600, token_type: 'bearer', user: { id: owner, aud: 'authenticated', email: 'test@example.invalid' } }));
    const proofKey = await crypto.subtle.importKey('raw', new Uint8Array(32).fill(1), { name: 'HMAC', hash: 'SHA-256' }, false, ['sign']);
    await new Promise<void>((resolve, reject) => {
      const request = indexedDB.open('gnomcode-remote-device', 1);
      request.onupgradeneeded = () => request.result.createObjectStore('devices');
      request.onerror = () => reject(request.error);
      request.onsuccess = () => {
        const tx = request.result.transaction('devices', 'readwrite');
        tx.objectStore('devices').put({ offer, proofKey }, `${owner}:${host}`);
        tx.oncomplete = () => { request.result.close(); sessionStorage.setItem('gnomcode-pairing-offer', JSON.stringify(offer)); resolve(); };
      };
    });
  }, { owner, host, offer });
  await page.route('https://*.supabase.co/**', route => {
    const url = new URL(route.request().url());
    return route.fulfill({ json: url.pathname.endsWith('/spawnt_device_requests') ? (url.searchParams.has('id') ? { offer, state: 'approved', revoked: false } : [{ id: host, desktop_id: host, offer, state: 'approved', revoked: false }]) : url.pathname.endsWith('/user') ? { id: owner } : [] });
  });
  let awake = false;
  let helloSeen = false;
  await page.routeWebSocket('ws://127.0.0.1:9999/**', ws => {
    let channel: any;
    const reply = async (value: object) => ws.send(Buffer.from(JSON.stringify(await channel.seal({ type: 'chunk', data: JSON.stringify(value), final: true }))));
    let chain = Promise.resolve();
    ws.onMessage(data => { chain = chain.then(async () => {
      const frame = JSON.parse(data.toString());
      if (frame.type === 'hello') {
        helloSeen = true;
        if (!awake || channel) return; // Silent desktop; once awake, answer the first hello only.
        const nonce = 'a'.repeat(32);
        channel = await createChannel(keys, frame.public_key, `${host}:${nonce}`, 'desktop');
        ws.send(Buffer.from(JSON.stringify({ type: 'challenge', nonce })));
        return;
      }
      const request = await channel.open(frame);
      if (request.type === 'resume') await reply({ type: 'resumed' });
      else if (request.type === 'sessions') await reply({ type: 'sessions', sessions: [{ id: 'codex:test', agent: 'codex', title: 'Woke up', state: 'idle', source: 'managed', can_prompt: true, hosted: true }] });
      else await reply({ type: request.type, items: [], providers: [], events: [] });
    }); });
  });
  await page.goto('/');
  await expect.poll(() => page.evaluate(() => !!sessionStorage.getItem('gnomcode-pairing-offer'))).toBeTruthy();
  await page.goto('/console?view=chat');
  // Start the fake clock only once the browser has said hello; advancing it before the connection timer exists races the page.
  await expect.poll(() => helloSeen, { timeout: 15000 }).toBe(true);
  // Past the 30 s window with a silent desktop: the phone reports it, but must not stop there.
  // Remote alternates between unavailable and a quiet retry every 15 s, so step the clock until the outage is reported.
  const unavailable = page.getByRole('status').filter({ hasText: 'Desktop unavailable' }).first();
  let reported = false;
  for (let i = 0; i < 16 && !reported; i++) {
    await page.clock.runFor(5000);
    reported = await unavailable.isVisible();
  }
  expect(reported, 'a silent desktop is reported as unavailable').toBe(true);
  awake = true;
  for (let i = 0; i < 8; i++) await page.clock.runFor(5000);
  await expect(page.getByText('Woke up').first()).toBeVisible({ timeout: 15000 });
});
