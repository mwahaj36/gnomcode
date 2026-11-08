import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {createRequire} from 'node:module';
const {chromium}=createRequire(import.meta.url)('playwright');
const root=new URL('../',import.meta.url);
const html=(await readFile(new URL('crates/daemon/src/console.html',root),'utf8')).replace(/<script\b[^>]*>[\s\S]*?<\/script>/g,'');
const browser=await chromium.launch({headless:true,...(process.env.GNOMCODE_TEST_BROWSER?{executablePath:process.env.GNOMCODE_TEST_BROWSER}:{channel:'msedge'})});
try{
 for(const width of [1440,900,390])for(const theme of ['dark','light']){
  const page=await browser.newPage({viewport:{width,height:900}});
  await page.route('**/*',async route=>{
   const url=new URL(route.request().url());
   if(url.pathname==='/')return route.fulfill({contentType:'text/html',body:html});
   const assets={'/gnomcode-console.css':'web/app/gnomcode-console.css','/gnomcode-tokens.css':'web/app/gnomcode-tokens.css'};
   return assets[url.pathname]?route.fulfill({contentType:'text/css',body:await readFile(new URL(assets[url.pathname],root),'utf8')}):route.fulfill({status:404,body:''});
  });
  await page.goto('http://127.0.0.1:49100/');
  await page.evaluate(theme=>{
   document.documentElement.dataset.theme=theme;
   document.body.dataset.consoleView='sessions';
   document.querySelectorAll('[data-local-view]').forEach(el=>el.hidden=!el.dataset.localView.includes('sessions'));
   document.querySelector('.conversation-filters').open=true;
   const hidden=document.createElement('div');hidden.id='hidden-layout-probe';hidden.hidden=true;hidden.style.display='flex';document.body.append(hidden);
  },theme);
  assert.equal(await page.locator('#hidden-layout-probe').isVisible(),false,'layout styles must respect hidden content');
  const box=await page.locator('#include-subagents').boundingBox();
  assert.ok(box&&box.width<=20&&box.height<=20,`${width}/${theme}: checkbox must remain compact`);
  assert.ok((await page.locator('.subagent-filter').boundingBox()).height>=44,'checkbox label remains a touch target');
  await page.evaluate(()=>{document.querySelector('#view-console').classList.remove('active');document.querySelector('#view-automations').classList.add('active');document.body.dataset.consoleView='automations';});
  for(const id of ['rule-agent','rule-spend','rule-turns','rule-action']){
   const field=page.locator('#'+id),rect=await field.boundingBox(),label=await field.locator('..').boundingBox();
   assert.ok(rect&&label&&rect.y>label.y,`${id}: label must sit above its field`);
   assert.ok(rect.height>=44&&rect.width>0,`${id}: control must be usable`);
   if(width<760)assert.ok(await field.evaluate(el=>parseFloat(getComputedStyle(el).fontSize)>=16),`${id}: phone inputs must avoid focus zoom`);
  }
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth+1),false,`${width}/${theme}: no page overflow`);
  await page.close();
 }
 console.log('PASS: checkbox geometry, touch targets, stacked field labels and responsive forms in both themes at three widths');
}finally{await browser.close()}
