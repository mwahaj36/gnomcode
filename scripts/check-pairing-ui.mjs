// Run against a real daemon and Next.js server. Only external account transport is stubbed.
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { readFile } from 'node:fs/promises';
import assert from 'node:assert/strict';
const require = createRequire(resolve(process.argv[2], 'package.json'));
const { chromium } = require('playwright');
const daemon = JSON.parse(await readFile(process.argv[3], 'utf8'));
const web = process.argv[4] || 'http://127.0.0.1:3010';
const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage();
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.goto(`http://127.0.0.1:${daemon.port}/?token=${daemon.token}`);
  await page.waitForFunction(() => !document.getElementById('sessions-table').textContent.includes('Loading sessions'));
  assert.equal(await page.locator('#connection-status').isVisible(), false);
  await page.getByRole('button', {name: 'Auth & Cloud Sync', exact: true}).click();
  await page.getByRole('button', {name: 'Connect Account', exact: true}).click();
  await page.locator('#browser-pair-code').filter({hasText: /^[A-F0-9]{8}$/}).waitFor();
  assert.match(await page.locator('#browser-pair-link').getAttribute('href'), /^https:\/\/fleethud.vercel.app\/pair#/);
  await page.getByRole('button', {name: 'Cancel', exact: true}).click();
  await page.goto(`http://127.0.0.1:${daemon.port}/`);
  await page.locator('#connection-status').waitFor({state:'visible'});
  assert.match(await page.locator('#connection-status').innerText(), /Authorization expired/);

  const id = 'a'.repeat(32), key = '01'.repeat(32);
  const fragment = new URLSearchParams({id,key,expires:String(Math.floor(Date.now()/1000)+550),name:'Cloud test workspace',project:'https://jjnuxngvhuvxkxrfmnqx.supabase.co'});
  const exp = Math.floor(Date.now()/1000)+3600;
  const payload = Buffer.from(JSON.stringify({sub:'00000000-0000-0000-0000-000000000001',exp,aud:'authenticated',role:'authenticated'})).toString('base64url');
  const header = Buffer.from(JSON.stringify({alg:'HS256'})).toString('base64url');
  const signature = Buffer.from('synthetic-test-signature').toString('base64url');
  const token = [header,payload,signature].join('.');
  const session = {provider_token:'unneeded-github-token',access_token:token,refresh_token:'test-refresh-only',expires_in:3600,expires_at:exp,token_type:'bearer',user:{id:'00000000-0000-0000-0000-000000000001',aud:'authenticated',role:'authenticated',email:'test@example.invalid',app_metadata:{provider:'email'},user_metadata:{},created_at:'2026-01-01T00:00:00Z'}};
  let envelope;
  let challenge;
  await page.route('https://jjnuxngvhuvxkxrfmnqx.supabase.co/**', async route => {
    if (route.request().url().includes('/auth/v1/authorize?')) {
      const url = new URL(route.request().url());
      challenge = url.searchParams.get('code_challenge');
      assert(challenge);
      assert.equal(url.searchParams.get('redirect_to'), `${web}/console`);
      return route.fulfill({status:302,headers:{location:`${web}/console?code=synthetic-code`}});
    }
    if (route.request().url().includes('/auth/v1/token?grant_type=pkce')) {
      const body = route.request().postDataJSON();
      assert.equal(body.auth_code,'synthetic-code');
      const { createHash } = await import('node:crypto');
      assert.equal(createHash('sha256').update(body.code_verifier).digest('base64url'), challenge);
      return route.fulfill({json:session});
    }
    if (route.request().url().includes('/auth/v1/token?grant_type=password')) return route.fulfill({json:session});
    if (route.request().url().endsWith('/rest/v1/rpc/fleethud_pair_approve')) { envelope = route.request().postDataJSON(); return route.fulfill({json:null}); }
    return route.fulfill({status:400,json:{message:'Unexpected test account request'}});
  });
  await page.goto(`${web}/pair#${fragment}`);
  await page.getByLabel('Email', {exact:true}).fill('test@example.invalid');
  await page.getByLabel('Password', {exact:true}).fill('synthetic-test-password');
  await page.getByRole('button',{name:'Sign in with email',exact:true}).click();
  await page.getByLabel('I started this request and the verification code matches.').check();
  await page.getByRole('button',{name:'Connect this workspace or device',exact:true}).click();
  await page.getByRole('heading',{name:'Approval sent',exact:true}).waitFor();
  assert(envelope);
  assert(!JSON.stringify(envelope).includes('test-refresh-only'));
  const aes = await crypto.subtle.importKey('raw', Buffer.from(key,'hex'),'AES-GCM',false,['decrypt']);
  const plain = await crypto.subtle.decrypt({name:'AES-GCM',iv:Buffer.from(envelope.p_iv,'base64'),additionalData:Buffer.from(id)},aes,Buffer.from(envelope.p_ciphertext,'base64'));
  assert.equal(JSON.parse(Buffer.from(plain).toString()).refresh_token,'test-refresh-only');
  assert.equal(JSON.parse(Buffer.from(plain).toString()).provider_token,undefined);
  assert.equal(await page.evaluate(() => sessionStorage.getItem('fleethud_pair_request')),null);
  assert.equal(await page.evaluate(() => sessionStorage.getItem('fleethud-pair-auth')),null);
  assert.equal(await page.evaluate(() => localStorage.getItem('sb-jjnuxngvhuvxkxrfmnqx-auth-token')),null);
  await page.goto(`${web}/`);
  await page.goto(`${web}/pair#${fragment}`);
  await page.getByRole('button',{name:'Continue with GitHub',exact:true}).click();
  await page.getByLabel('I started this request and the verification code matches.').waitFor();
  assert(challenge);
  assert.equal(await page.evaluate(() => localStorage.getItem('sb-jjnuxngvhuvxkxrfmnqx-auth-token')),null);
  await page.getByLabel('I started this request and the verification code matches.').check();
  await page.getByRole('button',{name:'Connect this workspace or device',exact:true}).click();
  await page.getByRole('heading',{name:'Approval sent',exact:true}).waitFor();
  await page.goto(`${web}/`);
  await page.goto(`${web}/pair#${new URLSearchParams({...Object.fromEntries(fragment),expires:'1'})}`);
  await page.getByRole('alert').filter({hasText:/expired/}).waitFor();
  assert.deepEqual(errors,[]);
  console.log('PASS: live console renders, unauthorized errors visible, desktop pairing starts/cancels, email and PKCE browser approval encrypt separate sessions, expired links rejected');
} finally { await browser.close(); }
