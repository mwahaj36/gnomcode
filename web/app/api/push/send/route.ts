import webpush from 'web-push';
import {pushAccount} from '../../../../lib/push-account';
import {notificationSubscription} from '../../../../lib/push-policy';
import {isVapidConfigured,VAPID_PUBLIC_KEY,VAPID_PRIVATE_KEY,VAPID_SUBJECT} from '../../../../lib/vapid';
export const runtime='nodejs';

export async function POST(request:Request){
 const account=await pushAccount(request);if(!account)return Response.json({error:'Sign in required.'},{status:401});
 if(!isVapidConfigured())return Response.json({error:'Background notifications are not configured.'},{status:503});
 const text=await request.text();if(text.length>1024)return Response.json({error:'Invalid notification request.'},{status:413});
 let value:{event_id?:unknown;kind?:unknown};try{value=JSON.parse(text);}catch{return Response.json({error:'Invalid request.'},{status:400});}
 if(!value || typeof value!=='object' || typeof value.event_id!=='string' || !/^[A-Za-z0-9:._-]{1,256}$/.test(value.event_id) || value.kind!=='agent_event')return Response.json({error:'Invalid event.'},{status:400});
 const {data:subscriptions,error}=await account.client.from('gnomcode_push_subscription').select('endpoint,subscription').eq('user_id',account.user.id).limit(32);
 if(error)return Response.json({error:'Notification storage is unavailable.'},{status:503});
 if(!subscriptions?.length)return Response.json({ok:true,devices:0});
 const {data:claim,error:claimError}=await account.client.from('gnomcode_push_delivery').upsert({user_id:account.user.id,event_id:value.event_id},{onConflict:'user_id,event_id',ignoreDuplicates:true}).select('event_id');
 if(claimError)return Response.json({error:'Could not record notification delivery.'},{status:503});
 if(!claim?.length)return Response.json({ok:true,duplicate:true});
 const payload=JSON.stringify({title:'Gnomcode',body:'An agent needs your attention or finished its turn. Open Gnomcode for details.',tag:value.event_id,data:{url:'/remote'}});
 let sent=0,failed=false;
 await Promise.all(subscriptions.map(async row=>{
   const subscription=notificationSubscription(row.subscription);if(!subscription)return;
   try {await webpush.sendNotification(subscription,payload,{vapidDetails:{subject:VAPID_SUBJECT,publicKey:VAPID_PUBLIC_KEY,privateKey:VAPID_PRIVATE_KEY},TTL:120,timeout:8000});sent++;}
   catch(error){const status=(error as {statusCode?:number}).statusCode;if(status===404 || status===410)await account.client.from('gnomcode_push_subscription').delete().eq('user_id',account.user.id).eq('endpoint',row.endpoint);else failed=true;}
 }));
 await account.client.from('gnomcode_push_delivery').delete().eq('user_id',account.user.id).lt('created_at',new Date(Date.now()-86400000).toISOString());
 if(failed && !sent){await account.client.from('gnomcode_push_delivery').delete().eq('user_id',account.user.id).eq('event_id',value.event_id);return Response.json({error:'Push service did not accept the notification.'},{status:502});}
 return Response.json({ok:true,devices:sent});
}
