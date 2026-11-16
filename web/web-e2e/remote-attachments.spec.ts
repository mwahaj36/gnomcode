import { expect, test } from '@playwright/test';
import { createChannel, generateKeyPair } from '../lib/remote-crypto.mjs';

for (const width of [1440, 390]) {
test.describe(`Remote at ${width}px`, () => {
test.use({viewport:{width,height:900}});
test('Remote attachment exceeds one relay frame, waits for acknowledgement, and sends the saved path', async ({ page }, info) => {
  test.setTimeout(90000);
  const owner = '00000000-0000-0000-0000-000000000001';
  const host = '11111111111111111111111111111111';
  const keys = await generateKeyPair();
  const offer = { owner_id: owner, desktop_id: host, request_id: host, room: host, key: keys.publicHex, relay: 'ws://127.0.0.1:9999/relay', protocol_version: 3 };
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
    return route.fulfill({ json: url.pathname.endsWith('/spawnt_device_requests') ? (url.searchParams.has('id') ? { offer, state: 'approved', revoked: false } : [{ id:host,desktop_id:host,offer,state:'approved',revoked:false }]) : url.pathname.endsWith('/user') ? { id: owner } : [] });
  });
  let uploaded: any;
  let prompt: any;
  let acknowledge: (() => Promise<void>) | undefined;
  const frames: number[] = [];
  const terminalInputs: string[] = [];
  let notificationSettings:any = {approval_waiting:true,turn_finished:true,turn_finished_min_minutes:2,limit_low:true,limit_reset:true,session_stopped:true,needs_attention:true,managed_only:false};
  await page.routeWebSocket('ws://127.0.0.1:9999/**', ws => {
    let channel: any;
    let buffer = '';
    let chain = Promise.resolve();
    const reply = async (value: object) => ws.send(Buffer.from(JSON.stringify(await channel.seal({ type: 'chunk', data: JSON.stringify(value), final: true }))));
    ws.onMessage(data => { chain = chain.then(async () => {
      frames.push(Buffer.byteLength(data));
      expect(Buffer.byteLength(data)).toBeLessThanOrEqual(65536);
      const frame = JSON.parse(data.toString());
      if (frame.type === 'hello') {
        const nonce = 'a'.repeat(32);
        channel = await createChannel(keys, frame.public_key, `${host}:${nonce}`, 'desktop');
        ws.send(Buffer.from(JSON.stringify({ type: 'challenge', nonce }))); return;
      }
      let request = await channel.open(frame);
      if (request.type === 'upload_chunk') {
        buffer += request.data;
        if (!request.final) return;
        request = JSON.parse(buffer); buffer = '';
      }
      if (request.type === 'resume') await reply({ type: 'resumed' });
      else if (request.type === 'sessions') await reply({ type: 'sessions', sessions: [{ id: 'codex:test', agent: 'codex', title: 'Attachment test', state: 'idle', source: 'managed', can_prompt: true, hosted: true }] });
      else if (request.type === 'notification_settings') {if(request.settings)notificationSettings=request.settings;await reply({type:'notification_settings',settings:notificationSettings});}
      else if (request.type === 'terminal_screen') { const data=Buffer.from(Array.from({length:100},(_,i)=>`history line ${i}\r\n`).join('')+'Sign in at https://example.invalid/login\r\nEnter code:'); await reply({type:'terminal_screen',session_id:'codex:test',terminal_data:request.after===undefined?[...data]:[],cursor:data.length,reset:request.after===undefined}); }
      else if (request.type === 'terminal_input') { terminalInputs.push(request.text); await reply({type:'terminal_input_ack',session_id:'codex:test'}); }
      else if (request.type === 'setup') await reply({type:'setup',session_id:'codex:test',blocker:null,limit_message:'Limit reached. Individual quota reached. Resets in 2 hours.'});
      else if (request.type === 'chat') await reply({ type: 'chat', session_id: 'codex:test', events: [] });
      else if (request.type === 'upload_attachment') { uploaded = request; acknowledge = () => reply({ type: 'attachment_ack', request_id: request.request_id, workspace_path: '.gnomcode/attachments/notes.txt' }); }
      else if (request.type === 'prompt') { prompt = request; await reply({ type: 'prompt_ack', id: request.id, session_id: 'codex:test' }); }
      else await reply({ type: request.type, items: [], providers: [] });
    }); });
  });
  await page.goto('/');
  await expect.poll(()=>page.evaluate(()=>!!sessionStorage.getItem('gnomcode-pairing-offer'))).toBeTruthy();
  await page.goto('/console?view=chat');
  const input = page.locator('.remote-composer input[type=file]');
  await expect(input).toBeAttached({ timeout: 30000 });
  await page.getByRole('textbox', { name: 'Message the agent', exact: true }).fill('Read the attachment');
  await input.setInputFiles({ name: 'notes.txt', mimeType: 'text/plain', buffer: Buffer.alloc(128 * 1024, 65) });
  const send = page.locator('.remote-composer button[type=submit]');
  await expect(send).toBeDisabled();
  await expect.poll(() => Boolean(uploaded)).toBeTruthy();
  expect(Buffer.from(uploaded.data_base64, 'base64')).toEqual(Buffer.alloc(128 * 1024, 65));
  await page.getByRole('textbox', { name: 'Message the agent', exact: true }).press('Enter');
  expect(prompt).toBeUndefined();
  await acknowledge!();
  await expect(page.getByRole('region', { name: 'Staged attachments' })).toContainText('Saved');
  await expect(send).toBeEnabled();
  await send.click();
  await expect.poll(() => prompt?.text).toContain('.gnomcode/attachments/notes.txt');
  expect(frames.length).toBeGreaterThan(10);
  await page.getByRole('button', { name: 'Agent terminal', exact: true }).click();
  const terminal = page.getByRole('region', { name: 'Agent terminal', exact: true });
  await expect(terminal.locator('.xterm')).toBeVisible();
  await expect(terminal.locator('.xterm-screen')).toContainText('Enter code:');
  const keyboard = terminal.locator('.xterm-helper-textarea');
  await keyboard.focus(); await page.keyboard.type('one-time-code'); await page.keyboard.press('Enter');
  await expect.poll(()=>terminalInputs.join('')).toContain('one-time-code\r');
  await terminal.getByRole('button',{name:'Escape',exact:true}).click();
  await expect.poll(()=>terminalInputs.join('')).toContain('\u001b');
  await terminal.getByRole('button',{name:'Earlier output',exact:true}).click();
  await expect(terminal.locator('.xterm-screen')).toContainText('history line 0');
  await page.screenshot({path:info.outputPath('terminal.png')});
  expect(await page.evaluate(()=>document.documentElement.scrollWidth-innerWidth)).toBeLessThanOrEqual(1);
  await terminal.getByRole('button',{name:'Close terminal',exact:true}).click();
  await expect(page.getByRole('alert').filter({hasText:'Limit reached'})).toContainText('Resets in 2 hours');
  await page.getByRole('button',{name:'Overview',exact:true}).click();
  await page.getByText('Notification preferences',{exact:true}).click();
  await page.getByLabel('Only Gnomcode-managed conversations',{exact:true}).check();
  await page.getByLabel('Agent errors',{exact:true}).uncheck();
  await page.getByRole('button',{name:'Save notification preferences',exact:true}).click();
  await expect.poll(()=>notificationSettings.managed_only).toBe(true);
  expect(notificationSettings.needs_attention).toBe(false);
  await page.getByRole('button',{name:'Close overview',exact:true}).click();
  await page.clock.install();
  await input.setInputFiles({ name: 'timeout.txt', mimeType: 'text/plain', buffer: Buffer.alloc(128 * 1024, 66) });
  await expect(send).toBeDisabled();
  await page.clock.fastForward(61000);
  await expect(page.getByRole('region', { name: 'Staged attachments' })).toContainText('Upload timed out');
  await expect(send).toBeDisabled();
  await page.getByRole('button', { name: 'Remove timeout.txt', exact: true }).click();
  await input.setInputFiles({ name: 'too-big.txt', mimeType: 'text/plain', buffer: Buffer.alloc(15 * 1024 * 1024 + 1) });
  await expect(page.getByRole('region', { name: 'Staged attachments' })).toContainText(/15/);
});

});
}
