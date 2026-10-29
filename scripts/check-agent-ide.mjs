import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile, mkdir } from 'node:fs/promises';
import { createRequire } from 'node:module';

// Run with Playwright available on NODE_PATH; exercises the shipped console against controlled API replies.
const { chromium } = createRequire(import.meta.url)('playwright');
const writes = [];
const errors = [];
const session = { id:'codex:sample', title:'Review authentication', agent:'codex', state:'idle', last_event_at:new Date().toISOString() };
const staticFiles = {
  '/':'crates/daemon/src/console.html', '/console-app.js':'crates/daemon/src/console-app.js',
  '/console-presentation.js':'web/lib/console-presentation.js', '/spawnt-console.css':'web/app/spawnt-console.css',
  '/spawnt-tokens.css':'web/app/spawnt-tokens.css', '/spawnt-mark.svg':'web/public/spawnt-mark.svg',
};
const server = createServer(async (request, response) => {
  const url = new URL(request.url, 'http://localhost');
  let body = ''; for await (const chunk of request) body += chunk;
  if (request.method === 'POST') writes.push({ path:url.pathname, body:body ? JSON.parse(body) : null });
  let value = {};
  if (url.pathname === '/api/v1/sessions') value = { sessions:[session], page:1, total_pages:1, total_count:1 };
  else if (url.pathname === '/api/v1/sessions/hosted') value = { sessions:[] };
  else if (url.pathname === '/api/v1/projects') value = [];
  else if (url.pathname === '/api/v1/models') value = [{ model:'o3', agent:'codex', display_name:'OpenAI o3' }];
  else if (url.pathname.endsWith('/chat')) value = [
    { event_id:'1', turn_index:1, role:'user', content:'Review authentication', created_at:session.last_event_at },
    { event_id:'2', turn_index:2, role:'assistant', content:'I will review the login flow.', created_at:session.last_event_at },
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
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const browser = await chromium.launch();
try {
  const page = await browser.newPage({ viewport:{width:1440,height:1000} });
  page.on('pageerror', error => errors.push(error.message));
  await page.goto(`http://127.0.0.1:${server.address().port}/?token=test#sessions`);
  await page.getByRole('button', {name:/Review authentication/}).click();
  await page.locator('#chat-stream').getByText('I will review the login flow.').waitFor();
  assert.doesNotMatch(await page.locator('#chat-stream').textContent(), /Command output/);
  await page.locator('#chat-prompt-text').fill('Review auth\nThen run tests');
  await page.locator('#chat-send-btn').click();
  await page.waitForFunction(() => document.getElementById('chat-prompt-text').value === '');
  assert.deepEqual(writes.find(write => write.path.endsWith('/prompt')).body, {text:'Review auth\nThen run tests'});
  await page.locator('#chat-model-select').selectOption('o3');
  await page.locator('#chat-switch-btn').click();
  await page.waitForFunction(() => document.getElementById('chat-session-label').textContent.includes('codex-switched'));
  assert.deepEqual(writes.find(write => write.path.endsWith('/switch')).body, {target_model:'o3',agent:'codex'});
  await page.getByRole('button', {name:'+ New Session',exact:true}).click();
  await page.locator('#launch-agent-select').selectOption('claude');
  assert.equal(await page.locator('#launch-agent-btn').isDisabled(), true);
  assert.match(await page.locator('#launch-agent-hint').textContent(), /npm install/);
  await page.locator('#launch-agent-select').selectOption('terminal');
  assert.equal(await page.locator('#launch-model-input').isDisabled(), true);
  await page.locator('#launch-agent-select').selectOption('codex');
  await page.locator('#launch-repo-input').fill('C:\\workspace\\demo');
  await page.locator('#launch-agent-btn').click();
  await page.waitForFunction(() => document.getElementById('chat-session-label').textContent.includes('codex-launched'));
  assert.equal(writes.find(write => write.path.endsWith('/launch')).body.agent, 'codex');
  await page.getByRole('button', {name:'+ New Session',exact:true}).click();
  await mkdir(new URL('../.tmp/ide-verification/', import.meta.url), {recursive:true});
  for (const width of [1440,390]) {
    await page.setViewportSize({width,height:1000});
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true, `no horizontal overflow at ${width}px`);
    await page.screenshot({path:new URL(`../.tmp/ide-verification/launcher-${width}.png`, import.meta.url).pathname.replace(/^\/([A-Za-z]:)/,'$1'),fullPage:true});
  }
  await page.route('**/api/v1/models', route => route.fulfill({status:200,contentType:'application/json',body:'[]'}));
  await page.reload();
  await page.getByRole('button', {name:/Review authentication/}).click();
  await page.locator('#chat-stream').getByText('I will review the login flow.').waitFor();
  assert.equal(await page.locator('#chat-model-select').isDisabled(), true, 'missing CLI metadata cannot produce a guessed model list');
  assert.equal(await page.locator('#chat-switch-btn').isDisabled(), true);
  assert.deepEqual(errors, []);
  console.log('PASS: session selection, clean chat, multiline prompt, model handoff, installation hints, launcher and responsive layout');
} finally { await browser.close(); await new Promise(resolve => server.close(resolve)); }
