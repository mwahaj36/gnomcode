import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile, mkdir } from 'node:fs/promises';
import { createRequire } from 'node:module';

// Run with Playwright available on NODE_PATH; exercises the shipped console against controlled API replies.
const { chromium } = createRequire(import.meta.url)('playwright');
const {WebSocketServer} = createRequire(new URL('../web/package.json',import.meta.url))('next/dist/compiled/ws');
const writes = [];
const errors = [];
let remotePaired = false;
const session = { id:'codex:sample', title:'Review authentication', agent:'codex', model:'gpt-6.1-sol', state:'idle', last_event_at:new Date().toISOString() };
const staticFiles = {
  '/':'crates/daemon/src/console.html', '/theme-init.js':'crates/daemon/src/theme-init.js', '/console-app.js':'crates/daemon/src/console-app.js',
  '/console-presentation.js':'web/lib/console-presentation.js', '/spawnt-console.css':'web/app/spawnt-console.css',
  '/vendor/xterm-addon-fit.js':'web/public/vendor/xterm-addon-fit.js', '/vendor/xterm.js':'web/public/vendor/xterm.js', '/vendor/xterm.css':'web/public/vendor/xterm.css', '/spawnt-tokens.css':'web/app/spawnt-tokens.css', '/spawnt-mark.svg':'web/public/spawnt-mark.svg',
};
const server = createServer(async (request, response) => {
  const url = new URL(request.url, 'http://localhost');
  let body = ''; for await (const chunk of request) body += chunk;
  if (request.method === 'POST') writes.push({ path:url.pathname, body:body ? JSON.parse(body) : null });
  let value = {};
  if (url.pathname === '/api/v1/auth/session') value = {paired:false};
  else if (url.pathname === '/api/v1/sync/status') value = {message:'This desktop is local only.',last_success_at:null};
  else if (url.pathname === '/api/v1/user') value = {display_name:'Developer',openai_plan_usd:20,claude_plan_usd:20,google_plan_usd:0};
  else if (url.pathname === '/api/v1/stats') value = {total_turns:42,total_projects:3,db_size_bytes:1048576};
  else if (['/api/v1/pricing/custom','/api/v1/resume-queue','/api/v1/rules','/api/v1/alerts'].includes(url.pathname)) value = [];
  else if (url.pathname === '/api/v1/license') value = {licensed:false};
  else if (url.pathname === '/api/v1/pro/relay') value = {relays:[],config:{}};
  else if (url.pathname === '/api/v1/sessions') value = { sessions:[session], page:1, total_pages:1, total_count:1 };
  else if (url.pathname === '/api/v1/sessions/hosted') value = { sessions:[
    ...(writes.some(write => write.path.endsWith('/switch')) ? [{session_id:'codex-switched',agent:'codex',title:'Model handoff',model:'o3',repo_path:'C:\\workspace\\demo',started_at:session.last_event_at}] : []),
    ...(writes.some(write => write.path.endsWith('/launch')) ? [{session_id:'codex-launched',agent:'codex',title:'Picked workspace',repo_path:'C:\\workspace\\picked',started_at:session.last_event_at}] : []),
  ] };
  else if (url.pathname === '/api/v1/projects') value = [];
  else if (url.pathname === '/api/v1/remote/status') value = {enabled:true,paired:remotePaired};
  else if (url.pathname.endsWith('/terminal')) value = {session_id:'terminal-test',repo_path:'C:\\workspace\\demo'};
  else if (url.pathname === '/api/v1/models') value = [{ model:'gpt-6.1-sol',agent:'codex',display_name:'GPT 6.1' },{ model:'o3', agent:'codex',display_name:'OpenAI o3' }];
  else if (url.pathname.endsWith('/chat')) value = [
    { event_id:'1', turn_index:1, role:'user', content:'Review authentication', created_at:session.last_event_at },
    { event_id:'2', turn_index:2, role:'assistant', content:'I will review the login flow.\n\n## Findings\n\n**Validated** with `cargo test`.\n\n- Session stays in chat\n- Terminal stays available\n\n```rust\nlet safe = "<script>";\n```\n\n| Check | Status |\n| --- | --- |\n| Tests | Pass |', created_at:session.last_event_at },
    { event_id:'3', turn_index:3, role:'tool', content:'Command output: do not display', created_at:session.last_event_at },
  ];
  else if (url.pathname.endsWith('/switch')) value = { session_id:'codex-switched' };
  else if (url.pathname === '/api/v1/sessions/launch') value = { session_id:'codex-launched' };
  else if (url.pathname.endsWith('/focus')) value = { message:'Agent window opened.', focused:true };
  else if (url.pathname === '/api/v1/agents/installed') value = { agents:[{id:'codex',name:'OpenAI Codex',installed:true},{id:'claude',name:'Claude Code',installed:false,install_command:'npm install -g @anthropic-ai/claude-code',login_command:'claude'},{id:'terminal',name:'Terminal Shell',installed:true}] };
  else if (url.pathname === '/api/v1/report') value = { timeline:{daily:[],providers:[],range_label:'All time'} };
  else if (url.pathname.includes('quota') || url.pathname.endsWith('/active')) value = [];
  if (staticFiles[url.pathname] || url.pathname.startsWith('/fonts/')) {
    const path = staticFiles[url.pathname] || `web/public${url.pathname}`;
    const type = path.endsWith('.html') ? 'text/html' : path.endsWith('.js') ? 'text/javascript' : path.endsWith('.css') ? 'text/css' : path.endsWith('.svg') ? 'image/svg+xml' : 'font/woff2';
    response.writeHead(200, {'Content-Type':type}); response.end(await readFile(new URL(`../${path}`, import.meta.url))); return;
  }
  response.writeHead(200, {'Content-Type':'application/json'}); response.end(JSON.stringify(value));
});
const terminalServer = new WebSocketServer({noServer:true});
const terminalInput=[];
server.on('upgrade',(request,socket,head)=>terminalServer.handleUpgrade(request,socket,head,ws=>{ws.send('\x1b[32mWorkspace shell ready\x1b[0m\r\n> ');ws.on('message',data=>{if(data.toString().startsWith('{"resize":'))return;terminalInput.push(data.toString());ws.send(data);});}));
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const browser = await chromium.launch(process.env.SPAWNT_TEST_BROWSER ? {executablePath:process.env.SPAWNT_TEST_BROWSER} : {});
try {
  const page = await browser.newPage({ viewport:{width:1440,height:1000} });
  await page.addInitScript(() => { window.spawntDesktop = {chooseWorkspace:async () => 'C:\\workspace\\picked'}; });
  page.on('pageerror', error => errors.push(error.message));
  await page.goto(`http://127.0.0.1:${server.address().port}/?token=test#sessions`);
  assert.equal(await page.getByRole('button',{name:'Pin navigation',exact:true}).count(),0,'navigation has a single predictable toggle');
  await page.getByRole('button', {name:/Review authentication/}).click();
  await page.locator('#chat-stream').getByText('I will review the login flow.').waitFor();
  await page.waitForFunction(() => document.getElementById('remote-status').textContent.includes('Waiting for the code'));
  assert.equal(await page.locator('#remote-connection-state').isVisible(),false,'initial connection status must not create a toast');
  remotePaired = true;
  await page.locator('#remote-connection-state').waitFor({state:'visible'});
  assert.doesNotMatch(await page.locator('#chat-stream').textContent(), /Command output/);
  assert.equal(await page.locator('#chat-status').textContent(), '2 messages', 'hidden tool events are not counted as conversation messages');
  await page.getByRole('button',{name:'Terminal',exact:true}).click();
  await page.locator('#workspace-terminal-pane').waitFor({state:'visible'});
  await page.locator('#workspace-terminal-screen .xterm-screen').waitFor();
  await page.locator('#workspace-terminal-screen .xterm-helper-textarea').pressSequentially('pwd');
  await page.waitForTimeout(150);
  assert.match(terminalInput.join(''), /pwd/, 'integrated terminal forwards real typing');
  assert.ok(writes.some(write=>write.path.endsWith('/terminal')),'shell launch resolves the current conversation workspace');
  await page.getByRole('button',{name:'Hide',exact:true}).click();
  assert.equal(await page.locator('#chat-model-select').inputValue(), 'gpt-6.1-sol', 'model picker shows the running model');
  assert.equal(await page.locator('#chat-model-status').textContent(), '', 'confirmed model is not repeated beside its selector');
  await page.locator('#chat-stream h2').getByText('Findings').waitFor();
  assert.equal(await page.locator('#chat-stream pre code').textContent(), 'let safe = "<script>";');
  const userBox = await page.locator('.sp-chat-user').boundingBox();
  const agentBox = await page.locator('.sp-chat-assistant').boundingBox();
  assert.ok(userBox.x > agentBox.x, 'user bubbles align right of agent replies');
  const compactHeight = await page.locator('#chat-prompt-text').evaluate(element => element.clientHeight);
  await page.locator('#chat-prompt-text').fill(Array.from({length:30}, () => 'A long prompt line').join('\n'));
  const expandedHeight = await page.locator('#chat-prompt-text').evaluate(element => element.clientHeight);
  assert.ok(expandedHeight > compactHeight && expandedHeight <= 160, 'composer grows to its bounded height');
  await page.locator('#chat-prompt-text').fill('Composing text');
  await page.locator('#chat-prompt-text').dispatchEvent('keydown', {key:'Enter',isComposing:true});
  assert.equal(writes.filter(write => write.path.endsWith('/prompt')).length, 0, 'IME Enter must not submit a draft');
  await page.locator('#chat-prompt-text').fill('Keyboard prompt');
  await page.locator('#chat-prompt-text').press('Shift+Enter');
  assert.equal(await page.locator('#chat-prompt-text').inputValue(), 'Keyboard prompt\n');
  await page.locator('#chat-prompt-text').press('Enter');
  await page.waitForFunction(() => document.getElementById('chat-prompt-text').value === '');
  assert.deepEqual(writes.at(-1).body, {text:'Keyboard prompt'});
  await page.locator('#chat-prompt-text').fill('Review auth\nThen run tests');
  await page.locator('#chat-send-btn').click();
  await page.waitForFunction(() => document.getElementById('chat-prompt-text').value === '');
  assert.deepEqual(writes.filter(write => write.path.endsWith('/prompt')).at(-1).body, {text:'Review auth\nThen run tests'});
  await page.route('**/api/v1/sessions/*/prompt', route => route.fulfill({status:502,contentType:'application/json',body:'{}'}));
  await page.locator('#chat-prompt-text').fill('Keep my draft');
  await page.locator('#chat-prompt-text').press('Enter');
  await page.getByText(/Could not send\./).waitFor();
  assert.equal(await page.locator('#chat-prompt-text').inputValue(), 'Keep my draft');
  assert.equal(await page.locator('#chat-send-btn').isDisabled(), false);
  await page.waitForTimeout(2300);
  assert.match(await page.locator('#app-notice-message').textContent(), /Could not send\./, 'polling must preserve the error toast');
  assert.equal(await page.locator('#chat-delivery-status').textContent(), '');
  await page.locator('[data-action="dismiss-notice"]').click();
  assert.equal(await page.locator('#app-notice').isHidden(), true);
  await page.unroute('**/api/v1/sessions/*/prompt');
  await page.locator('#chat-model-select').selectOption('o3');
  await page.getByRole('heading',{name:'Model handoff',exact:true}).waitFor();
  assert.doesNotMatch(await page.locator('#chat-session-label').textContent(), /codex-switched/);
  assert.deepEqual(writes.find(write => write.path.endsWith('/switch')).body, {target_model:'o3',agent:'codex'});
  assert.equal(await page.locator('#chat-prompt-text').inputValue(), 'Keep my draft', 'an explicit model handoff carries the unsent draft');
  await page.getByRole('button', {name:/Review authentication/}).click();
  assert.equal(await page.locator('#chat-prompt-text').inputValue(), '', 'handoff moves the unsent draft to the target session');
  await page.locator('#chat-prompt-text').fill('Keep my draft');
  let finishSend;
  const sendStarted = new Promise(resolve => {
    page.route('**/api/v1/sessions/*/prompt', route => { finishSend = () => route.fulfill({status:200,contentType:'application/json',body:'{}'}); resolve(); });
  });
  await page.locator('#chat-send-btn').click();
  await sendStarted;
  await page.locator('.sp-chat-user').filter({hasText:'Keep my draft'}).waitFor();
  await page.locator('.sp-chat-waiting').getByText('Sending...').waitFor();
  await page.locator('#chat-model-select').selectOption('o3');
  await page.getByRole('heading',{name:'Model handoff',exact:true}).waitFor();
  await page.locator('#chat-prompt-text').fill('New session draft');
  await finishSend();
  assert.equal(await page.locator('#chat-prompt-text').inputValue(), 'New session draft', 'old delivery cannot clear the new session draft');
  assert.equal(await page.locator('#chat-send-btn').isDisabled(), false, 'old pending prompt cannot block a different session');
  await page.unroute('**/api/v1/sessions/*/prompt');
  await page.getByRole('button', {name:'New conversation',exact:true}).click();
  await page.locator('#launch-agent-select').selectOption('claude');
  assert.equal(await page.locator('#launch-agent-btn').isDisabled(), true);
  assert.match(await page.locator('#launch-agent-hint').textContent(), /npm install/);
  await page.locator('#launch-agent-select').selectOption('terminal');
  assert.equal(await page.locator('#launch-model-input').isDisabled(), true);
  await page.locator('#launch-agent-select').selectOption('codex');
  await page.getByRole('button', {name:'Choose folder',exact:true}).click();
  assert.equal(await page.locator('#launch-repo-input').inputValue(),'C:\\workspace\\picked');
  await page.evaluate(() => {window.spawntDesktop.chooseWorkspace = async () => null;});
  await page.getByRole('button', {name:'Choose folder',exact:true}).click();
  assert.equal(await page.locator('#launch-repo-input').inputValue(),'C:\\workspace\\picked','cancelling the folder dialog preserves the chosen workspace');
  await page.locator('#launch-agent-btn').click();
  await page.getByRole('heading',{name:'Picked workspace',exact:true}).waitFor();
  assert.equal(writes.find(write => write.path.endsWith('/launch')).body.agent, 'codex');
  assert.equal(await page.locator('#chat-terminal-view').isHidden(), true, 'agent launch keeps conversation visible');
  assert.equal(await page.locator('#chat-stream').isVisible(), true);
  await mkdir(new URL('../.tmp/ide-verification/', import.meta.url), {recursive:true});
  for (const width of [1440,390]) {
    await page.setViewportSize({width,height:1000});
    if (width === 390) {
      if (await page.locator('#chat-list-toggle').getAttribute('aria-expanded') === 'true') await page.locator('#chat-list-toggle').click();
      await page.locator('#chat-list-toggle').evaluate(element => element.blur());
      assert.equal(await page.locator('#ide-session-sidebar').isVisible(), false, 'mobile drawer closes without reopening itself');
    }

    assert.equal(await page.locator('#chat-stream').evaluate(element => [...element.querySelectorAll('.sp-chat-body,.sp-chat-body pre,.sp-chat-table')].every(node => node.scrollWidth <= node.clientWidth + 1)), true, `bubble content fits at ${width}px`);
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true, `no horizontal overflow at ${width}px`);
    await page.locator('#chat-stream').evaluate(element => element.scrollTop = 0);
    await page.locator('#remote-connection-state').waitFor({state:'hidden'});
    await page.screenshot({path:new URL(`../.tmp/ide-verification/chat-${width}.png`, import.meta.url).pathname.replace(/^\/([A-Za-z]:)/,'$1'),fullPage:true});
    await page.evaluate(() => document.documentElement.dataset.theme = 'dark');
    await page.waitForTimeout(220);
    assert.equal(await page.locator('#chat-list-toggle').evaluate(element => getComputedStyle(element).color), 'rgb(246, 247, 248)', 'dark theme controls use readable foreground');

    await page.screenshot({path:new URL(`../.tmp/ide-verification/chat-dark-${width}.png`, import.meta.url).pathname.replace(/^\/([A-Za-z]:)/,'$1'),fullPage:true});
    await page.evaluate(() => delete document.documentElement.dataset.theme);
  }
  await page.setViewportSize({width:1440,height:1000});
  await page.locator('.sp-console-nav').hover();await page.waitForTimeout(200);
  assert.equal(await page.locator('.sp-console-nav').evaluate(node=>Math.round(node.getBoundingClientRect().width)),208,'hover opens the rail');
  await page.mouse.move(1000,500);await page.waitForTimeout(200);
  assert.equal(await page.locator('.sp-console-nav').evaluate(node=>Math.round(node.getBoundingClientRect().width)),64,'pointer leave collapses the rail');
  await page.locator('#tab-btn-settings').click();
  await page.evaluate(() => {document.documentElement.dataset.theme = 'dark'; document.activeElement?.blur();});
  await page.mouse.move(1000, 500);
  await page.waitForTimeout(250);
  await page.screenshot({path:new URL('../.tmp/ide-verification/profile-dark-1440.png', import.meta.url).pathname.replace(/^\/([A-Za-z]:)/,'$1'),fullPage:true});
  await page.evaluate(() => {const status={enabled:true,paired:false,room_id:'a'.repeat(32),public_key:'b'.repeat(64),relay_url:'wss://relay.example/relay',pairing_code:'63311530'};showRemoteStatus(status);document.getElementById('desktop-pairing-code').focus();document.getElementById('desktop-pairing-code').select();window.pairingNode=document.getElementById('desktop-pairing-code');showRemoteStatus(status);});
  assert.equal(await page.locator('#desktop-pairing-code').evaluate(node=>node===window.pairingNode && node.selectionEnd-node.selectionStart===8),true,'status polling preserves code selection for copying');
  await page.route('**/api/v1/models', route => route.fulfill({status:200,contentType:'application/json',body:'[]'}));
  await page.locator('#tab-btn-sessions').click();
  await page.reload();
  await page.getByRole('button', {name:/Review authentication/}).click();
  await page.locator('#chat-stream').getByText('I will review the login flow.').waitFor();
  assert.equal(await page.locator('#chat-model-select option[value="o3"]').count(),0,'missing CLI metadata cannot produce guessed model names');
  assert.ok(await page.locator('#chat-model-select option[data-model="@default"]').count()>0,'installed agents remain switchable through their actual CLI defaults');
  assert.equal(await page.locator('#chat-switch-btn').isDisabled(), true);
  await page.evaluate(async () => {
    const {updateChat} = await import('/console-presentation.js');
    const stream = document.getElementById('chat-stream');
    const events = Array.from({length:40}, (_, i) => ({role:i % 2 ? 'assistant' : 'user',content:`Message ${i}`,created_at:'2026-10-29'}));
    updateChat(stream, events);
    stream.scrollTop = 100;
    updateChat(stream, [...events, {role:'assistant',content:'New reply',created_at:'2026-10-29'}]);
    if (stream.scrollTop !== 100) throw new Error('New replies moved the reader away from earlier messages');
    stream.scrollTop = stream.scrollHeight;
    updateChat(stream, [...events, {role:'assistant',content:'Latest reply',created_at:'2026-10-29'}]);
    if (stream.scrollHeight - stream.scrollTop - stream.clientHeight > 2) throw new Error('Latest replies did not stay in view');
  });
  if (process.env.SPAWNT_CAPTURE_ALL) {
    await mkdir(new URL('../.tmp/interface-review/', import.meta.url), {recursive:true});
    for (const width of [1440,390]) for (const theme of ['dark','light']) {
      await page.setViewportSize({width,height:900});
      await page.evaluate(theme => { delete document.documentElement.dataset.theme; while ((document.documentElement.dataset.theme || 'system') !== theme) cycleTheme(); },theme);
      for (const view of (process.env.SPAWNT_CAPTURE_VIEW ? [process.env.SPAWNT_CAPTURE_VIEW] : ['overview','sessions','usage','limits','projects','automations','settings','auth'])) {
        await page.evaluate(view => { location.hash = view; }, view);
        await page.waitForTimeout(300);
        await page.evaluate(() => document.activeElement?.blur());
        await page.mouse.move(width - 20, 450);
        await page.waitForTimeout(220);
        if (await page.evaluate(() => document.documentElement.scrollWidth > innerWidth)) console.log(await page.evaluate(() => ({width:innerWidth,document:document.documentElement.scrollWidth,offenders:[...document.querySelectorAll('.view-content.active *')].filter(node => {if(node.getBoundingClientRect().right <= innerWidth || getComputedStyle(node).display === 'none')return false;for(let parent=node.parentElement;parent&&parent!==document.body;parent=parent.parentElement)if(['auto','scroll','hidden'].includes(getComputedStyle(parent).overflowX))return false;return true;}).map(node => ({tag:node.tagName,id:node.id,cls:node.className,right:node.getBoundingClientRect().right,width:node.getBoundingClientRect().width,parent:node.parentElement.className,text:node.textContent.slice(0,40)}))})));

        assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth),true, `native ${view}/${theme}/${width} page overflow`);
        if (view === 'auth') assert.equal(await page.locator('#console-title').textContent(),'Account');
        if (view === 'automations') for (const id of ['relay-policy-select','relay-strategy-select']) assert.ok(await page.locator('#'+id).inputValue(), 'relay selector keeps a valid choice');
        const text = await page.locator('.view-content.active').textContent();
        assert.doesNotMatch(text, /undefined|NaN/, `${view} has invalid numeric state`);
        await page.screenshot({path:new URL(`../.tmp/interface-review/native-${view}-${theme}-${width}.png`,import.meta.url).pathname.replace(/^\/([A-Za-z]:)/,'$1'),fullPage:true});
      }
      await page.evaluate(() => { void confirmAction('Permanently delete this local chat history? This cannot be undone.', 'Delete history'); });
      await page.locator('#confirm-dialog').waitFor({state:'visible'});
      assert.equal(await page.locator('#confirm-dialog').evaluate(element => element.contains(document.activeElement)),true,'dialog owns focus');
      await page.screenshot({path:new URL(`../.tmp/interface-review/native-confirm-${theme}-${width}.png`,import.meta.url).pathname.replace(/^\/([A-Za-z]:)/,'$1'),fullPage:true});
      await page.keyboard.press('Escape');
      assert.equal(await page.locator('#confirm-dialog').isVisible(),false,'Escape dismisses confirmation');
    }
    console.log('PASS: all eight native views and confirmation dialog at desktop/phone sizes in both themes');
  }
  for (const width of [180,240,320,390,1440]) {
    const overflow = await page.evaluate(async width => {
      const {renderActivity}=await import('/console-presentation.js');
      const host=document.createElement('div');host.style.cssText=`position:fixed;left:0;top:0;width:${width}px;z-index:1000`;
      host.innerHTML=renderActivity({range_label:'All time',activity_interval:'week',total_tokens:840,daily:Array.from({length:84},(_,i)=>({date:'2025-12-29',total_tokens:10,chart_pct:50,turns:1}))});
      document.body.append(host);
      const result=[host,...host.querySelectorAll('.sp-activity,.sp-activity-bars,.sp-activity-grid')].some(node=>node.scrollWidth>node.clientWidth+1);
      host.remove();return result;
    },width);
    assert.equal(overflow,false,`activity must fit without horizontal scrolling at ${width}px`);
  }
  console.log('PASS: maximum activity intervals fit 180–1440px without horizontal scrolling');
  assert.deepEqual(errors, []);
  console.log('PASS: session selection, clean chat, multiline prompt, model handoff, installation hints, launcher and responsive layout');
} finally { await browser.close(); await new Promise(resolve => server.close(resolve)); }
