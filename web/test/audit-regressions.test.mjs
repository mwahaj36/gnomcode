import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {createRequire} from 'node:module';
const require=createRequire(import.meta.url);
const ts=require('typescript');
const source=readFileSync(new URL('../../crates/daemon/src/console-app.js',import.meta.url),'utf8');
const ast=ts.createSourceFile('console.js',source,ts.ScriptTarget.Latest,true,ts.ScriptKind.JS);
const fn=name=>ast.statements.find(node=>ts.isFunctionDeclaration(node)&&node.name?.text===name)?.getText(ast);

test('failed model changes restore the confirmed model before another prompt',async()=>{
  const option={dataset:{model:'failed',agent:'antigravity'}};
  const select={value:'antigravity:failed',selectedOptions:[option],dataset:{currentSelection:'codex:current',currentModel:'current'},disabled:false};
  const nodes={'chat-model-select':select,'chat-switch-btn':{},'chat-send-btn':{},'chat-prompt-text':{value:'draft'}};
  const run=new Function('document',`const activeChatSession='chat',pendingPrompts=new Set();let modelHandoffBusy=false;const apiJson=async()=>{throw Error('busy')},showNotice=()=>{},showCurrentChatModel=()=>{};${fn('switchChatModel')};return switchChatModel;`)({getElementById:id=>nodes[id]});
  await run();
  assert.equal(select.value,'codex:current');
  assert.equal(nodes['chat-prompt-text'].value,'draft');
});

test('guardrail setup saves an alert rule without navigating away',async()=>{
 const values={'rule-agent':'codex','rule-spend':'5','rule-turns':'','rule-action':'Warn'};
 const nodes=Object.fromEntries(Object.entries(values).map(([id,value])=>[id,{value}]));
 nodes['rule-save']={disabled:false};nodes['rule-status']={textContent:''};let saved;
 const run=new Function('document','apiJson',`const headers={},loadProData=async()=>{};${fn('saveGuardrail')};return saveGuardrail;`)({getElementById:id=>nodes[id]},async(url,options)=>{saved={url,body:JSON.parse(options.body)};});
 await run();assert.equal(saved.url,'/api/v1/rules');assert.equal(saved.body.agent,'codex');assert.equal(saved.body.max_spend_usd,5);assert.equal(saved.body.max_turns,null);assert.equal(saved.body.action,'Warn');assert.match(nodes['rule-status'].textContent,/saved/i);
});
