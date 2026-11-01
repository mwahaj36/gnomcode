import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const {chromium}=createRequire(import.meta.url)('playwright');
const site=process.argv[2];
if (!site) throw Error('Usage: node scripts/check-account-ux.mjs <site>');
const browser=await chromium.launch(process.env.SPAWNT_TEST_BROWSER ? {executablePath:process.env.SPAWNT_TEST_BROWSER} : {});
try {
  const page=await browser.newPage({viewport:{width:390,height:844}});
  await page.goto(`${site}/auth`);
  const password=page.locator('#auth-password');
  await password.fill('example-password');
  await page.getByRole('button',{name:'Show password'}).click();
  assert.equal(await password.getAttribute('type'),'text');
  await page.getByRole('button',{name:'Hide password'}).click();
  assert.equal(await password.getAttribute('type'),'password');
  await page.getByRole('button',{name:'Show password'}).click();
  await page.getByRole('button',{name:'Create account',exact:true}).click();
  assert.equal(await password.getAttribute('type'),'password','changing account task hides the password');
  assert.equal(await password.getAttribute('autocomplete'),'new-password');
  await page.getByRole('button',{name:'Reset password',exact:true}).click();
  assert.equal(await page.getByRole('heading',{name:'Reset password'}).isVisible(),true);
  assert.equal(await password.count(),0);
  await page.close();

  const context=await browser.newContext({viewport:{width:390,height:844}});
  await context.addInitScript(()=>localStorage.setItem('sb-jjnuxngvhuvxkxrfmnqx-auth-token',JSON.stringify({access_token:'test-token',refresh_token:'test-refresh',expires_at:Math.floor(Date.now()/1000)+3600,expires_in:3600,token_type:'bearer',user:{id:'00000000-0000-4000-8000-000000000001',aud:'authenticated',role:'authenticated',email:'test@example.invalid'}})));
  await context.route('https://*.supabase.co/**',route=>{
    const path=new URL(route.request().url()).pathname;
    if(path.endsWith('/logout')) return route.fulfill({status:400,contentType:'application/json',body:JSON.stringify({message:'Unavailable'})});
    if(path.includes('/auth/v1/')) return route.fulfill({contentType:'application/json',body:JSON.stringify({id:'00000000-0000-4000-8000-000000000001',aud:'authenticated',role:'authenticated',email:'test@example.invalid'})});
    return route.fulfill({contentType:'application/json',body:JSON.stringify({display_name:'Test',openai_plan_usd:20,claude_plan_usd:20,google_plan_usd:0,custom_pricing:[],live_snapshot:null})});
  });
  const profile=await context.newPage();
  await profile.goto(`${site}/console?view=settings`);
  const newPassword=profile.getByLabel('New password (at least 6 characters)');
  await newPassword.fill('example-password');
  await profile.getByRole('button',{name:'Show password'}).click();
  assert.equal(await newPassword.getAttribute('type'),'text');
  await profile.getByRole('button',{name:'Hide password'}).click();
  assert.equal(await newPassword.getAttribute('type'),'password');
  await profile.getByRole('button',{name:'Sign out'}).click();
  await profile.getByRole('status').filter({hasText:'Could not sign out all sessions'}).waitFor();
  assert.equal(await profile.getByText('Not signed in',{exact:true}).isVisible(),true,'failed remote revocation still clears the local session');
  await context.close();
  console.log('PASS: account visibility, task changes and sign-out failure feedback');
} finally { await browser.close(); }
