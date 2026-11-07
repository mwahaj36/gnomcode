import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {createRequire} from 'node:module';
import vm from 'node:vm';
const require=createRequire(import.meta.url), ts=require('typescript');
const js=ts.transpileModule(readFileSync(new URL('../lib/notifications.ts',import.meta.url),'utf8'),{compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022}}).outputText;
function load(serviceWorker){
 const sent=[];class Notification{static permission='granted';constructor(title){sent.push(title)}};
 const context={exports:{},require:()=>({supabase:{}}),window:{Notification},navigator:{serviceWorker},console,setTimeout,clearTimeout};
 vm.runInNewContext(js,context);return {api:context.exports,sent};
}
test('notification falls back when no service worker is registered',async()=>{
 const {api,sent}=load({ready:new Promise(()=>{}),getRegistration:async()=>undefined});
 const result=await Promise.race([api.sendAppNotification({title:'Test',silent:true}),new Promise(resolve=>setTimeout(()=>resolve('timed out'),100))]);
 assert.equal(result,true);assert.deepEqual(sent,['Test']);
});
test('notification uses a registered worker on mobile',async()=>{
 const sent=[];const worker={showNotification:async title=>sent.push(title)};
 const {api}=load({getRegistration:async()=>worker,ready:Promise.resolve(worker)});
 assert.equal(await api.sendAppNotification({title:'Mobile',silent:true}),true);assert.deepEqual(sent,['Mobile']);
});
