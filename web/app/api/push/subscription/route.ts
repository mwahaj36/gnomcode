import {pushAccount} from '../../../../lib/push-account';
import {notificationSubscription} from '../../../../lib/push-policy';

export async function POST(request:Request){
  const account=await pushAccount(request);
  if(!account)return Response.json({error:'Sign in to enable notifications.'},{status:401});
  const text=await request.text();
  if(text.length>4096)return Response.json({error:'Subscription is too large.'},{status:413});
  let value:unknown;try{value=JSON.parse(text);}catch{return Response.json({error:'Invalid subscription.'},{status:400});}
  const subscription=notificationSubscription(value);
  if(!subscription)return Response.json({error:'Unsupported notification destination.'},{status:400});
  const {count,error:countError}=await account.client.from('gnomcode_push_subscription').select('*',{count:'exact',head:true}).eq('user_id',account.user.id);
  if(countError)return Response.json({error:'Notification storage is unavailable.'},{status:503});
  if((count || 0)>=32 && !(await account.client.from('gnomcode_push_subscription').select('endpoint').eq('user_id',account.user.id).eq('endpoint',subscription.endpoint).maybeSingle()).data)return Response.json({error:'Notification device limit reached. Remove an old device first.'},{status:409});
  const {error}=await account.client.from('gnomcode_push_subscription').upsert({user_id:account.user.id,endpoint:subscription.endpoint,subscription,updated_at:new Date().toISOString()},{onConflict:'user_id,endpoint'});
  return error?Response.json({error:'Could not save notification setup.'},{status:503}):Response.json({ok:true});
}
export async function DELETE(request:Request){
 const account=await pushAccount(request);if(!account)return Response.json({error:'Sign in required.'},{status:401});
 let value:unknown;try{value=await request.json();}catch{return Response.json({error:'Choose this device subscription.'},{status:400});}
 const subscription=notificationSubscription(value);if(!subscription)return Response.json({error:'Invalid subscription.'},{status:400});
 const {error}=await account.client.from('gnomcode_push_subscription').delete().eq('user_id',account.user.id).eq('endpoint',subscription.endpoint);
 return error?Response.json({error:'Could not disable notifications.'},{status:503}):Response.json({ok:true});
}
