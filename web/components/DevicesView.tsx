"use client";

import {useState} from 'react';
import Link from 'next/link';
import type {DeviceOffer} from '../lib/remote-device';
import {relativeTime,isOnline,deviceKind,groupDevices} from '../lib/devices-presentation.mjs';

export type Desktop={id:string;name:string;last_seen_at:string;remote_enabled:boolean};
export type Request={desktop_id:string;id:string;name:string;state:string;revoked:boolean;offer:DeviceOffer|null};

const icon={fill:'none',stroke:'currentColor',strokeWidth:1.6,strokeLinecap:'round' as const,strokeLinejoin:'round' as const,'aria-hidden':true,width:20,height:20,viewBox:'0 0 24 24'};
const DesktopIcon=()=><svg {...icon}><rect x="3" y="4" width="18" height="13" rx="2"/><path d="M8 21h8m-4-4v4"/></svg>;
const PhoneIcon=()=><svg {...icon}><rect x="7" y="2.5" width="10" height="19" rx="2.5"/><path d="M11 18.5h2"/></svg>;
const BrowserIcon=()=><svg {...icon}><rect x="3" y="4" width="18" height="16" rx="2"/><path d="M3 9h18"/></svg>;

const stateText:Record<string,string>={this:'Signed in on this browser',approved:'Can open your desktop',pending:'Waiting for your desktop to accept',revoked:'Access removed'};

/** Presentation only: data and actions come from DesktopDevices. */
export default function DevicesView({owner,status,busy,desktops,requests,mineIds,now,onOpen,onDisable,onRemove}:{owner:string|null|undefined;status:string;busy:boolean;desktops:Desktop[];requests:Request[];mineIds:string[];now:number;onOpen:(desktop:Desktop)=>void;onDisable:(desktop:Desktop)=>void;onRemove:(request:Request)=>void}) {
  const [confirming,setConfirming]=useState<string|null>(null);
  const groups=groupDevices(requests,mineIds) as {key:string;label:string;rows:Request[]}[];
  const lead=desktops.find(host=>host.remote_enabled);
  return <section className="web-profile-section sp-devices" aria-label="Devices">
    <h2>Devices</h2>
    {owner===null && <div className="sp-empty"><p>Sign in with the same account as your desktop to see it here.</p><Link className="sp-dev-primary" href="/auth">Sign in</Link></div>}
    {owner!==null && <>
      <h3 className="sp-group-label">Your desktops {desktops.length>0&&<span>{desktops.length}</span>}</h3>
      {desktops.length===0 ? <div className="sp-empty"><p>No desktop is linked to this account yet. Sign in to the native app with the same account to add it.</p></div> :
        <div className="sp-dev-cards">{desktops.map(desktop=>{
          const online=isOnline(desktop.last_seen_at,now),seen=relativeTime(desktop.last_seen_at,now),on=desktop.remote_enabled;
          return <article className="sp-card sp-dev-card" key={desktop.id} aria-label={desktop.name}>
            <div className="sp-dev-card-head"><span className="sp-dev-icon" aria-hidden="true"><DesktopIcon/></span>
              <div className="sp-dev-main"><strong>{desktop.name}</strong><span title={desktop.last_seen_at}>{seen?`Last seen ${seen}`:'Not seen yet'}</span></div>
              {!on ? <span className="sp-chip sp-chip--neutral">Access off</span> : <span className={`sp-chip ${online?'sp-chip--ok':'sp-chip--unknown'}`}>{online?'Online':'Offline'}</span>}
            </div>
            <div className="sp-dev-actions">
              <button type="button" className={desktop.id===lead?.id?'sp-dev-primary':'sp-dev-secondary'} disabled={busy||!on} onClick={()=>onOpen(desktop)}>Open desktop</button>
              {on ? <button type="button" className="sp-dev-quiet-danger" disabled={busy} onClick={()=>onDisable(desktop)}>Disable account access</button> : <span className="sp-section-meta">Turn access back on in the native app.</span>}
            </div>
          </article>;
        })}</div>}
      <h3 className="sp-group-label sp-dev-browsers">Connected browsers {requests.length>0&&<span>{requests.length}</span>}</h3>
      {requests.length===0 ? <div className="sp-empty"><p>No browsers or phones are enrolled yet. Open a desktop from this browser to add it.</p></div> :
        groups.map(group=><div key={group.key} className="sp-dev-group"><h4 className="sp-group-label">{group.label} <span>{group.rows.length}</span></h4>
          <ul className="sp-session-list" role="list">{group.rows.map(req=>{
            const kind=deviceKind(req.name),asking=confirming===req.id;
            return <li className="sp-dev-row" key={req.id} title={`Device ID ${req.id}`}>
              <span className="sp-dev-icon" aria-hidden="true">{kind==='phone'?<PhoneIcon/>:<BrowserIcon/>}</span>
              <div className="sp-dev-main"><strong>{req.name}</strong><span>{stateText[group.key]}</span></div>
              {asking ? <div className="sp-dev-confirm" role="group" aria-label={`Remove ${req.name}`}>
                <button type="button" className="sp-dev-quiet" disabled={busy} onClick={()=>setConfirming(null)}>Cancel</button>
                <button type="button" className="sp-dev-quiet-danger" disabled={busy} onClick={()=>{setConfirming(null);onRemove(req);}}>Remove</button>
              </div> : <span className={`sp-chip ${group.key==='pending'?'sp-chip--low':group.key==='revoked'?'sp-chip--exhausted':group.key==='this'?'sp-chip--ok':'sp-chip--neutral'}`}>{group.key==='this'?'This browser':group.key==='pending'?'Pending':group.key==='revoked'?'Revoked':'Approved'}</span>}
              {!asking && <button type="button" className="sp-dev-quiet-danger" aria-label={`Remove ${req.name}`} disabled={busy} onClick={()=>setConfirming(req.id)}>Remove</button>}
            </li>;
          })}</ul></div>)}
    </>}
    <p className="sp-dev-status sp-section-meta" role="status">{owner===null?'':status}</p>
  </section>;
}
