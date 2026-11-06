import {createClient} from '@supabase/supabase-js';
import {supabaseUrl,supabasePublishableKey} from './supabase';

export async function pushAccount(request:Request){
  const authorization=request.headers.get('authorization');
  if(!authorization?.startsWith('Bearer ') || authorization.length>10000)return null;
  const client=createClient(supabaseUrl,supabasePublishableKey,{auth:{persistSession:false,autoRefreshToken:false},global:{headers:{Authorization:authorization}}});
  const {data,error}=await client.auth.getUser(authorization.slice(7));
  return error || !data.user ? null : {client,user:data.user};
}
