import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {mkdir} from 'node:fs/promises';
const {chromium}=createRequire(import.meta.url)('playwright');
const site=process.argv[2];
if(!site)throw Error('Usage: node scripts/check-downloads-and-404.mjs <site>');
const browser=await chromium.launch(process.env.GNOMCODE_TEST_BROWSER?{executablePath:process.env.GNOMCODE_TEST_BROWSER}:{});
try{
  const page=await browser.newPage();
  await mkdir('.tmp/downloads-404-verification',{recursive:true});
  for(const width of [320,390,1440])for(const theme of ['dark','light']){
    await page.setViewportSize({width,height:900});
    await page.emulateMedia({colorScheme:theme});
    await page.goto(`${site}/downloads`);
    await page.getByRole('tab',{name:'Windows',exact:true}).click();
    assert.equal(await page.getByRole('link',{name:/Download Windows installer/}).getAttribute('href'),'/downloads/Gnomcode-Setup.exe');
    assert.match(await page.getByRole('link',{name:/Download ZIP/}).getAttribute('href'),/^\/downloads\/Gnomcode-v[\d.]+-windows-x64.zip$/);
    const legal=page.getByText('By downloading, you agree to the',{exact:false});
    assert.equal(await legal.evaluate(node=>getComputedStyle(node).display),'block','legal text keeps ordinary inline flow');
    for(const platform of ['macOS','Linux']){
      await page.getByRole('tab',{name:platform,exact:true}).click();
      assert.equal(await page.locator('.sp-download-unavailable').evaluate(node=>getComputedStyle(node).display),'block','guide text keeps spaces around its link');
      assert.equal(await page.locator('.sp-download-actions').count(),0,'unpublished platforms do not offer broken downloads');
    }
    await page.getByRole('tab',{name:'Windows',exact:true}).click();
    assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);
    await page.screenshot({path:`.tmp/downloads-404-verification/downloads-${theme}-${width}.png`,fullPage:true});
    const response=await page.goto(`${site}/this-page-does-not-exist`);
    assert.equal(response.status(),404);
    await page.getByRole('heading',{name:/Page not found/}).waitFor();
    for(const href of ['/','/downloads','/docs'])assert.ok(await page.locator(`.sp-not-found a[href="${href}"]`).count());
    assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);
    await page.screenshot({path:`.tmp/downloads-404-verification/404-${theme}-${width}.png`,fullPage:true});
  }
  await page.goto(`${site}/docs`);
  await page.getByRole('tab',{name:'Windows',exact:true}).click();
  assert.equal(await page.getByRole('link',{name:/Download Windows installer/}).getAttribute('href'),'/downloads/Gnomcode-Setup.exe');
  console.log('PASS: Windows downloads, inline legal/guide text, honest platform availability and themed responsive HTTP 404 recovery');
}finally{await browser.close();}
