import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { mkdir } from 'node:fs/promises';
import { generateKeyPair, createChannel } from '../web/lib/remote-crypto.mjs';

const { chromium } = createRequire(import.meta.url)('playwright');
const [site, relay] = process.argv.slice(2);
if (!site || !relay?.startsWith('wss://')) throw Error('Usage: node scripts/check-remote-companion.mjs <site> <public-wss-relay>');
const keys=await generateKeyPair(); const room=crypto.randomUUID().replaceAll('-','');
const desktop=new WebSocket(`${relay}/${room}?role=desktop`); desktop.binaryType='arraybuffer';
await new Promise((resolve,reject)=>{desktop.onopen=resolve;desktop.onerror=reject;setTimeout(()=>reject(Error('Relay timeout')),15000).unref();});
let channel; let incoming=Promise.resolve(); const requests=[]; const failures=[]; let sessionReads=0;
async function reply(value) {
  const frame=await channel.seal({type:'chunk',data:JSON.stringify(value),final:true});
  desktop.send(new TextEncoder().encode(JSON.stringify(frame)));
}
desktop.onmessage=event=>{
  incoming=incoming.then(async()=>{
    const frame=JSON.parse(new TextDecoder().decode(event.data));
    if(frame.type==='hello') {channel=await createChannel(keys,frame.public_key,room,'desktop');return;}
    const message=await channel.open(frame); requests.push(message);
    if(message.type==='confirm') await reply({type:'confirm'});
    else if(message.type==='sessions') { sessionReads++; await reply({type:'sessions',sessions:sessionReads===1?[]:[{id:'codex-hosted',title:'Review authentication',agent:'codex',model:sessionReads>2?'gpt-6.1-sol':null,state:sessionReads>2?'idle':'waiting_for_history',can_prompt:sessionReads>2}]}); }
    else if(message.type==='chat') await reply({type:'chat',session_id:message.session_id,events:[
      {event_id:'0',turn_index:0,role:'system',status:'model_active',content:'Model active: gpt-6.1-sol',created_at:'2026-10-29T00:00:00Z'},
      {event_id:'1',turn_index:1,role:'user',content:'Review authentication',created_at:'2026-10-29T00:00:00Z'},
      {event_id:'2',turn_index:2,role:'assistant',content:'The login flow is **ready** for review.',created_at:'2026-10-29T00:00:01Z'},
      {event_id:'3',turn_index:3,role:'tool',content:'Command output: hidden',created_at:'2026-10-29T00:00:02Z'},
    ]});
    else if(message.type==='prompt') await reply({type:'prompt_ack',session_id:message.session_id});
    else if(message.type==='launch') await reply({type:'launch_ack',session_id:'codex:new'});
    else if(message.type==='workspaces') {
      const selected=message.path==='C:\\workspace\\picked';
      await reply({type:'workspaces',path:selected?'C:\\workspace\\picked':'C:\\workspace',parent:selected?'C:\\workspace':'C:\\',roots:['C:\\','D:\\'],folders:selected?[]:[{name:'picked',path:'C:\\workspace\\picked'}],truncated:false});
    }
  }).catch(error=>failures.push(error.message));
};
const browser=await chromium.launch(process.env.SPAWNT_TEST_BROWSER ? {executablePath:process.env.SPAWNT_TEST_BROWSER} : {});
try {
  const page=await browser.newPage({viewport:{width:390,height:844}});
  page.on('pageerror',error=>failures.push(error.message));
  await page.goto(`${site}/remote#${new URLSearchParams({room,key:keys.publicHex,relay})}`);
  assert.equal(await page.locator('footer').count(),0,'Remote must use the app shell, without the marketing footer');
  const remoteBox = await page.locator('.remote-heading').boundingBox();
  assert.ok(remoteBox.x >= 16 && remoteBox.width <= 358,'phone conversation needs horizontal padding');
  await page.getByRole('button',{name:'Connect securely'}).click();
  await page.getByRole('button',{name:'Codes match'}).waitFor();
  for(let attempt=0;attempt<100 && !channel;attempt++) await new Promise(resolve=>setTimeout(resolve,50));
  assert.ok(channel, 'desktop receives the phone public key');
  assert.match(await page.locator('.sp-remote').textContent(),new RegExp(channel.verifyCode));
  await mkdir(new URL('../.tmp/remote-verification/',import.meta.url),{recursive:true});
  await page.screenshot({path:new URL('../.tmp/remote-verification/phone-verification.png',import.meta.url).pathname.replace(/^\/([A-Za-z]:)/,'$1'),fullPage:true});
  await page.getByRole('button',{name:'Codes match'}).click();
  await page.locator('.sp-remote.is-paired').waitFor();
  await page.getByText('The login flow is ready for review.').waitFor();
  assert.equal(await page.locator('.remote-session-list button[aria-current]').count(),1,'newest session is discovered and opened without manual selection');
  await page.locator('.sp-chat-body strong').getByText('ready').waitFor();
  const chatWidth = await page.locator('.sp-remote-chat').evaluate(node => node.clientWidth - parseFloat(getComputedStyle(node).paddingLeft) - parseFloat(getComputedStyle(node).paddingRight));
  const streamBox = await page.locator('.sp-chat-stream').boundingBox();
  assert.ok(streamBox.width >= chatWidth - 1,`phone bubbles use the available conversation width (${streamBox.width}/${chatWidth})`);
  await page.locator('.sp-chat-notice').filter({hasText:'Model active: gpt-6.1-sol'}).waitFor();
  assert.doesNotMatch(await page.locator('.sp-remote-chat').textContent(),/Command output/);
  assert.equal(await page.locator('.sp-chat-user').evaluate(node=>getComputedStyle(node).backgroundColor),'rgb(255, 37, 68)','phone user bubbles use Spawnt red');
  assert.equal(await page.locator('.sp-remote-chat').evaluate(element=>[...element.querySelectorAll('.sp-chat-body,pre,.sp-chat-table')].every(node=>node.scrollWidth<=node.clientWidth+1)),true,'phone bubble content never scrolls horizontally');
  await page.locator('#remote-prompt').fill('Review synthetic input\nThen run checks');
  await page.waitForFunction(()=>!document.querySelector('button[type="submit"]:disabled') || [...document.querySelectorAll('button')].some(button=>button.textContent==='Send'&&!button.disabled));
  await page.getByRole('button',{name:'Send',exact:true}).click();
  await page.waitForFunction(()=>document.getElementById('remote-prompt').value==='');
  assert.equal(requests.find(request=>request.type==='prompt').text,'Review synthetic input\nThen run checks');
  assert.equal(requests.find(request=>request.type==='prompt').session_id,'codex-hosted');
  await mkdir(new URL('../.tmp/remote-verification/',import.meta.url),{recursive:true});
  await page.evaluate(() => window.scrollTo(0,0));
  await page.screenshot({path:new URL('../.tmp/remote-verification/phone-chat.png',import.meta.url).pathname.replace(/^\/([A-Za-z]:)/,'$1'),fullPage:true});
  for (const width of [1440,390]) for (const theme of ['dark','light']) {
    await page.setViewportSize({width,height:900});
    await page.emulateMedia({colorScheme:theme});
    await page.waitForTimeout(220);
    assert.equal(await page.locator('.sp-remote-chat').evaluate(element=>[...element.querySelectorAll('.sp-chat-body,pre,.sp-chat-table')].every(node=>node.scrollWidth<=node.clientWidth+1)),true,`Remote bubbles fit ${width}/${theme}`);
    await page.screenshot({path:new URL(`../.tmp/remote-verification/paired-${theme}-${width}.png`,import.meta.url).pathname.replace(/^\/([A-Za-z]:)/,'$1'),fullPage:true});
    await page.locator('.remote-actions > summary').click();
    await page.waitForTimeout(220);
    assert.equal(await page.getByRole('button',{name:'Disconnect',exact:true}).evaluate(node=>getComputedStyle(node).color),theme==='dark'?'rgb(246, 247, 248)':'rgb(12, 17, 25)','Remote action foreground follows theme');
    await page.screenshot({path:new URL(`../.tmp/remote-verification/actions-${theme}-${width}.png`,import.meta.url).pathname.replace(/^\/([A-Za-z]:)/,'$1'),fullPage:true});
    await page.locator('.remote-actions > summary').click();
  }
  await page.getByRole('button',{name:'Toggle conversations',exact:true}).click();
  await page.getByText('New agent session',{exact:true}).click();
  await page.getByRole('button',{name:'Browse desktop folders',exact:true}).click();
  await page.getByRole('button',{name:'picked',exact:true}).click();
  await page.getByText('No subfolders.',{exact:true}).waitFor();
  await page.getByRole('button',{name:'Use this folder',exact:true}).click();
  assert.equal(await page.locator('#remote-launch-repo').inputValue(),'C:\\workspace\\picked');
  await page.getByRole('button',{name:'Launch Agent',exact:true}).click();
  await page.getByText('Agent launched. Waiting for its conversation...').waitFor();
  assert.equal(requests.find(request=>request.type==='launch').agent,'claude');
  assert.equal(requests.find(request=>request.type==='launch').repo_path,'C:\\workspace\\picked');
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);
  await page.locator('.remote-actions > summary').click();
  await page.getByRole('button',{name:'Disconnect',exact:true}).click();
  await page.getByRole('button',{name:'Connect securely',exact:true}).waitFor();
  await page.waitForTimeout(100);
  assert.deepEqual(await page.locator('.sp-remote [role="alert"]').allTextContents(),[],'intentional disconnect must not show a connection failure');
  assert.deepEqual(failures,[]);
  console.log('PASS: real companion browser pairs through public relay, renders clean history, sends encrypted prompts and launch commands at phone width');
} finally {await browser.close(); desktop.close();}
process.exit(0);
