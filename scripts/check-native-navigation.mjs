import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
const require=createRequire(new URL('../web/package.json',import.meta.url));
const ts=require('typescript');
const html=readFileSync(new URL('../crates/daemon/src/console.html',import.meta.url),'utf8');
const source=readFileSync(new URL('../crates/daemon/src/console-app.js',import.meta.url),'utf8');
const file=ts.createSourceFile('console.js',source,ts.ScriptTarget.Latest,true,ts.ScriptKind.JS);
const fn=name=>file.statements.filter(ts.isFunctionDeclaration).find(node=>node.name?.text===name)?.getText(file)||'';
const window={location:new URL('http://127.0.0.1:41738/?token=unit-local-token#overview')};
const document={body:{dataset:{}},getElementById:()=>({setAttribute(){}}),querySelectorAll:()=>[]};
const history={pushState(_state,_title,url){window.location=new URL(url,window.location);}};
const navigate=new Function('document','window','history','URL',`let activeView;const activeChatSession='';${fn('consoleViewUrl')}\n${fn('setView')}\nreturn setView;`)(document,window,history,URL);
for(const view of ['sessions','settings','overview']) {
 navigate(view);
 assert.equal(window.location.searchParams.get('token'),'unit-local-token',`navigation to ${view} must preserve local authorization across reload`);
}
const nodes=new Map();
const errorDocument={getElementById(id){if(!nodes.has(id))nodes.set(id,{textContent:'Loading daily activity...',hidden:true});return nodes.get(id);}};
const load=new Function('document','fetch',`const headers={};const currentTimeline='all';let lastReportJson='';const summaryState={};const refreshSummary=async()=>{};const console={error(){}};${fn('apiJson')}\n${fn('showConnectionError')}\n${fn('loadReport')}\nreturn loadReport;`)(errorDocument,async()=>({ok:false,status:401,json:async()=>({})}));
assert.equal(await load(),false);
assert.doesNotMatch(nodes.get('daily-activity')?.textContent||'Loading',/Loading/,'failed requests must resolve the loading state');
assert.match(nodes.get('connection-status')?.textContent||'',/authoriz|reopen|tray/i,'missing authorization needs an actionable message');
const cacheDeclarations=file.statements.filter(ts.isVariableStatement).filter(node=>node.declarationList.declarations.some(declaration=>/^last(Report|Sessions|Quota)Json$/.test(declaration.name.getText(file)))).map(node=>node.getText(file)).join('\n');
let failQuota=false;
const quotaNodes=new Map();
const quotaDocument={getElementById(id){if(!quotaNodes.has(id))quotaNodes.set(id,{set textContent(value){this.innerHTML=value;}});return quotaNodes.get(id);}};
const quotaLoad=new Function('document','fetch',`const headers={};const summaryState={};const refreshSummary=async()=>{};${cacheDeclarations}\n${fn('apiJson')}\n${fn('loadQuota').replace("import('/console-presentation.js')", "Promise.resolve({updateQuota:(element)=>{element.innerHTML='<div>quota loaded</div>'}})")}\nreturn loadQuota;`)(quotaDocument,async()=>({ok:!failQuota,status:failQuota?503:200,json:async()=>[]}));
assert.equal(await quotaLoad(),true,'a healthy quota response must not fail on undeclared render caches');
failQuota=true;
assert.equal(await quotaLoad(),false);
failQuota=false;
assert.equal(await quotaLoad(),true);
assert.equal(quotaNodes.get('quota-table').innerHTML,'<div>quota loaded</div>');
for(const name of ['lastReportJson','lastSessionsJson','lastQuotaJson']) assert.match(cacheDeclarations,new RegExp(`\\b${name}\\b`),`${name} needs an initialized render cache`);
console.log('PASS: native authorization, loading recovery, initialized caches and successful quota retry');

