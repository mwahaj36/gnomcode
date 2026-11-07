import {isVapidConfigured,VAPID_PUBLIC_KEY} from '../../../../lib/vapid';
export async function GET(){return Response.json({configured:isVapidConfigured(),publicKey:isVapidConfigured()?VAPID_PUBLIC_KEY:null});}
