import { NextResponse } from "next/server";
import { VAPID_PUBLIC_KEY, isVapidConfigured } from "../../../../lib/vapid";

export async function GET() {
  return NextResponse.json({
    configured: isVapidConfigured(),
    publicKey: VAPID_PUBLIC_KEY,
  });
}