const accountNodes=new Map();
const accountDocument={getElementById(id){if(!accountNodes.has(id))accountNodes.set(id,{hidden:false,style:{},value:'unit-local-pairing',focus(){}});return accountNodes.get(id);}};
let accountPaired=true,accountFailed=false;
const checkAccount=new Function('document','apiJson',`const headers={};${fn('checkAuthSession')}return checkAuthSession;`)(accountDocument,async url=>{if(accountFailed)throw new Error('offline');return url.includes('auth/session')?{paired:accountPaired,email:'unit@example.invalid'}:{message:'Ready',last_success_at:null};});
assert.equal(await checkAccount(),true);assert.equal(accountNodes.get('auth-unpaired-card').hidden,true);
accountPaired=false;assert.equal(await checkAccount(),true);assert.equal(accountNodes.get('auth-paired-card').hidden,true);
accountFailed=true;assert.equal(await checkAccount(),false);assert.doesNotMatch(accountNodes.get('auth-cloud-status').textContent,/Checking/);
let requests=0,finishPair;
const pair=new Function('document','fetch','checkAuthSession',`const headers={};let accountWriteBusy=false;${fn('pairTokenDirectly')}return pairTokenDirectly;`)(accountDocument,()=>{requests++;return new Promise(resolve=>finishPair=resolve);},async()=>true);
const pendingPair=pair();await pair();assert.equal(requests,1,'concurrent pairing must be ignored');
finishPair({ok:true});await pendingPair;assert.equal(accountNodes.get('auth-token-input').value,'');assert.equal(accountNodes.get('auth-pair-button').disabled,false);
console.log('PASS: account connected/disconnected/error states and duplicate pairing prevention without reload');

const presentationSource=readFileSync(new URL('../web/lib/console-presentation.js',import.meta.url),'utf8');
const {reconcileChat}=await import(`data:text/javascript;base64,${Buffer.from(presentationSource).toString('base64')}`);
for (const deleted of [false,true]) {
 const input={value:'Hi',focus(){}},button={disabled:false},status={textContent:''};
 const promptDocument={getElementById:id=>id==='chat-prompt-text'?input:id==='chat-send-btn'?button:status};
 const optimisticPrompts=new Map(),chatDeletionGenerations=new Map(),chatDrafts=new Map([['sample','Hi']]);
 let acknowledge;
 const submit=new Function('document','fetch','optimisticPrompts','chatDeletionGenerations','chatDrafts','setTimeout',`
  const activeChatSession='sample',chatEvents=[],headers={},pendingPrompts=new Set();
  const renderCurrentChat=()=>{},resizePrompt=()=>{},loadChat=()=>{},showNotice=message=>{throw new Error(message)};
  ${fn('sendPrompt')}
  return sendPrompt;
 `)(promptDocument,()=>new Promise(resolve=>{acknowledge=resolve;}),optimisticPrompts,chatDeletionGenerations,chatDrafts,()=>{});
 const submission=submit();
 assert.equal(button.disabled,true);
 if (deleted) {
  chatDeletionGenerations.set('sample',1);
  optimisticPrompts.delete('sample');
 } else {
  const recorded=[{event_id:'user',role:'user',content:'Hi'},{event_id:'reply',role:'assistant',content:'Hello'}];
  optimisticPrompts.set('sample',reconcileChat(recorded,optimisticPrompts.get('sample'),'idle').pending);
  assert.equal(optimisticPrompts.get('sample').length,0,'a complete recorded reply reconciles before the HTTP acknowledgement');
 }
 acknowledge({ok:true});
 await submission;
 assert.equal(input.value,deleted?'Hi':'',deleted?'deletion cancels draft clearing by the outstanding acknowledgement':'a successful acknowledgement clears the unchanged draft even after reconciliation');
 assert.equal(chatDrafts.has('sample'),deleted,'only a non-deleted successful send clears its saved draft');
 assert.equal(button.disabled,false);
}
console.log('PASS: delayed prompt acknowledgements distinguish completed replies from deleted history');
