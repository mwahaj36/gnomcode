import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {createRequire} from 'node:module';
import vm from 'node:vm';
const require=createRequire(import.meta.url),ts=require('typescript');
const code=()=>ts.transpileModule(readFileSync(new URL('../lib/push-policy.ts',import.meta.url),'utf8'),{compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022}}).outputText;
test('push subscriptions accept only vendor endpoints and bounded keys',()=>{
 const ctx={exports:{},URL};vm.runInNewContext(code(),ctx);
 const value={endpoint:'https://fcm.googleapis.com/wp/test',keys:{p256dh:'a'.repeat(87),auth:'b'.repeat(22)}};
 assert.ok(ctx.exports.notificationSubscription(value));
 for(const endpoint of ['http://fcm.googleapis.com/test','https://127.0.0.1/test','https://fcm.googleapis.com.evil.test/test','https://user@fcm.googleapis.com/test','https://fcm.googleapis.com:444/test'])assert.equal(ctx.exports.notificationSubscription({...value,endpoint}),null);
 assert.equal(ctx.exports.notificationSubscription({...value,keys:{p256dh:'bad',auth:'bad'}}),null);
});
