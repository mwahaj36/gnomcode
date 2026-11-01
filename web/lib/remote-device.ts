import {generateKeyPair} from './remote-crypto.mjs';

export type DeviceOffer = {room:string;key:string;relay:string;protocol_version?:number;owner_id?:string;desktop_id?:string;request_id?:string};
export type SavedDevice = {offer:DeviceOffer; proofKey?:CryptoKey; enrollment?:Awaited<ReturnType<typeof generateKeyPair>>};

async function database() {
  return new Promise<IDBDatabase>((resolve,reject)=>{
    const request=indexedDB.open('spawnt-remote-device',1);
    request.onupgradeneeded=()=>request.result.createObjectStore('devices');
    request.onsuccess=()=>resolve(request.result);request.onerror=()=>reject(request.error);
  });
}
const recordKey=(owner:string,desktop:string)=>`${owner}:${desktop}`;

export async function savedDevice(owner:string,desktop?:string):Promise<SavedDevice|undefined> {
  const db=await database();
  try {return await new Promise((resolve,reject)=>{
    const store=db.transaction('devices').objectStore('devices');
    if(desktop){const request=store.get(recordKey(owner,desktop));request.onsuccess=()=>resolve(request.result);request.onerror=()=>reject(request.error);}
    else {const request=store.getAll();request.onsuccess=()=>resolve(request.result.find((saved:SavedDevice)=>saved.offer.owner_id===owner));request.onerror=()=>reject(request.error);}
  });} finally {db.close();}
}

async function save(saved:SavedDevice,current:()=>boolean=()=>true) {
  const {owner_id,desktop_id}=saved.offer;
  if(!owner_id || !desktop_id)throw new Error('Account device identity missing');
  const db=await database();
  try {if(!current())throw new Error('Connection changed');await new Promise<void>((resolve,reject)=>{
    const tx=db.transaction('devices','readwrite');tx.objectStore('devices').put(saved,recordKey(owner_id,desktop_id));
    tx.oncomplete=()=>resolve();tx.onerror=()=>reject(tx.error);tx.onabort=()=>reject(tx.error);
  });} finally {db.close();}
}

export async function enrollmentDevice(owner:string,desktop:string):Promise<SavedDevice> {
  const existing=await savedDevice(owner,desktop);
  if(existing)return existing;
  const enrollment=await generateKeyPair();
  const request_id=crypto.randomUUID().replaceAll('-','');
  const saved={offer:{owner_id:owner,desktop_id:desktop,request_id,room:request_id,key:'',relay:'',protocol_version:3},enrollment};
  await save(saved);return saved;
}

export async function rememberDevice(offer:DeviceOffer,secret:string,current:()=>boolean=()=>true) {
  if(!/^[a-f0-9]{64}$/.test(secret))throw new Error('Invalid device credential');
  const bytes=Uint8Array.from(secret.match(/../g)!,part=>parseInt(part,16));
  const proofKey=await crypto.subtle.importKey('raw',bytes,{name:'HMAC',hash:'SHA-256'},false,['sign']);bytes.fill(0);
  await save({offer,proofKey},current);return proofKey;
}

export async function forgetDevice(owner:string,desktop?:string) {
  const db=await database();
  try {await new Promise<void>((resolve,reject)=>{
    const tx=db.transaction('devices','readwrite'),store=tx.objectStore('devices');
    if(desktop)store.delete(recordKey(owner,desktop));
    else {const request=store.openCursor();request.onsuccess=()=>{const cursor=request.result;if(cursor){if(cursor.key==='desktop' || cursor.value.offer?.owner_id===owner)cursor.delete();cursor.continue();}};}
    tx.oncomplete=()=>resolve();tx.onerror=()=>reject(tx.error);tx.onabort=()=>reject(tx.error);
  });} finally {db.close();}
}

export async function deviceProof(key:CryptoKey,context:string) {
  const proof=await crypto.subtle.sign('HMAC',key,new TextEncoder().encode(context));
  return Array.from(new Uint8Array(proof),byte=>byte.toString(16).padStart(2,'0')).join('');
}
