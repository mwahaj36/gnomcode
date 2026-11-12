import type {PushSubscription} from 'web-push';

const EVENT_LABELS:Record<string,string>={
  approval_waiting:'Needs approval',
  turn_finished:'Turn finished',
  limit_low:'Limit low',
  limit_reset:'Limit reset'
};

export interface ParsedPushRequest {
  event_id:string;
  title:string;
  body:string;
  tag:string;
  data:{url:string;event?:string};
}

function cleanPushField(value:unknown,maxLength:number):string | null {
  if(value===undefined || value===null)return '';
  if(typeof value!=='string')return null;
  const cleaned=value.replace(/[\x00-\x1f\x7f]/g,' ').replace(/\s+/g,' ').trim();
  if(cleaned.length<=maxLength)return cleaned;
  return `${cleaned.slice(0,Math.max(0,maxLength-3)).trimEnd()}...`;
}

export function parsePushNotificationRequest(value:unknown):ParsedPushRequest | null {
  if(!value || typeof value!=='object')return null;
  const raw=value as Record<string,unknown>;
  if(typeof raw.event_id!=='string' || !/^[A-Za-z0-9:._-]{1,256}$/.test(raw.event_id) || raw.kind!=='agent_event')return null;
  if(raw.event===undefined){
    return {
      event_id:raw.event_id,
      title:'Gnomcode',
      body:'An agent needs your attention or finished its turn. Open Gnomcode for details.',
      tag:raw.event_id,
      data:{url:'/remote'}
    };
  }
  if(typeof raw.event!=='string' || !Object.prototype.hasOwnProperty.call(EVENT_LABELS,raw.event))return null;
  const eventLabel=EVENT_LABELS[raw.event];
  const agent=cleanPushField(raw.agent,32);
  if(agent===null)return null;
  const prefix=agent?`[${agent}] `:'';
  const body=`${prefix}${eventLabel}`;
  return {
    event_id:raw.event_id,
    title:'Gnomcode',
    body:body.slice(0,240),
    tag:raw.event_id,
    data:{url:'/remote',event:raw.event}
  };
}

export function notificationSubscription(value:unknown):PushSubscription | null {
  if(!value || typeof value !== 'object')return null;
  const sub=value as Partial<PushSubscription>;
  if(typeof sub.endpoint !== 'string' || sub.endpoint.length>2048 || !sub.keys)return null;
  try {
    const url=new URL(sub.endpoint);
    const allowed=['fcm.googleapis.com','web.push.apple.com','updates.push.services.mozilla.com'].includes(url.hostname)
      || url.hostname.endsWith('.push.services.mozilla.com') || url.hostname.endsWith('.notify.windows.com');
    if(!allowed || url.protocol!=='https:' || url.username || url.password || url.port || url.hash)return null;
  }catch{return null;}
  if(!/^[A-Za-z0-9_-]{86,88}$/.test(sub.keys.p256dh) || !/^[A-Za-z0-9_-]{22,24}$/.test(sub.keys.auth))return null;
  return {endpoint:sub.endpoint,keys:{p256dh:sub.keys.p256dh,auth:sub.keys.auth}};
}
