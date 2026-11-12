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
test('push notification request formats redacted event payload without chat content',()=>{
 const ctx={exports:{},URL};vm.runInNewContext(code(),ctx);
 assert.equal(typeof ctx.exports.parsePushNotificationRequest,'function');
 const generic=ctx.exports.parsePushNotificationRequest({event_id:'test-1',kind:'agent_event'});
 assert.equal(generic.tag,'test-1');
 assert.equal(generic.title,'Gnomcode');
 assert.equal(generic.body,'An agent needs your attention or finished its turn. Open Gnomcode for details.');
 const rich=ctx.exports.parsePushNotificationRequest({
  event_id:'approval:s1:123',
  kind:'agent_event',
  event:'approval_waiting',
  agent:'Claude Code',
  session_title:'Fix auth\nsecret chat content'
 });
 assert.equal(rich.title,'Gnomcode');
 assert.ok(rich.body.includes('Claude Code'));
 assert.ok(rich.body.includes('Needs approval'));
 assert.ok(!rich.body.includes('\n'));
 assert.equal(ctx.exports.parsePushNotificationRequest({event_id:'bad',kind:'agent_event',event:'unknown_event'}),null);
 assert.equal(ctx.exports.parsePushNotificationRequest({event_id:'bad id!',kind:'agent_event'}),null);
});
