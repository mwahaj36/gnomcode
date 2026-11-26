import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { mkdir,readFile } from 'node:fs/promises';
import { generateKeyPair, createChannel } from '../web/lib/remote-crypto.mjs';

const { chromium } = createRequire(import.meta.url)('playwright');
const [site, relay] = process.argv.slice(2);
const modelsOnly=process.argv.includes('--models-only');
if (!site || !/^wss:\/\/|^ws:\/\/127\.0\.0\.1:/.test(relay||'')) throw Error('Usage: node scripts/check-remote-companion.mjs <site> <public-wss-relay>');
const keys=await generateKeyPair(); let room=crypto.randomUUID().replaceAll('-','');
let finished=false,reconnectTimer,launched=false,firstPrompt=false,resumes=0,savedResumed=false,modelCatalogError=false;
let desktop=new WebSocket(`${relay}/${room}?role=desktop`); desktop.binaryType='arraybuffer';
await new Promise((resolve,reject)=>{desktop.onopen=resolve;desktop.onerror=reject;setTimeout(()=>reject(Error('Relay timeout')),15000).unref();});
const secret=crypto.getRandomValues(new Uint8Array(32));const secretHex=Array.from(secret,b=>b.toString(16).padStart(2,'0')).join('');
const proofKey=await crypto.subtle.importKey('raw',secret,{name:'HMAC',hash:'SHA-256'},false,['verify']);
let resumeContext='';
let channel; let incoming=Promise.resolve(); const requests=[]; const failures=[]; let sessionReads=0;
async function reply(value) {
  const frame=await channel.seal({type:'chunk',data:JSON.stringify(value),final:true});
  desktop.send(new TextEncoder().encode(JSON.stringify(frame)));
}
function bindDesktop(){const connection=desktop;channel=null;desktop.onmessage=event=>{
  incoming=incoming.then(async()=>{
    if(connection!==desktop)return;
    const frame=JSON.parse(new TextDecoder().decode(event.data));
    if(frame.type==='hello') {if(channel)return;const nonce=Array.from(crypto.getRandomValues(new Uint8Array(16)),b=>b.toString(16).padStart(2,'0')).join('');resumeContext=`${room}:${nonce}:${frame.public_key}`;const fresh=await createChannel(keys,frame.public_key,`${room}:${nonce}`,'desktop');if(connection!==desktop)return;channel=fresh;desktop.send(new TextEncoder().encode(JSON.stringify({type:'challenge',nonce})));return;}
    const message=await channel.open(frame);if(connection!==desktop)return; requests.push(message);
    if(message.type==='enroll') {await reply({type:'trusted',secret:secretHex});}
    else if(message.type==='resume'){const proof=Uint8Array.from(message.proof.match(/../g),part=>parseInt(part,16));assert.equal(await crypto.subtle.verify('HMAC',proofKey,proof,new TextEncoder().encode(resumeContext)),true);resumes++;await reply({type:'resumed'});}
    else if(message.type==='sessions') { sessionReads++; await reply({type:'sessions',sessions:sessionReads===1?[]:[{id:'saved-session',title:'Saved conversation',agent:'codex',state:savedResumed?'idle':'ended',hosted:savedResumed,can_prompt:savedResumed,can_resume:!savedResumed},{id:'external-history',title:'External history',agent:'codex',state:'idle',hosted:false,can_prompt:false},...(launched?[{id:'codex:new',title:'New phone conversation',agent:'codex',state:'waiting_for_history',hosted:true,can_prompt:true}]:[]),{hosted:true,id:'codex-hosted',limits:[{label:'5h',remaining_pct:sessionReads>4?37:75,basis:'exact',observed_at:new Date().toISOString(),stale:false}],title:'Review authentication',agent:'codex',model:sessionReads>2?'gpt-6.1-sol':null,state:sessionReads>2?'idle':'waiting_for_history',can_prompt:sessionReads>2}]}); }
    else if(message.type==='models') await reply(modelCatalogError?{type:'error',request_type:'models',message:'Update desktop model service.'}:{type:'models',providers:[{agent:'codex',name:'OpenAI Codex',status:'authenticated',message:'Signed in',models:[{agent:'codex',model:'gpt-test',display_name:'GPT test'}]},{agent:'claude',name:'Claude Code',status:'authenticated',message:'Signed in',models:[{agent:'claude',model:'sonnet',display_name:'Sonnet test'}]},{agent:'antigravity',name:'Antigravity',status:'unknown',message:'Native catalog unavailable.',models:[]}]});
    else if(message.type==='chat') await reply({type:'chat',session_id:message.session_id,events:message.session_id==='codex:new' ? (firstPrompt?[{event_id:'new-reply',turn_index:1,role:'assistant',content:'First phone prompt received.',created_at:new Date().toISOString()}]:[]) : [
      {event_id:'0',turn_index:0,role:'system',status:'model_active',content:'Model active: gpt-6.1-sol',created_at:'2026-10-29T00:00:00Z'},
      {event_id:'1',turn_index:1,role:'user',content:'Review authentication',created_at:'2026-10-29T00:00:00Z'},
      {event_id:'2',turn_index:2,role:'assistant',content:'The login flow is **ready** for review.',created_at:'2026-10-29T00:00:01Z'},
      {event_id:'3',turn_index:3,role:'tool',content:'Command output: hidden',created_at:'2026-10-29T00:00:02Z'},
    ]});
    else if(message.type==='resume_session') {assert.equal(message.session_id,'saved-session');savedResumed=true;await reply({type:'resume_session_ack',session_id:message.session_id});}
    else if(message.type==='switch') {await reply({type:'switch_ack',session_id:message.session_id,new_session_id:message.session_id});}
    else if(message.type==='prompt') {if(message.session_id==='codex:new')firstPrompt=true;await reply({type:'prompt_ack',session_id:message.session_id});}
    else if(message.type==='launch') {launched=true;await reply({type:'launch_ack',session_id:'codex:new'});}
    else if(message.type==='workspaces') {
      const selected=message.path==='C:\\workspace\\picked';
      await reply({type:'workspaces',path:selected?'C:\\workspace\\picked':'C:\\workspace',parent:selected?'C:\\workspace':'C:\\',roots:['C:\\','D:\\'],folders:selected?[]:[{name:'picked',path:'C:\\workspace\\picked'}],truncated:false});
    }
  }).catch(error=>failures.push(error.message));
};desktop.onclose=()=>{if(!finished && connection===desktop)reconnectTimer=setTimeout(()=>{desktop=new WebSocket(`${relay}/${room}?role=desktop`);desktop.binaryType='arraybuffer';bindDesktop();},200);};
}
bindDesktop();
const browser=await chromium.launch(process.env.GNOMCODE_TEST_BROWSER ? {executablePath:process.env.GNOMCODE_TEST_BROWSER} : {});
try {
  const page=await browser.newPage({viewport:{width:390,height:844}});
  page.setDefaultTimeout(15000);
  page.on('pageerror',error=>failures.push(error.message));
  const owner='10000000-0000-0000-0000-000000000001',desktopId='d'.repeat(32);
  const env=await readFile(new URL('../web/.env.local',import.meta.url),'utf8');
  const project=env.match(/NEXT_PUBLIC_SUPABASE_URL\s*=\s*["']?https:\/\/([^.]+)/)?.[1];
  let deviceRequest=null,approved=false,failDiscoveryOnce=false,holdDiscovery=false,discoveryStarted=false;
  const offer=()=>({room,key:keys.publicHex,relay,protocol_version:3,owner_id:owner,desktop_id:deviceRequest?.desktop_id||desktopId,request_id:room});
  await page.route('**/rest/v1/spawnt_device_requests**',async route=>{
    if(route.request().method()==='POST'){
      deviceRequest=route.request().postDataJSON();room=deviceRequest.id;approved=true;
      const previous=desktop;desktop=new WebSocket(`${relay}/${room}?role=desktop`);desktop.binaryType='arraybuffer';bindDesktop();previous.close();
      return route.fulfill({status:201,contentType:'application/json',body:''});
    }
    const singular=new URL(route.request().url()).searchParams.get('select')==='offer,revoked,state';
    if(singular && holdDiscovery){discoveryStarted=true;await new Promise(resolve=>setTimeout(resolve,3000));holdDiscovery=false;}
    if(singular && failDiscoveryOnce){failDiscoveryOnce=false;await new Promise(resolve=>setTimeout(resolve,11000));return route.fulfill({status:503,contentType:'application/json',body:JSON.stringify({message:'Temporary directory outage'})}).catch(()=>{});}
    const value=deviceRequest?{...deviceRequest,offer:approved?offer():null,state:approved?'approved':'pending',revoked:false}:null;
    return route.fulfill({status:200,contentType:'application/json',body:JSON.stringify(singular?value:value?[value]:[])});
  });
  await page.route('**/rest/v1/spawnt_desktops**',route=>route.fulfill({status:200,contentType:'application/json',body:JSON.stringify([{id:desktopId,name:'Test desktop',remote_enabled:true,last_seen_at:new Date().toISOString()},{id:'e'.repeat(32),name:'Second desktop',remote_enabled:true,last_seen_at:new Date().toISOString()}])}));
  await page.route('**/rest/v1/gnomcode_sync_state**',route=>route.fulfill({status:200,contentType:'application/json',body:JSON.stringify({live_snapshot:{sessions:[]}})}));
  await page.route('**/auth/v1/**',route=>route.fulfill({status:200,contentType:'application/json',body:JSON.stringify({id:owner,email:'test@example.invalid',aud:'authenticated',role:'authenticated'})}));
  await page.addInitScript(({project,owner})=>{
    localStorage.setItem(`sb-${project}-auth-token`,JSON.stringify({access_token:'test-access',refresh_token:'test-refresh',expires_at:Math.floor(Date.now()/1000)+3600,token_type:'bearer',user:{id:owner,email:'test@example.invalid',aud:'authenticated',role:'authenticated'}}));
    const Native=window.WebSocket;window.testSockets=[];window.WebSocket=class extends Native{constructor(...args){super(...args);window.testSockets.push(this);this.addEventListener('message',event=>{if(window.dropRemoteFrames)event.stopImmediatePropagation();});}};
  },{project,owner});
  await page.goto(`${site}/remote`);
  // A signed-in phone connects without requesting human approval on either desktop.
  await page.locator('.sp-remote.is-paired').waitFor();
  assert.equal(await page.locator('footer').count(),0,'Remote must use the app shell, without the marketing footer');
  const remoteBox = await page.locator('.remote-heading').boundingBox();
  assert.ok(remoteBox.x >= 0 && remoteBox.width <= 390,'phone conversation fits the app viewport');

  assert.equal(await page.getByRole('button',{name:'Codes match'}).count(),0,'no code confirmation exists');
  await page.getByText('The login flow is ready for review.').waitFor();
  holdDiscovery=true;
  await page.evaluate(()=>window.testSockets.at(-1).close());
  await page.waitForFunction(()=>document.querySelector('.remote-reconnect'));
  await new Promise((resolve,reject)=>{const started=Date.now();const timer=setInterval(()=>{if(discoveryStarted){clearInterval(timer);resolve();}else if(Date.now()-started>15000){clearInterval(timer);reject(new Error('Reconnect discovery did not start'));}},25);});
  assert.equal(await page.locator('.remote-workspace').isVisible(),true,'conversation stays visible while reconnect discovery is pending');
  assert.equal(await page.getByText('The login flow is ready for review.').isVisible(),true);
  await page.locator('.sp-remote.is-paired').waitFor();
  await page.evaluate(owner=>new Promise((resolve,reject)=>{
    const request=indexedDB.open('gnomcode-remote-device',1);
    request.onsuccess=()=>{const db=request.result;const tx=db.transaction('devices','readwrite'),store=tx.objectStore('devices');const get=store.getAll();get.onsuccess=()=>{const saved=get.result.find(value=>value.offer.owner_id===owner);const old='0'.repeat(32);store.put({...saved,offer:{...saved.offer,desktop_id:old,room:old,request_id:old}},`${owner}:${old}`);};tx.oncomplete=()=>{db.close();resolve();};tx.onerror=reject;};request.onerror=reject;
  }),owner);
  await page.evaluate(()=>sessionStorage.removeItem('gnomcode-pairing-offer'));
  await page.getByRole('button',{name:'Desktop connection',exact:true}).click();
  await page.getByRole('link',{name:'Dashboard',exact:true}).click();
  for(let attempt=0;attempt<3;attempt++){
    await page.getByRole('button',{name:'Sessions',exact:true}).click();
    await page.getByRole('button',{name:'Workspace',exact:true}).click();
    await page.locator('.sp-remote.is-paired').waitFor();
    await page.getByText('The login flow is ready for review.').waitFor();
    await page.goBack();
    await page.locator('.sp-remote').waitFor({state:'detached'});
    await page.goForward();
    await page.locator('.sp-remote.is-paired').waitFor();
    await page.getByText('The login flow is ready for review.').waitFor();
    await page.getByRole('button',{name:'Desktop connection',exact:true}).click();
    await page.getByRole('link',{name:'Dashboard',exact:true}).click();
  }
  await page.getByRole('button',{name:'Workspace',exact:true}).click();
  await page.locator('.sp-remote.is-paired').waitFor();
  await page.goto(`${site}/downloads`);
  failDiscoveryOnce=true;
  await page.goto(`${site}/remote`);
  await page.locator('.sp-remote.is-paired').waitFor();
  await page.getByText('The login flow is ready for review.').waitFor();
  assert.equal(failDiscoveryOnce,false,'reopening the chat exercises the temporary directory failure');
  assert.equal(await page.locator('.remote-session-list button[aria-current]').count(),1,'newest session is discovered and opened without manual selection');
  await page.locator('.sp-chat-body strong').getByText('ready').waitFor();
  await page.locator('.remote-heading').getByText('37% left',{exact:true}).waitFor();
  await page.locator('#remote-prompt').fill('Draft survives provider selection');
  await page.getByRole('button',{name:'Change model',exact:true}).click();
  await page.getByRole('button',{name:'Sonnet test',exact:true}).waitFor();
  assert.equal(await page.getByRole('button',{name:'GPT test',exact:true}).isVisible(),true);
  assert.equal(await page.getByText('Native catalog unavailable.',{exact:true}).isVisible(),true);
  assert.equal(await page.locator('#remote-model').count(),0,'model selection never requires typing a model ID');
  await page.screenshot({path:new URL('../.tmp/remote-verification/model-picker-390.png',import.meta.url).pathname.replace(/^\/([A-Za-z]:)/,'$1'),fullPage:true});
  modelCatalogError=true;
  await page.getByRole('button',{name:'Refresh models',exact:true}).click();
  await page.getByText('Your desktop could not provide a model list. Update Gnomcode on your desktop, then refresh.',{exact:true}).waitFor();
  assert.equal(await page.locator('#remote-prompt').inputValue(),'Draft survives provider selection','catalog errors do not cancel the prompt');
  modelCatalogError=false;
  await page.getByRole('button',{name:'Refresh models',exact:true}).click();
  await page.getByRole('button',{name:'Sonnet test',exact:true}).click();
  await page.getByText('Switching model...',{exact:true}).waitFor();
  assert.equal(requests.filter(request=>request.type==='switch').at(-1).agent,'claude');
  assert.equal(requests.filter(request=>request.type==='switch').at(-1).model,'sonnet');
  assert.equal(await page.locator('#remote-prompt').inputValue(),'Draft survives provider selection');
  if(!modelsOnly){
  const chatWidth = await page.locator('.sp-remote-chat').evaluate(node => node.clientWidth - parseFloat(getComputedStyle(node).paddingLeft) - parseFloat(getComputedStyle(node).paddingRight));
  const streamBox = await page.locator('.sp-chat-stream').boundingBox();
  assert.ok(streamBox.width >= chatWidth - 1,`phone bubbles use the available conversation width (${streamBox.width}/${chatWidth})`);
  await page.locator('.sp-chat-notice').filter({hasText:'Model active: gpt-6.1-sol'}).waitFor();
  assert.doesNotMatch(await page.locator('.sp-remote-chat').textContent(),/Command output/);
  assert.equal(await page.locator('.sp-chat-user').evaluate(node=>{const probe=document.createElement('span');probe.style.color='var(--sp-primary)';node.appendChild(probe);const want=getComputedStyle(probe).color;probe.remove();return getComputedStyle(node).backgroundColor===want;}),true,'phone user bubbles use the Gnomcode accent');
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
    await page.getByRole('button',{name:'Desktop connection',exact:true}).click();
    await page.waitForTimeout(220);
    assert.equal(await page.getByRole('link',{name:'Profile',exact:true}).isVisible(),true,'desktop panel keeps navigation visible');
    await page.screenshot({path:new URL(`../.tmp/remote-verification/actions-${theme}-${width}.png`,import.meta.url).pathname.replace(/^\/([A-Za-z]:)/,'$1'),fullPage:true});
    await page.getByRole('button',{name:'Close desktop connection',exact:true}).click();
  }
  await page.locator('.remote-new').click();
  await page.getByRole('button',{name:'Browse desktop folders',exact:true}).click();
  await page.locator('.remote-folder-modal[open]').waitFor();
  await page.screenshot({path:new URL('../.tmp/remote-verification/phone-folder-modal.png',import.meta.url).pathname.replace(/^\/([A-Za-z]:)/,'$1'),fullPage:true});
  const useFolder=await page.getByRole('button',{name:'Use this folder',exact:true}).boundingBox();assert.ok(useFolder && useFolder.y+useFolder.height<=900,'folder confirmation stays visible');
  await page.getByRole('button',{name:'picked',exact:true}).click();
  await page.getByText('No subfolders.',{exact:true}).waitFor();
  await page.getByRole('button',{name:'Use this folder',exact:true}).click();
  assert.equal(await page.locator('#remote-launch-repo').inputValue(),'C:\\workspace\\picked');
  await page.getByRole('button',{name:'Launch Agent',exact:true}).click();
  await page.getByText('Agent started. Send its first message below.').waitFor();
  assert.equal(requests.find(request=>request.type==='launch').agent,'claude');
  assert.equal(requests.find(request=>request.type==='launch').repo_path,'C:\\workspace\\picked');
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);
  await page.locator('#remote-prompt').fill('First message entirely from phone');
  await page.getByRole('button',{name:'Send',exact:true}).click();
  await page.getByText('First phone prompt received.',{exact:true}).waitFor();
  assert.equal(requests.filter(request=>request.type==='prompt').at(-1).session_id,'codex:new');
  await page.goto(`${site}/console?view=sessions`);
  await page.goto(`${site}/remote`);
  await page.locator('.sp-remote.is-paired').waitFor();
  await page.getByText('First phone prompt received.',{exact:true}).waitFor();
  assert.deepEqual(await page.locator('.sp-remote [role=alert]').allTextContents(),[],'leaving and reopening a conversation reconnects without pairing');
  await page.reload();
  await page.locator('.sp-remote.is-paired').waitFor();
  await page.getByText('First phone prompt received.',{exact:true}).waitFor();
  await page.locator('#remote-prompt').fill('Draft survives reconnect\nKeep this second line');
  await page.evaluate(()=>{window.dropRemoteFrames=true;});
  await page.getByRole('button',{name:'Try again',exact:true}).waitFor({timeout:25000});
  assert.equal(await page.locator('#remote-prompt').inputValue(),'Draft survives reconnect\nKeep this second line');
  assert.equal(await page.locator('.remote-composer').evaluate(node=>{const input=node.querySelector('textarea').getBoundingClientRect(),toolbar=node.querySelector('.remote-composer-toolbar').getBoundingClientRect();return input.bottom<=toolbar.top+1;}),true,'composer text never overlaps its controls');
  assert.equal(await page.getByRole('link',{name:'Profile',exact:true}).count(),1);
  assert.equal(await page.getByRole('link',{name:'Dashboard',exact:true}).isVisible(),true);
  assert.equal(await page.getByRole('button',{name:'Send',exact:true}).isDisabled(),true);
  await page.setViewportSize({width:390,height:350});
  await page.locator('#remote-prompt').focus();
  await page.waitForFunction(()=>document.querySelector('.sp-remote').getBoundingClientRect().height<=innerHeight+1);
  const reconnectSend=await page.getByRole('button',{name:'Send',exact:true}).boundingBox();
  if(!(reconnectSend && reconnectSend.y>=0 && reconnectSend.y+reconnectSend.height<=350)) console.log('Reconnect layout:', await page.locator('.sp-remote, .remote-heading, .remote-reconnect, .remote-workspace, .remote-conversation, .remote-composer').evaluateAll(nodes=>nodes.map(n=>({name:n.className,top:n.getBoundingClientRect().top,bottom:n.getBoundingClientRect().bottom,height:n.getBoundingClientRect().height}))));
  assert.ok(reconnectSend && reconnectSend.y>=0 && reconnectSend.y+reconnectSend.height<=350,'Send stays visible under a short reconnect keyboard');
  await page.screenshot({path:new URL('../.tmp/remote-verification/reconnecting-chat.png',import.meta.url).pathname.replace(/^\/([A-Za-z]:)/,'$1'),fullPage:true});
  await page.setViewportSize({width:390,height:844});
  await page.evaluate(()=>{window.dropRemoteFrames=false;});
  await page.getByRole('button',{name:'Try again',exact:true}).click();
  await page.locator('.sp-remote.is-paired').waitFor();
  assert.equal(await page.locator('#remote-prompt').inputValue(),'Draft survives reconnect\nKeep this second line');
  assert.equal(await page.getByText('Actions',{exact:true}).count(),0);
  await page.screenshot({path:new URL('../.tmp/remote-verification/recovered-chat.png',import.meta.url).pathname.replace(/^\/([A-Za-z]:)/,'$1'),fullPage:true});
  assert.ok(resumes>=1,'reload authenticates saved nonextractable device trust');
  assert.equal(await page.evaluate(key=>new Promise((resolve,reject)=>{const r=indexedDB.open('gnomcode-remote-device',1);r.onsuccess=()=>{const db=r.result;const q=db.transaction('devices').objectStore('devices').get(key);q.onsuccess=()=>{resolve(q.result?.proofKey?.extractable);db.close();};q.onerror=reject;};r.onerror=reject;}),`${owner}:${desktopId}`),false,'remembered proof key is not exportable');
  assert.equal(await page.getByRole('button',{name:'Codes match'}).count(),0,'reload does not ask for pairing');
  await page.evaluate(()=>{window.testSockets.at(-1).close();document.dispatchEvent(new Event('visibilitychange'));setTimeout(()=>document.dispatchEvent(new Event('visibilitychange')),10);});
  await page.locator('.sp-remote.is-paired').waitFor();
  await page.waitForFunction(()=>document.querySelector('.remote-heading :is(h1,h2)')?.textContent==='New phone conversation');
  for(const width of [320,390]) for(const height of [600,350,844]){
    await page.setViewportSize({width,height});await page.locator('#remote-prompt').focus();await page.waitForTimeout(100);
    assert.equal(await page.evaluate(()=>getComputedStyle(document.body).position),'fixed','keyboard focus cannot scroll the document');
    assert.equal(await page.locator('.remote-composer').evaluate(node=>{const input=node.querySelector('textarea').getBoundingClientRect(),toolbar=node.querySelector('.remote-composer-toolbar').getBoundingClientRect();return input.bottom<=toolbar.top;}),true,'text and buttons occupy separate rows');
    const send=await page.getByRole('button',{name:'Send',exact:true}).boundingBox();
    assert.ok(send && send.y>=0 && send.y+send.height<=height,`Send visible without page scroll at ${width}x${height}`);
    assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);
  }
  await page.getByRole('button',{name:'Open conversations',exact:true}).click();
  assert.equal(await page.getByRole('button',{name:'Close conversations',exact:true}).last().evaluate(node=>node===document.activeElement),true,'drawer receives keyboard focus');
  await page.keyboard.press('Escape');
  assert.equal(await page.getByRole('button',{name:'Open conversations',exact:true}).evaluate(node=>node===document.activeElement),true,'Escape closes the drawer and restores focus');
  await page.getByRole('button',{name:'Open conversations',exact:true}).click();
  await page.getByRole('button',{name:/External history/}).click();
  await page.getByText('Read-only history.',{exact:true}).waitFor();
  assert.equal(await page.locator('.remote-composer').count(),0);
  await page.getByRole('status').filter({hasText:'Loading messages...'}).waitFor({state:'hidden'});
  assert.deepEqual(await page.locator('.sp-remote [role=alert]').allTextContents(),[]);
  await page.screenshot({path:new URL('../.tmp/remote-verification/phone-readonly.png',import.meta.url).pathname.replace(/^\/([A-Za-z]:)/,'$1'),fullPage:true});
  await page.getByRole('button',{name:'Open conversations',exact:true}).click();
  await page.locator('#remote-history-search').fill('Saved');
  assert.equal(await page.locator('.remote-session-list button').count(),1);
  await page.getByRole('button',{name:/Saved conversation/}).click();
  await page.getByRole('button',{name:'Resume conversation',exact:true}).click();
  await page.getByText('Ready to continue.',{exact:true}).waitFor();
  await page.locator('.remote-composer').waitFor();
  assert.equal(await page.locator('.remote-heading :is(h1,h2)').textContent(),'Saved conversation','native resume retains the selected conversation');
  await page.locator('#remote-prompt').fill('Continue this saved chat');
  await page.getByRole('button',{name:'Change model',exact:true}).click();
  await page.getByRole('button',{name:'Sonnet test',exact:true}).waitFor();
  assert.equal(await page.getByRole('button',{name:'GPT test',exact:true}).isVisible(),true);
  await page.getByRole('button',{name:'Sonnet test',exact:true}).click();
  await page.getByText('Switching model...',{exact:true}).waitFor();
  assert.equal(requests.filter(request=>request.type==='switch').at(-1).agent,'claude');
  assert.equal(requests.filter(request=>request.type==='switch').at(-1).model,'sonnet');
  assert.equal(await page.locator('.remote-heading :is(h1,h2)').textContent(),'Saved conversation');
  assert.equal(await page.locator('#remote-prompt').inputValue(),'Continue this saved chat','model change retains the phone draft');
  await page.getByRole('button',{name:'Send',exact:true}).click();
  await page.waitForFunction(()=>document.getElementById('remote-prompt').value==='');
  assert.equal(requests.filter(request=>request.type==='prompt').at(-1).session_id,'saved-session','continued prompt targets the same saved conversation');
  await page.screenshot({path:new URL('../.tmp/remote-verification/phone-resumed.png',import.meta.url).pathname.replace(/^\/([A-Za-z]:)/,'$1'),fullPage:true});
  await page.locator('#remote-prompt').fill('Keep this draft while the desktop is offline');
  const promptCount=requests.filter(request=>request.type==='prompt').length;
  await page.evaluate(()=>{window.dropRemoteFrames=true;});
  await page.locator('.remote-reconnect').getByText('Desktop unavailable',{exact:true}).waitFor({timeout:55000});
  assert.equal(await page.locator('#remote-prompt').inputValue(),'Keep this draft while the desktop is offline');
  assert.equal(await page.getByRole('button',{name:'Send',exact:true}).isDisabled(),true);
  const socketCount=await page.evaluate(()=>window.testSockets.length);
  await page.waitForTimeout(3500);
  assert.equal(await page.evaluate(()=>window.testSockets.length),socketCount,'unreachable desktops stop automatic connection attempts');
  assert.equal(requests.filter(request=>request.type==='prompt').length,promptCount,'recovery never replays a prompt');
  await page.screenshot({path:'.tmp/remote-verification/offline-draft-390.png',fullPage:true});
  await page.evaluate(()=>{window.dropRemoteFrames=false;});
  await page.getByRole('button',{name:'Try again',exact:true}).click();
  await page.locator('.sp-remote.is-paired').waitFor();
  assert.equal(await page.locator('#remote-prompt').inputValue(),'Keep this draft while the desktop is offline');
  await page.getByRole('button',{name:'Desktop connection',exact:true}).click();
  await page.getByRole('button',{name:'Forget this desktop',exact:true}).click();
  await page.getByRole('heading',{name:'Devices',exact:true}).waitFor();
  await page.waitForTimeout(100);
  assert.deepEqual(await page.locator('.sp-remote [role="alert"]').allTextContents(),[],'intentional disconnect must not show a connection failure');
  await page.getByRole('button',{name:'Open desktop',exact:true}).last().waitFor();
  assert.equal(await page.getByRole('button',{name:'Open desktop',exact:true}).count(),2,'all linked native apps remain discoverable');
  await page.getByRole('button',{name:'Open desktop',exact:true}).last().click();
  await page.locator('.sp-remote.is-paired').waitFor();
  assert.equal(deviceRequest.desktop_id,'e'.repeat(32),'the same signed-in phone opens another native app without another approval');
  assert.equal(await page.getByRole('button',{name:'Approve',exact:true}).count(),0,'there is no per-desktop approval step');
  assert.deepEqual(failures,[]);
  console.log('PASS: account-wide automatic access across two native apps, encrypted device enrollment, touch sheets, searchable history, visible composer, resume/model switch, trust reload, stalled-open-socket recovery with preserved draft and usable navigation');
  }else{assert.deepEqual(failures,[]);console.log('PASS: encrypted provider-grouped model choices, explicit cross-provider switch, catalog failure/retry and preserved draft');}
} catch(error){console.error('Browser failures:',failures);for(const page of browser.contexts().flatMap(context=>context.pages())){console.error((await page.locator('body').innerText()).slice(-3000));await page.screenshot({path:'.tmp/remote-failure.png',fullPage:true});}throw error;} finally {finished=true;clearTimeout(reconnectTimer);await browser.close(); desktop.close();}
process.exit(0);
