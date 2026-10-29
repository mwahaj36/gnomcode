import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { generateKeyPair, createChannel } from '../web/lib/remote-crypto.mjs';

const { chromium } = createRequire(import.meta.url)('playwright');
const [site, relay] = process.argv.slice(2);
if (!site || !relay?.startsWith('wss://')) throw Error('Usage: node scripts/check-remote-companion.mjs <site> <public-wss-relay>');
const keys=await generateKeyPair(); const room=crypto.randomUUID().replaceAll('-','');
const desktop=new WebSocket(`${relay}/${room}?role=desktop`); desktop.binaryType='arraybuffer';
await new Promise((resolve,reject)=>{desktop.onopen=resolve;desktop.onerror=reject;setTimeout(()=>reject(Error('Relay timeout')),15000).unref();});
let channel; let incoming=Promise.resolve(); const requests=[]; const failures=[];
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
    else if(message.type==='sessions') await reply({type:'sessions',sessions:[{id:'codex:demo',title:'Review authentication',agent:'codex',state:'idle',can_prompt:true}]});
    else if(message.type==='chat') await reply({type:'chat',session_id:message.session_id,events:[
      {event_id:'1',turn_index:1,role:'user',content:'Review authentication',created_at:'2026-10-29T00:00:00Z'},
      {event_id:'2',turn_index:2,role:'assistant',content:'The login flow is ready for review.',created_at:'2026-10-29T00:00:01Z'},
      {event_id:'3',turn_index:3,role:'tool',content:'Command output: hidden',created_at:'2026-10-29T00:00:02Z'},
    ]});
    else if(message.type==='prompt') await reply({type:'prompt_ack'});
    else if(message.type==='launch') await reply({type:'launch_ack',session_id:'codex:new'});
  }).catch(error=>failures.push(error.message));
};
const browser=await chromium.launch();
try {
  const page=await browser.newPage({viewport:{width:390,height:844}});
  page.on('pageerror',error=>failures.push(error.message));
  await page.goto(`${site}/remote#${new URLSearchParams({room,key:keys.publicHex,relay})}`);
  await page.getByRole('button',{name:'Connect securely'}).click();
  await page.getByRole('button',{name:'Codes match'}).waitFor();
  for(let attempt=0;attempt<100 && !channel;attempt++) await new Promise(resolve=>setTimeout(resolve,50));
  assert.ok(channel, 'desktop receives the phone public key');
  assert.match(await page.locator('.sp-remote').textContent(),new RegExp(channel.verifyCode));
  await page.getByRole('button',{name:'Codes match'}).click();
  await page.getByText('Encrypted connection active.').waitFor();
  await page.getByLabel('Agent session').selectOption('codex:demo');
  await page.getByText('The login flow is ready for review.').waitFor();
  assert.doesNotMatch(await page.locator('.sp-remote-chat').textContent(),/Command output/);
  await page.locator('#remote-prompt').fill('Review synthetic input');
  await page.getByRole('button',{name:'Send prompt',exact:true}).click();
  await page.waitForFunction(()=>document.getElementById('remote-prompt').value==='');
  assert.equal(requests.find(request=>request.type==='prompt').text,'Review synthetic input');
  await page.locator('#remote-launch-repo').fill('C:\\workspace\\demo');
  await page.getByRole('button',{name:'Launch Agent',exact:true}).click();
  await page.getByText('Agent launched. Refresh sessions after the CLI starts.').waitFor();
  assert.equal(requests.find(request=>request.type==='launch').agent,'claude');
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);
  assert.deepEqual(failures,[]);
  console.log('PASS: real companion browser pairs through public relay, renders clean history, sends encrypted prompts and launch commands at phone width');
} finally {await browser.close(); desktop.close();}
process.exit(0);
