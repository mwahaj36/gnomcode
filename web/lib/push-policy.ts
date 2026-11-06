import type {PushSubscription} from 'web-push';

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
