import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
const require=createRequire(new URL('../web/package.json',import.meta.url));
const ts=require('typescript');
const html=readFileSync(new URL('../crates/daemon/src/console.html',import.meta.url),'utf8');
const source=[...html.matchAll(/<script(?:\s[^>]*)?>([\s\S]*?)<\/script>/g)].map(match=>match[1]).join('\n');
const file=ts.createSourceFile('console.js',source,ts.ScriptTarget.Latest,true,ts.ScriptKind.JS);
const fn=name=>file.statements.filter(ts.isFunctionDeclaration).find(node=>node.name?.text===name)?.getText(file)||'';
const window={location:new URL('http://127.0.0.1:41738/?token=unit-local-token#overview')};
const document={getElementById:()=>({setAttribute(){}}),querySelectorAll:()=>[]};
const history={pushState(_state,_title,url){window.location=new URL(url,window.location);}};
const navigate=new Function('document','window','history','URL',`let activeView;${fn('consoleViewUrl')}\n${fn('setView')}\nreturn setView;`)(document,window,history,URL);
for(const view of ['sessions','settings','overview']) {
 navigate(view);
 assert.equal(window.location.searchParams.get('token'),'unit-local-token',`navigation to ${view} must preserve local authorization across reload`);
}
const nodes=new Map();
const errorDocument={getElementById(id){if(!nodes.has(id))nodes.set(id,{textContent:'Loading daily activity...',hidden:true});return nodes.get(id);}};
const load=new Function('document','fetch',`const headers={};const currentTimeline='all';let lastReportJson='';const summaryState={};const refreshSummary=async()=>{};const console={error(){}};${fn('apiJson')}\n${fn('showConnectionError')}\n${fn('loadReport')}\nreturn loadReport;`)(errorDocument,async()=>({ok:false,status:401}));
assert.equal(await load(),false);
assert.doesNotMatch(nodes.get('daily-activity')?.textContent||'Loading',/Loading/,'failed requests must resolve the loading state');
assert.match(nodes.get('connection-status')?.textContent||'',/authoriz|reopen|tray/i,'missing authorization needs an actionable message');
console.log('PASS: native navigation preserves authorization and failed reports resolve loading with recovery guidance');
