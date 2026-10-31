import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
const { chromium } = createRequire(import.meta.url)('playwright');
const site = process.argv[2];
if (!site) throw Error('Usage: check-mobile-console.mjs <local-site>');
const browser = await chromium.launch({executablePath:process.env.SPAWNT_TEST_BROWSER,headless:true});
const failures=[];
try {
  const page=await browser.newPage({viewport:{width:390,height:844},isMobile:true,hasTouch:true});
  await page.goto(`${site}/remote`);
  await page.getByRole('heading',{name:'Connect your desktop'}).waitFor();
  const rail=page.getByRole('navigation',{name:'Console views'});
  try { assert.equal(await rail.isVisible(),true,'Remote pairing must retain the console rail'); }
  catch(error){failures.push(error.message);}
  await page.close();

  const context=await browser.newContext({viewport:{width:390,height:844},isMobile:true,hasTouch:true});
  await context.addInitScript(()=>{
    localStorage.setItem('sb-jjnuxngvhuvxkxrfmnqx-auth-token',JSON.stringify({access_token:'test-token',refresh_token:'test-refresh',expires_at:Math.floor(Date.now()/1000)+3600,token_type:'bearer',user:{id:'00000000-0000-0000-0000-000000000001',aud:'authenticated',email:'test@example.invalid'}}));
  });
  let snapshot={generated_at:new Date().toISOString(),sessions:[{id:'one',project_name:'Fleet HUD <test>',agent:'codex',state:'idle',is_active:false,last_event_at:new Date().toISOString()}],timelines:{},quota:[]};
  await context.route('https://*.supabase.co/**',route=>{
    const url=new URL(route.request().url());
    return route.fulfill({contentType:'application/json',body:JSON.stringify(url.pathname.includes('spawnt_sync_state')?{live_snapshot:snapshot}:[])});
  });
  const projects=await context.newPage();
  await projects.goto(`${site}/console?view=projects`);
  await projects.getByRole('heading',{name:'Project activity'}).waitFor();
  try {
    const project=projects.getByRole('button',{name:'Fleet HUD <test>',exact:true});
    await project.waitFor({timeout:5000});
    await project.click();
    assert.equal(new URL(projects.url()).searchParams.get('view'),'sessions');
    assert.equal(await projects.getByRole('combobox',{name:'Project',exact:true}).inputValue(),'Fleet HUD <test>');
  } catch(error){failures.push(`Synced project must open its filtered session history: ${error.message}`);}
  snapshot=null;
  await projects.goto(`${site}/console?view=projects`);
  try { await projects.getByRole('button',{name:'Connect your desktop',exact:true}).waitFor({timeout:5000}); }
  catch(error){failures.push(`Missing snapshot must offer a connection action: ${error.message}`);}
  await context.close();
  assert.deepEqual(failures,[]);
  console.log('PASS: mobile pairing retains navigation; synced projects open filtered sessions; missing data offers connection');
} finally { await browser.close(); }
