import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdtemp, mkdir, readFile, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { createServer } from 'node:net';

const binary = path.resolve(process.env.SPAWNT_BINARY || 'target/release/spawnt.exe');
const home = await mkdtemp(path.join(tmpdir(), 'spawnt-ide-check-'));
const env = {...process.env, SPAWNT_HOME:home, HOME:home, USERPROFILE:home, CLAUDE_CONFIG_DIR:path.join(home,'claude'), CODEX_HOME:path.join(home,'codex'), GEMINI_HOME:path.join(home,'gemini')};
await mkdir(env.CLAUDE_CONFIG_DIR); await mkdir(env.CODEX_HOME); await mkdir(env.GEMINI_HOME);
if (process.env.SPAWNT_TEST_NPM_SHIM === '1') {
  const prefix=path.join(home,'npm'); const pkg=path.join(prefix,'node_modules','@openai','codex');
  await mkdir(pkg,{recursive:true});
  await writeFile(path.join(prefix,'codex.cmd'),'@echo off\r\n');
  await writeFile(path.join(pkg,'package.json'),JSON.stringify({bin:{codex:'cli.js'}}));
  await writeFile(path.join(pkg,'cli.js'),'let input=""; if(process.stdin.isTTY) process.stdin.setRawMode(true); process.stdin.on("data", data => { input+=data.toString(); if(/[\\r\\n]/.test(input)) { console.log("RECEIVED " + input.trim()); input=""; } }); console.log("SPAWNT_NPM_READY");');
  const pathKey=Object.keys(env).find(key=>key.toLowerCase()==='path') || 'PATH';
  env[pathKey]=prefix+path.delimiter+env[pathKey];
}
const reservation=createServer();
await new Promise(resolve=>reservation.listen(0,'127.0.0.1',resolve));
const port=reservation.address().port;
await new Promise(resolve=>reservation.close(resolve));
const daemon = spawn(binary, ['daemon','--port',String(port)], {env, windowsHide:true, stdio:['ignore','pipe','pipe']});
let daemonLog='';
daemon.stdout.on('data',data=>{daemonLog+=data;}); daemon.stderr.on('data',data=>{daemonLog+=data;});
let hosted;
let socket;
try {
  let info;
  for (let attempt=0; attempt<100; attempt++) {
    try { info=JSON.parse(await readFile(path.join(home,'daemon.json'),'utf8')); break; } catch { await new Promise(resolve=>setTimeout(resolve,100)); }
  }
  assert.ok(info?.port, `daemon publishes its actual loopback port: ${daemonLog}`);
  const base=`http://127.0.0.1:${info.port}`;
  const headers={Authorization:`Bearer ${info.token}`, 'Content-Type':'application/json'};
  async function api(route, body) {
    const response=await fetch(base+route,{method:body?'POST':'GET',headers,body:body?JSON.stringify(body):undefined});
    assert.equal(response.status,200,route); return response.json();
  }
  const agent=process.env.SPAWNT_TEST_NPM_SHIM === '1' ? 'codex' : 'terminal';
  hosted=await api('/api/v1/sessions/launch',{agent,repo_path:home});
  assert.equal(hosted.agent,agent);
  socket=new WebSocket(`${base.replace('http:','ws:')}/api/v1/sessions/${hosted.session_id}/pty?token=${encodeURIComponent(info.token)}`);
  socket.binaryType='arraybuffer';
  let output=''; socket.onmessage=event=>{output+=new TextDecoder().decode(event.data);};
  await new Promise((resolve,reject)=>{socket.onopen=resolve; socket.onerror=reject; setTimeout(()=>reject(Error('PTY proxy timeout')),10000).unref();});
  if(agent==='codex') for(let attempt=0;attempt<100 && !output.includes('SPAWNT_NPM_READY');attempt++) await new Promise(resolve=>setTimeout(resolve,100));
  const shellCommand=agent==='codex' ? 'SPAWNT_IDE_SMOKE' : process.platform==='win32' ? 'Write-Output ("SPAWNT_" + "IDE_SMOKE")' : 'printf SPAWNT_\'IDE_SMOKE\\n\'';
  await api(`/api/v1/sessions/${hosted.session_id}/prompt`,{text:shellCommand});
  const expected=agent==='codex' ? 'RECEIVED SPAWNT_IDE_SMOKE' : 'SPAWNT_IDE_SMOKE';
  for(let attempt=0; attempt<100 && !output.includes(expected); attempt++) await new Promise(resolve=>setTimeout(resolve,100));
  assert.ok(output.includes(expected), `child executes the submitted prompt and streams binary output: ${output}`);
  const installed=await api('/api/v1/agents/installed');
  assert.equal(installed.agents.find(agent=>agent.id==='terminal').installed,true);
  socket.close();
  daemon.kill();
  await new Promise(resolve=>setTimeout(resolve,300));
  const health=await fetch(`http://127.0.0.1:${hosted.port}/health`,{headers:{Authorization:`Bearer ${hosted.token}`}});
  assert.equal(health.status,200,'PTY owner survives daemon shutdown');
  if(agent==='codex') assert.match(output,/SPAWNT_NPM_READY/, 'npm entry point launches through Node');
  console.log(`PASS: fresh binary launches ${agent}, submits input, streams output and preserves the PTY after daemon shutdown`);
} finally {
  socket?.close(); daemon.kill();
  if(hosted) await fetch(`http://127.0.0.1:${hosted.port}/stop`,{method:'POST',headers:{Authorization:`Bearer ${hosted.token}`}}).catch(()=>{});
}
