import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { mkdir, readFile } from 'node:fs/promises';
const { chromium } = createRequire(import.meta.url)('playwright');
const browser = await chromium.launch({headless:true, executablePath:process.env.SPAWNT_TEST_BROWSER});
const base = process.argv[2] || 'http://127.0.0.1:43929';
await mkdir('.tmp/orchestration-hero', {recursive:true});
try {
  const page = await browser.newPage({reducedMotion:'reduce'});
  for (const width of [390, 768, 1440]) for (const theme of ['dark', 'light']) {
    await page.setViewportSize({width,height:1000});
    await page.goto(base, {waitUntil:'networkidle'});
    await page.evaluate(theme=>{document.documentElement.dataset.theme=theme;localStorage.setItem('spawnt-theme',theme);},theme);
    assert.equal(await page.locator('#content > section#agents').count(),1,'agent strip is its own section');
    const hero=page.locator('.sp-hero');
    assert.equal(await hero.locator('table,.sp-summary-grid,.sp-tour-overview').count(),0,'hero shows orchestration, not reporting');
    assert.ok(await hero.getByText('Context carried forward', {exact:true}).isVisible());
    assert.ok(await hero.getByText('From your phone', {exact:true}).isVisible());
    assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),'page fits viewport');
    if (width === 390) {
      assert.ok(await page.locator('.sp-nav-right .sp-nav-login').isVisible(), 'phone header exposes account access');
      assert.equal(await page.locator('.sp-nav-right .sp-nav-cta').isVisible(), false, 'phone header hides Download');
    }
    const bounds=await page.locator('.sp-workflow-visual').boundingBox();
    for(const item of await page.locator('.sp-workflow-desktop,.sp-workflow-phone').all()) {
      const box=await item.boundingBox();
      assert.ok(box.x>=bounds.x-1 && box.x+box.width<=bounds.x+bounds.width+1,'workflow panes fit their region');
    }
    await page.screenshot({path:`.tmp/orchestration-hero/${width}-${theme}.png`});
  }
  await page.setViewportSize({width:390,height:900});
  await page.goto(base+'/docs',{waitUntil:'networkidle'});
  assert.ok(await page.locator('.sp-site-actions .sp-site-login').isVisible(), 'shared phone header exposes account access');
  assert.equal(await page.locator('.sp-site-actions .sp-site-download').isVisible(),false);
  await page.emulateMedia({reducedMotion:'no-preference'});
  await page.goto(base,{waitUntil:'networkidle'});
  const track=page.locator('.sp-marquee-track');
  const first=await track.evaluate(node=>getComputedStyle(node).transform);
  await page.waitForFunction(first=>getComputedStyle(document.querySelector('.sp-marquee-track')).transform!==first,first);
  await page.locator('.sp-marquee-window').hover();
  await page.waitForFunction(()=>document.querySelector('.sp-marquee-track').getAnimations()[0]?.playbackRate===0);
  await page.mouse.move(0,0);
  await page.waitForFunction(()=>document.querySelector('.sp-marquee-track').getAnimations()[0]?.playbackRate===1);
  await page.emulateMedia({reducedMotion:'reduce'});
  await page.waitForFunction(()=>getComputedStyle(document.querySelector('.sp-marquee-track')).animationName==='none');
  assert.equal(await track.evaluate(node=>getComputedStyle(node).animationName),'none','reduced motion disables marquee animation');
  const env=await readFile(new URL('../web/.env.local',import.meta.url),'utf8');
  const project=env.match(/NEXT_PUBLIC_SUPABASE_URL\s*=\s*["']?https:\/\/([^.]+)/)?.[1];
  assert.ok(project,'local public account project configuration is available');
  const context=await browser.newContext({viewport:{width:390,height:900},reducedMotion:'reduce'});
  await context.addInitScript(project=>{
    const user={id:'00000000-0000-4000-8000-000000000001',aud:'authenticated',role:'authenticated',email:'preview@example.invalid'};
    localStorage.setItem(`sb-${project}-auth-token`,JSON.stringify({access_token:'preview-token',refresh_token:'preview-refresh',expires_at:Math.floor(Date.now()/1000)+3600,expires_in:3600,token_type:'bearer',user}));
  },project);
  await context.route('**/auth/v1/**',route=>route.fulfill({status:200,contentType:'application/json',body:JSON.stringify({id:'00000000-0000-4000-8000-000000000001',aud:'authenticated',role:'authenticated',email:'preview@example.invalid'})}));
  const signedIn=await context.newPage();
  for(const route of ['/','/docs']) for(const width of [390,1440]) {
    await signedIn.setViewportSize({width,height:900});
    await signedIn.goto(base+route,{waitUntil:'networkidle'});
    const primary=signedIn.locator('header .sp-account-cta');
    assert.equal(await primary.innerText(),'Console');
    assert.equal(await primary.getAttribute('href'),'/console');
    const rgb=(await primary.evaluate(node=>getComputedStyle(node).backgroundColor)).match(/\d+/g).map(Number);
    assert.ok(rgb[0]>2*rgb[1]&&rgb[0]>2*rgb[2],'account is the red primary action');
    if(width===1440) {
      const secondary=signedIn.locator(route==='/'?'.sp-nav-right .sp-nav-cta':'.sp-site-actions .sp-site-download');
      assert.ok((await secondary.boundingBox()).x<(await primary.boundingBox()).x,'Download precedes account');
    }
  }
  await context.close();
  console.log('PASS: independent support strip, conversation/handoff/phone hero, no report graphic, 3 widths and both themes.');
} finally {await browser.close();}
