type SavedDevice = {offer:{room:string;key:string;relay:string;protocol_version?:number}; proofKey:CryptoKey};

async function database() {
  return new Promise<IDBDatabase>((resolve,reject)=>{
    const request=indexedDB.open('spawnt-remote-device',1);
    request.onupgradeneeded=()=>request.result.createObjectStore('devices');
    request.onsuccess=()=>resolve(request.result);request.onerror=()=>reject(request.error);
  });
}

export async function savedDevice():Promise<SavedDevice|undefined> {
  const db=await database();
  try {return await new Promise((resolve,reject)=>{const request=db.transaction('devices').objectStore('devices').get('desktop');request.onsuccess=()=>resolve(request.result);request.onerror=()=>reject(request.error);});}
  finally {db.close();}
}

export async function rememberDevice(offer:SavedDevice['offer'], secret:string, current:()=>boolean=()=>true) {
  if(!/^[a-f0-9]{64}$/.test(secret))throw new Error('Invalid device credential');
  const bytes=Uint8Array.from(secret.match(/../g)!,part=>parseInt(part,16));
  const proofKey=await crypto.subtle.importKey('raw',bytes,{name:'HMAC',hash:'SHA-256'},false,['sign']);bytes.fill(0);
  const db=await database();
  try {if(!current())throw new Error('Connection changed');await new Promise<void>((resolve,reject)=>{const tx=db.transaction('devices','readwrite');tx.objectStore('devices').put({offer,proofKey},'desktop');tx.oncomplete=()=>resolve();tx.onerror=()=>reject(tx.error);tx.onabort=()=>reject(tx.error);});}
  finally {db.close();}
  return proofKey;
}

export async function forgetDevice() {
  const db=await database();
  try {await new Promise<void>((resolve,reject)=>{const tx=db.transaction('devices','readwrite');tx.objectStore('devices').delete('desktop');tx.oncomplete=()=>resolve();tx.onerror=()=>reject(tx.error);});}
  finally {db.close();}
}

export async function deviceProof(key:CryptoKey, context:string) {
  const proof=await crypto.subtle.sign('HMAC',key,new TextEncoder().encode(context));
  return Array.from(new Uint8Array(proof),byte=>byte.toString(16).padStart(2,'0')).join('');
}
