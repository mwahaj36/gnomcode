"use client";

import {useEffect,useRef,useState} from 'react';
import {supabase} from '../lib/supabase';
import {enrollmentDevice,forgetDevice,savedDevice,type DeviceOffer} from '../lib/remote-device';
import {approvedRequest} from '../lib/device-approval.mjs';
import DevicesView,{type Desktop} from './DevicesView';

type Request={desktop_id:string;id:string;name:string;state:string;revoked:boolean;offer:DeviceOffer|null};

export default function DesktopDevices({onPair,autoOpen=false}:{onPair?:(offer:DeviceOffer)=>void;autoOpen?:boolean}={}) {
  const [owner,setOwner]=useState<string|null>();
  const [desktops,setDesktops]=useState<Desktop[]>([]);
  const [requests,setRequests]=useState<Request[]>([]);
  const [status,setStatus]=useState('Checking your devices...');
  const [busy,setBusy]=useState(false);
  const [loaded,setLoaded]=useState(false);
  const [removeTarget,setRemoveTarget]=useState<Desktop|null>(null);
  const [removeError,setRemoveError]=useState('');
  const removeDialog=useRef<HTMLDialogElement|null>(null);
  const [waiting,setWaiting]=useState<{desktop:string;request:string}|null>(null);
  const [mineIds,setMineIds]=useState<string[]>([]);
  const [now,setNow]=useState(()=>Date.now());
  const autoStarted=useRef(false);
  const [disableTarget,setDisableTarget]=useState<Desktop|null>(null);
  const disableDialog=useRef<HTMLDialogElement|null>(null);
  useEffect(()=>{const {data:{subscription}}=supabase.auth.onAuthStateChange((_event,session)=>{setOwner(session?.user.id||null);setDesktops([]);setRequests([]);setLoaded(false);});return()=>subscription.unsubscribe();},[]);
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
        if(active){setLoaded(true);setDesktops([...(hosts.data||[])].sort((a,b)=>Date.parse(b.last_seen_at)-Date.parse(a.last_seen_at)));setRequests(clients.data||[]);setStatus(hosts.data?.length?'Select a desktop to open its conversations.':'Sign in to the native app to add your desktop.');}
      }catch{if(active)setStatus('Your device list is unavailable. Check your connection; the list will retry automatically.');}
      finally{running=false;}
    }
    void refresh();const timer=setInterval(()=>void refresh(),5000);
    return()=>{active=false;controller.abort();clearInterval(timer);};
  },[owner]);
  useEffect(()=>{
    if(!owner)return;let active=true;
    void Promise.all(desktops.map(host=>savedDevice(owner,host.id).catch(()=>undefined))).then(saved=>{if(active)setMineIds(saved.flatMap(row=>row?.offer.request_id?[row.offer.request_id]:[]));});
    return()=>{active=false;};
  },[owner,desktops,requests]);
  useEffect(()=>{const timer=setInterval(()=>setNow(Date.now()),30000);return()=>clearInterval(timer);},[]);
  useEffect(()=>{setNow(Date.now());},[desktops]);
  useEffect(()=>{if(disableTarget&&!disableDialog.current?.open)disableDialog.current?.showModal();},[disableTarget]);
  useEffect(()=>{
    if(autoOpen&&!autoStarted.current&&!busy&&owner&&desktops.some(host=>host.remote_enabled)){autoStarted.current=true;void connect(desktops.find(host=>host.remote_enabled)!);}
  },[autoOpen,desktops,owner,busy]);
  useEffect(()=>{
    if(!waiting || busy)return;
    const request=approvedRequest(requests,waiting);
    const desktop=desktops.find(row=>row.id===waiting.desktop);
    if(request && desktop){setWaiting(null);void connect(desktop);}
  },[requests,desktops,waiting,busy]);
  useEffect(()=>{
    if(!waiting)return;
    const timer=setTimeout(()=>{setWaiting(null);setStatus('Your desktop is not responding. Keep it awake with Gnomcode running, then select Open desktop to retry.');},30000);
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
        setWaiting({desktop:desktop.id,request:saved.offer.request_id!});setStatus('Connecting securely through your account...');return;
      }
      if(!request.offer || request.state!=='approved'){setWaiting({desktop:desktop.id,request:saved.offer.request_id!});setStatus('Waiting for your desktop to connect. Keep it awake with Gnomcode running.');return;}
      const offer={...request.offer,owner_id:owner,desktop_id:desktop.id,request_id:request.id};
      if(onPair)onPair(offer);else{sessionStorage.setItem('gnomcode-pairing-offer',JSON.stringify(offer));window.location.assign('/remote');}
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
  useEffect(()=>{if(removeTarget&&!removeDialog.current?.open)removeDialog.current?.showModal();},[removeTarget]);
  async function removeDesktop(desktop:Desktop) {
    if(!owner || busy)return;
    setBusy(true);setRemoveError('');
    try {
      const {data,error}=await supabase.from('spawnt_desktops').delete().eq('user_id',owner).eq('id',desktop.id).select('id');
      if(error || !data?.some(row=>row.id===desktop.id))throw new Error('Not removed');
      setDesktops(current=>current.filter(host=>host.id!==desktop.id));
      setRequests(current=>current.filter(request=>request.desktop_id!==desktop.id));
      await forgetDevice(owner,desktop.id).catch(()=>{});
      setStatus(`Removed ${desktop.name}. Local sessions were kept.`);
      removeDialog.current?.close();
    } catch { setRemoveError('Could not remove this desktop. Check your connection and retry.'); }
    finally { setBusy(false); }
  }
  async function removeRequest(req: Request) {
    if (!owner || busy) return;
    setBusy(true);
    try {
      const { error } = await supabase.from('spawnt_device_requests').delete().eq('user_id', owner).eq('id', req.id);
      if (error) throw error;
      await forgetDevice(owner, req.desktop_id);
      setRequests(current => current.filter(r => r.id !== req.id));
      setStatus(`Removed ${req.name}.`);
    } catch {
      setStatus('Could not remove device. Check your connection and retry.');
    } finally {
      setBusy(false);
    }
  }
  return <>
    <DevicesView owner={owner} status={status} busy={busy} desktops={desktops} requests={requests} mineIds={mineIds} now={now} onOpen={host=>void connect(host)} onDisable={setDisableTarget} onRemove={req=>void removeRequest(req)} onRemoveDesktop={host=>{setRemoveError('');setRemoveTarget(host);}} loaded={loaded}/>
    {removeTarget&&<dialog ref={removeDialog} className="remote-sheet sp-account-access-dialog" aria-labelledby="remove-desktop-heading" onClose={()=>setRemoveTarget(null)}>
      <h2 id="remove-desktop-heading">Remove desktop?</h2>
      <p>Remove {removeTarget.name} and its browser connections from this account. Local sessions and files stay on the desktop. A desktop still signed in may appear again when it reconnects.</p>
      {removeError&&<p role="alert">{removeError}</p>}
      <div className="web-profile-actions"><button type="button" autoFocus disabled={busy} onClick={()=>removeDialog.current?.close()}>Cancel</button><button type="button" disabled={busy} onClick={()=>void removeDesktop(removeTarget)}>Remove desktop</button></div>
    </dialog>}
    {disableTarget&&<dialog ref={disableDialog} className="remote-sheet sp-account-access-dialog" aria-labelledby="disable-account-heading" onClose={()=>setDisableTarget(null)}><h2 id="disable-account-heading">Disable account access?</h2><p>Every device signed in to your account will lose access to {disableTarget.name} when that desktop next reaches the account service. Local sessions keep running. You can enable access again in the native app.</p><div className="web-profile-actions"><button type="button" disabled={busy} onClick={()=>disableDialog.current?.close()}>Cancel</button><button type="button" disabled={busy} onClick={()=>void disableAccess(disableTarget)}>Disable account access</button></div></dialog>}
  </>;
}
