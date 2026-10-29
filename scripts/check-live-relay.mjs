import assert from 'node:assert/strict';
import { generateKeyPair, createChannel } from '../web/lib/remote-crypto.mjs';

const base=process.argv[2];
if(!base?.startsWith('https://')) throw Error('Usage: node scripts/check-live-relay.mjs https://<public-worker-host>');
const health=await fetch(`${base}/health`, {signal:AbortSignal.timeout(15000)});
assert.equal(health.status,200); assert.equal((await health.json()).ok,true);
const room=crypto.randomUUID().replaceAll('-','');
const desktopKeys=await generateKeyPair(), phoneKeys=await generateKeyPair();
const desktopChannel=await createChannel(desktopKeys,phoneKeys.publicHex,room,'desktop');
const phoneChannel=await createChannel(phoneKeys,desktopKeys.publicHex,room,'phone');
assert.equal(desktopChannel.verifyCode,phoneChannel.verifyCode);
const sockets=[];
try {
  for(const role of ['desktop','phone']) {
    const socket=new WebSocket(`${base.replace('https:','wss:')}/relay/${room}?role=${role}`);
    socket.binaryType='arraybuffer'; sockets.push(socket);
    await new Promise((resolve,reject)=>{
      const timer=setTimeout(()=>reject(Error('Relay connection timeout')),15000);
      socket.onopen=()=>{clearTimeout(timer);resolve();};
      socket.onerror=()=>{clearTimeout(timer);reject(Error('Relay WebSocket failed'));};
    });
  }
  async function transfer(from, to, sender, receiver, message) {
    const received=new Promise((resolve,reject)=>{
      const timer=setTimeout(()=>reject(Error('Relay delivery timeout')),10000);
      to.onmessage=event=>{clearTimeout(timer);resolve(event.data);};
    });
    const frame=await sender.seal(message);
    from.send(new TextEncoder().encode(JSON.stringify(frame)));
    const forwarded=JSON.parse(new TextDecoder().decode(await received));
    assert.deepEqual(forwarded,frame);
    assert.deepEqual(await receiver.open(forwarded),message);
    await assert.rejects(receiver.open(forwarded));
  }
  await transfer(sockets[0],sockets[1],desktopChannel,phoneChannel,{type:'confirm'});
  await transfer(sockets[1],sockets[0],phoneChannel,desktopChannel,{type:'confirm'});
  await transfer(sockets[0],sockets[1],desktopChannel,phoneChannel,{type:'chat',events:[{role:'assistant',content:'Synthetic relay check'}]});
  console.log('PASS: live public relay health, two peers, matching E2EE codes, bidirectional confirmation, encrypted chat and replay rejection');
} finally { sockets.forEach(socket=>socket.close()); }
// Cloudflare may keep closing WebSocket transports alive; assertions above have completed.
process.exit(0);
