"use client";

import {useEffect,useRef,useState} from 'react';
import Link from 'next/link';
import {supabase} from '../lib/supabase';
import {enrollmentDevice,forgetDevice,type DeviceOffer} from '../lib/remote-device';

type Desktop={id:string;name:string;last_seen_at:string;remote_enabled:boolean};
type Request={desktop_id:string;id:string;name:string;state:string;revoked:boolean;offer:DeviceOffer|null};

export default function DesktopDevices({onPair,autoOpen=false}:{onPair?:(offer:DeviceOffer)=>void;autoOpen?:boolean}={}) {
  const [owner,setOwner]=useState<string|null>();
  const [desktops,setDesktops]=useState<Desktop[]>([]);
  const [requests,setRequests]=useState<Request[]>([]);
  const [status,setStatus]=useState('Checking your devices...');
  const [busy,setBusy]=useState(false);
  const [waiting,setWaiting]=useState('');
  const autoStarted=useRef(false);
  const [disableTarget,setDisableTarget]=useState<Desktop|null>(null);
  const disableDialog=useRef<HTMLDialogElement|null>(null);
  useEffect(()=>{const {data:{subscription}}=supabase.auth.onAuthStateChange((_event,session)=>{setOwner(session?.user.id||null);setDesktops([]);setRequests([]);});return()=>subscription.unsubscribe();},[]);
  useEffect(()=>{
    if(!owner){if(owner===null)setStatus('Sign in with the same account as your desktop.');return;}
    let active=true,running=false;
    const controller=new AbortController();
    async function refresh(){
      if(document.hidden || running)return;running=true;
      try{
        const [hosts,clients]=await Promise.all([
          supabase.from('spawnt_desktops').select('id,name,last_seen_at,remote_enabled').eq('user_id',owner!).abortSignal(AbortSignal.any([controller.signal,AbortSignal.timeout(10000)])),
          supabase.from('spawnt_device_requests').select('desktop_id,id,name,state,revoked,offer').eq('user_id',owner!).abortSignal(AbortSignal.any([controller.signal,AbortSignal.timeout(10000)])),
        ]);
        if(hosts.error || clients.error)throw new Error('Directory unavailable');
        if(active){setDesktops([...(hosts.data||[])].sort((a,b)=>Date.parse(b.last_seen_at)-Date.parse(a.last_seen_at)));setRequests(clients.data||[]);setStatus(hosts.data?.length?'Select a desktop to open its conversations.':'Sign in to the native app to add your desktop.');}
      }catch{if(active)setStatus('Your device list is unavailable. Check your connection; the list will retry automatically.');}
      finally{running=false;}
    }
    void refresh();const timer=setInterval(()=>void refresh(),5000);
    return()=>{active=false;controller.abort();clearInterval(timer);};
  },[owner]);
  useEffect(()=>{if(disableTarget&&!disableDialog.current?.open)disableDialog.current?.showModal();},[disableTarget]);
  useEffect(()=>{
    if(autoOpen&&!autoStarted.current&&!busy&&owner&&desktops.some(host=>host.remote_enabled)){autoStarted.current=true;void connect(desktops.find(host=>host.remote_enabled)!);}
  },[autoOpen,desktops,owner,busy]);
  useEffect(()=>{
    if(!waiting || busy)return;
    const request=requests.find(row=>row.desktop_id===waiting && !row.revoked && row.state==='approved' && row.offer);
    const desktop=desktops.find(row=>row.id===waiting);
    if(request && desktop){setWaiting('');void connect(desktop);}
  },[requests,desktops,waiting,busy]);
  useEffect(()=>{
    if(!waiting)return;
    const timer=setTimeout(()=>{setWaiting('');setStatus('Your desktop is not responding. Keep it awake with Spawnt running, then select Open desktop to retry.');},30000);
    return()=>clearTimeout(timer);
  },[waiting]);
  async function connect(desktop:Desktop){
    if(!owner || busy)return;setBusy(true);
    try{
      let saved=await enrollmentDevice(owner,desktop.id);
      const request=requests.find(row=>row.desktop_id===desktop.id && row.id===saved.offer.request_id);
      if(request?.revoked){await forgetDevice(owner,desktop.id);setStatus('This saved connection was removed. Open the desktop again to reconnect through your account.');return;}
      if(!request){
        if(!saved.enrollment){await forgetDevice(owner,desktop.id);saved=await enrollmentDevice(owner,desktop.id);}
        const {error}=await supabase.from('spawnt_device_requests').insert({user_id:owner,desktop_id:desktop.id,id:saved.offer.request_id,public_key:saved.enrollment!.publicHex,name:`${matchMedia('(pointer:coarse)').matches?'Phone':'Desktop'} browser ${saved.offer.request_id!.slice(-4)}`});
        if(error)throw error;
        setWaiting(desktop.id);setStatus('Connecting securely through your account...');return;
      }
      if(!request.offer || request.state!=='approved'){setWaiting(desktop.id);setStatus('Waiting for your desktop to connect. Keep it awake with Spawnt running.');return;}
      const offer={...request.offer,owner_id:owner,desktop_id:desktop.id,request_id:request.id};
      if(onPair)onPair(offer);else{sessionStorage.setItem('spawnt-pairing-offer',JSON.stringify(offer));window.location.assign('/remote');}
    }catch{setStatus('Could not connect this device. Check your connection and retry.');}
    finally{setBusy(false);}
  }
  async function disableAccess(desktop:Desktop){
    if(!owner || busy)return;setBusy(true);
    try{
      const {error}=await supabase.from('spawnt_desktops').update({remote_enabled:false}).eq('user_id',owner).eq('id',desktop.id);
      if(error)throw error;
      setDesktops(current=>current.map(host=>host.id===desktop.id?{...host,remote_enabled:false}:host));
      setStatus('Account access disabled for this desktop. It stops connections when it next reaches the account service.');
      disableDialog.current?.close();
    }catch{setStatus('Could not disable account access. Check your connection and retry.');}
    finally{setBusy(false);}
  }
  return <section className="web-profile-section" aria-label="Devices"><h2>Devices</h2><p role="status">{status}</p>
    {owner===null && <Link className="sp-console-action" href="/auth">Sign in</Link>}
    {desktops.map(desktop=><div className="sp-device-row" key={desktop.id}><div><strong>{desktop.name}</strong><p>{desktop.remote_enabled?'Available to signed-in devices':'Account access disabled'} · Last seen {new Date(desktop.last_seen_at).toLocaleString()}</p><button type="button" className="sp-console-action" disabled={busy||!desktop.remote_enabled} onClick={()=>void connect(desktop)}>Open desktop</button></div>{desktop.remote_enabled&&<button type="button" className="sp-console-action" disabled={busy} onClick={()=>setDisableTarget(desktop)}>Disable account access</button>}</div>)}
    {disableTarget&&<dialog ref={disableDialog} className="remote-sheet sp-account-access-dialog" aria-labelledby="disable-account-heading" onClose={()=>setDisableTarget(null)}><h2 id="disable-account-heading">Disable account access?</h2><p>Every device signed in to your account will lose access to {disableTarget.name} when that desktop next reaches the account service. Local sessions keep running. You can enable access again in the native app.</p><div className="web-profile-actions"><button type="button" disabled={busy} onClick={()=>disableDialog.current?.close()}>Cancel</button><button type="button" disabled={busy} onClick={()=>void disableAccess(disableTarget)}>Disable account access</button></div></dialog>}
  </section>;
}
