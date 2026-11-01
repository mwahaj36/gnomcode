"use client";

import {useEffect,useState} from 'react';
import Link from 'next/link';
import {supabase} from '../lib/supabase';
import {enrollmentDevice,forgetDevice,type DeviceOffer} from '../lib/remote-device';

type Desktop={id:string;name:string;last_seen_at:string};
type Request={desktop_id:string;id:string;name:string;state:string;revoked:boolean;offer:DeviceOffer|null};

export default function DesktopDevices({onPair}:{onPair?:(offer:DeviceOffer)=>void}={}) {
  const [owner,setOwner]=useState<string|null>();
  const [desktops,setDesktops]=useState<Desktop[]>([]);
  const [requests,setRequests]=useState<Request[]>([]);
  const [status,setStatus]=useState('Checking your devices...');
  const [busy,setBusy]=useState(false);
  const [waiting,setWaiting]=useState('');
  useEffect(()=>{const {data:{subscription}}=supabase.auth.onAuthStateChange((_event,session)=>{setOwner(session?.user.id||null);setDesktops([]);setRequests([]);});return()=>subscription.unsubscribe();},[]);
  useEffect(()=>{
    if(!owner){if(owner===null)setStatus('Sign in with the same account as your desktop.');return;}
    let active=true,running=false;
    const controller=new AbortController();
    async function refresh(){
      if(document.hidden || running)return;running=true;
      try{
        const [hosts,clients]=await Promise.all([
          supabase.from('spawnt_desktops').select('id,name,last_seen_at').eq('user_id',owner!).abortSignal(AbortSignal.any([controller.signal,AbortSignal.timeout(10000)])),
          supabase.from('spawnt_device_requests').select('desktop_id,id,name,state,revoked,offer').eq('user_id',owner!).abortSignal(AbortSignal.any([controller.signal,AbortSignal.timeout(10000)])),
        ]);
        if(hosts.error || clients.error)throw new Error('Directory unavailable');
        if(active){setDesktops(hosts.data||[]);setRequests(clients.data||[]);setStatus(hosts.data?.length?'Select a desktop to open its conversations.':'Sign in to the native app to add your desktop.');}
      }catch{if(active)setStatus('Your device list is unavailable. Check your connection; the list will retry automatically.');}
      finally{running=false;}
    }
    void refresh();const timer=setInterval(()=>void refresh(),5000);
    return()=>{active=false;controller.abort();clearInterval(timer);};
  },[owner]);
  useEffect(()=>{
    if(!waiting || busy)return;
    const request=requests.find(row=>row.desktop_id===waiting && !row.revoked && row.state==='approved' && row.offer);
    const desktop=desktops.find(row=>row.id===waiting);
    if(request && desktop){setWaiting('');void connect(desktop);}
  },[requests,desktops,waiting,busy]);
  async function connect(desktop:Desktop){
    if(!owner || busy)return;setBusy(true);
    try{
      let saved=await enrollmentDevice(owner,desktop.id);
      const request=requests.find(row=>row.desktop_id===desktop.id && row.id===saved.offer.request_id);
      if(request?.revoked){await forgetDevice(owner,desktop.id);setStatus('Access was revoked. Select the desktop again to request approval.');return;}
      if(!request){
        if(!saved.enrollment){await forgetDevice(owner,desktop.id);saved=await enrollmentDevice(owner,desktop.id);}
        const {error}=await supabase.from('spawnt_device_requests').insert({user_id:owner,desktop_id:desktop.id,id:saved.offer.request_id,public_key:saved.enrollment!.publicHex,name:`${matchMedia('(pointer:coarse)').matches?'Phone':'Desktop'} browser ${saved.offer.request_id!.slice(-4)}`});
        if(error)throw error;
        setWaiting(desktop.id);setStatus('Access requested. Open Devices on your desktop or an approved device and select Approve.');return;
      }
      if(!request.offer || request.state!=='approved'){setWaiting(desktop.id);setStatus('Waiting for approval. Open Devices on your desktop or an approved device.');return;}
      const offer={...request.offer,owner_id:owner,desktop_id:desktop.id,request_id:request.id};
      if(onPair)onPair(offer);else{sessionStorage.setItem('spawnt-pairing-offer',JSON.stringify(offer));window.location.assign('/remote');}
    }catch{setStatus('Could not connect this device. Check your connection and retry.');}
    finally{setBusy(false);}
  }
  async function revoke(request:Request){
    if(!owner || busy)return;setBusy(true);
    try{const {error}=await supabase.from('spawnt_device_requests').update({revoked:true}).eq('user_id',owner).eq('desktop_id',request.desktop_id).eq('id',request.id);if(error)throw error;setRequests(current=>current.map(row=>row.desktop_id===request.desktop_id&&row.id===request.id?{...row,revoked:true}:row));setStatus('Access revoked. The desktop applies this when it next reaches the account service.');}
    catch{setStatus('Could not revoke access. Check your connection and retry.');}finally{setBusy(false);}
  }
  return <section className="web-profile-section" aria-label="Devices"><h2>Devices</h2><p role="status">{status}</p>
    {owner===null && <Link className="sp-console-action" href="/auth">Sign in</Link>}
    {desktops.map(desktop=><div className="sp-device-row" key={desktop.id}><div><strong>{desktop.name}</strong><p>Last seen {new Date(desktop.last_seen_at).toLocaleString()}</p></div><button type="button" className="sp-console-action" disabled={busy} onClick={()=>void connect(desktop)}>Open desktop</button></div>)}
    {requests.filter(request=>!request.revoked).map(request=><div className="sp-device-row" key={`${request.desktop_id}:${request.id}`}><div><strong>{request.name}</strong><p>{request.state==='approved'?'Approved':'Approval requested'} · {desktops.find(desktop=>desktop.id===request.desktop_id)?.name||'Desktop'}</p></div><button type="button" className="sp-console-action" disabled={busy} onClick={()=>void revoke(request)}>Revoke access</button></div>)}
  </section>;
}
