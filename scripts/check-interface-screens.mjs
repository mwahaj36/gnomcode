import assert from 'node:assert/strict';
import {mkdir,readFile,writeFile} from 'node:fs/promises';
import {createRequire} from 'node:module';
const {chromium}=createRequire(import.meta.url)('playwright');
const base=process.argv[2] || 'http://127.0.0.1:43904';
const env=await readFile(new URL('../web/.env.local',import.meta.url),'utf8');
const project=env.match(/NEXT_PUBLIC_SUPABASE_URL\s*=\s*["']?https:\/\/([^.]+)/)?.[1];
const browser=await chromium.launch({headless:true,executablePath:process.env.GNOMCODE_TEST_BROWSER});
const findings=[];
const root=new URL('../.tmp/interface-review/',import.meta.url);
await mkdir(root,{recursive:true});
try {
 for(const width of [1440,390]) for(const theme of ['dark','light']) {
  const context=await browser.newContext({viewport:{width,height:900},reducedMotion:'reduce'});
  await context.addInitScript(({theme,project})=>{
   localStorage.setItem('gnomcode-theme',theme);
   const user={id:'00000000-0000-4000-8000-000000000001',aud:'authenticated',role:'authenticated',email:'preview@example.invalid'};
   if(location.pathname === '/auth') localStorage.removeItem(`sb-${project}-auth-token`);
   else localStorage.setItem(`sb-${project}-auth-token`,JSON.stringify({access_token:'preview-token',refresh_token:'preview-refresh',expires_at:Math.floor(Date.now()/1000)+3600,expires_in:3600,token_type:'bearer',user}));
  },{theme,project});
  await context.route('**/auth/v1/**',route=>route.fulfill({status:200,contentType:'application/json',body:JSON.stringify({id:'00000000-0000-4000-8000-000000000001',aud:'authenticated',role:'authenticated',email:'preview@example.invalid'})}));
  await context.route('**/rest/v1/**',route=>{
   const value={display_name:'Preview user',openai_plan_usd:20,claude_plan_usd:20,google_plan_usd:0,custom_pricing:[],remote_offer:{room:"a".repeat(32),key:"b".repeat(64),relay:"wss://relay.example/relay",expires_at:new Date(Date.now()+300000).toISOString(),name:"My desktop",pairing_code:"87654321"},live_snapshot:{generated_at:new Date().toISOString(),sessions:[],quota:[],active_count:0,ongoing_count:0,timelines:{}},updated_at:new Date().toISOString()};
   return route.fulfill({status:200,contentType:'application/json',body:JSON.stringify(value)});
  });
  const page=await context.newPage();
  const errors=[];page.on('pageerror',error=>errors.push(error.message));
  const routes=['/','/docs','/downloads','/pricing','/auth','/remote',...['overview','sessions','chat','usage','limits','projects','automations','settings'].map(view=>`/console?view=${view}`)];
  for(const route of routes){
   if(route==='/auth') await page.evaluate(project=>{const key=`sb-${project}-auth-token`;sessionStorage.setItem('preview-auth',localStorage.getItem(key)||'');localStorage.removeItem(key);},project);
   await page.goto(base+route,{waitUntil:'networkidle'});
   await page.evaluate(()=>document.fonts.ready);
   await page.waitForTimeout(300);
   if(['/','/docs'].includes(route)) assert.ok(await page.getByRole('link',{name:'Console',exact:true}).count() > 0, 'signed-in marketing navigation opens Console directly');
   if(route==='/console?view=settings') { await page.locator('#pairing-code').fill('00000000'); await page.getByRole('button',{name:'Pair desktop',exact:true}).click(); await page.getByRole('alert').filter({hasText:'Code does not match'}).waitFor(); await page.locator('#pairing-code').fill('87654321');
   assert.equal(await page.getByRole('button',{name:'Pair desktop',exact:true}).isEnabled(),true); }

   if(route==='/auth') assert.ok(await page.locator('#auth-email').isVisible(), 'Auth capture must show the signed-out form');
   const label=route==='/'?'home':route.replace(/[/?=]/g,'-').replace(/^-/, '');
   const metrics=await page.evaluate(()=>({overflow:document.documentElement.scrollWidth>innerWidth,headings:[...document.querySelectorAll('h1')].map(node=>node.textContent),invalid:/undefined|NaN/.test(document.body.innerText)}));
   if(metrics.overflow) console.log(route,width,await page.evaluate(()=>[...document.querySelectorAll('body *')].filter(node=>{if(node.getBoundingClientRect().right<=innerWidth)return false;for(let p=node.parentElement;p&&p!==document.body;p=p.parentElement)if(['auto','scroll','hidden'].includes(getComputedStyle(p).overflowX))return false;return true;}).slice(0,12).map(node=>({tag:node.tagName,cls:node.className,right:node.getBoundingClientRect().right,text:node.textContent.slice(0,40)}))));
   if(metrics.overflow||metrics.invalid||errors.length)findings.push({route,width,theme,...metrics,errors:[...errors]});
   await page.screenshot({path:new URL(`web-${label}-${theme}-${width}.png`,root).pathname.replace(/^\/([A-Za-z]:)/,'$1'),fullPage:true});
   if(route==='/auth') await page.evaluate(project=>{localStorage.setItem(`sb-${project}-auth-token`,sessionStorage.getItem('preview-auth')||'');},project);
   if(width===390 && ['/','/docs'].includes(route)){
    const menu=page.getByRole('button',{name:'Toggle menu'});
    await menu.click(); assert.equal(await menu.getAttribute('aria-expanded'),'true');
    await page.screenshot({path:new URL(`web-${label}-menu-${theme}-${width}.png`,root).pathname.replace(/^\/([A-Za-z]:)/,'$1'),fullPage:true});
    await menu.press('Escape');assert.equal(await menu.getAttribute('aria-expanded'),'false');
   }
  }
  await context.close();
 }
 await writeFile(new URL('web-findings.json',root),JSON.stringify(findings,null,2));
 assert.deepEqual(findings,[],'web screen sweep found layout or runtime defects');
 console.log('PASS: all web routes and console views in both themes at desktop and phone sizes; mobile menus open and dismiss with Escape');
}finally{await browser.close();}
