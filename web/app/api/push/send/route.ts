import { NextResponse } from "next/server";
import webpush from "web-push";
import { VAPID_PUBLIC_KEY, VAPID_PRIVATE_KEY, VAPID_SUBJECT, isVapidConfigured } from "../../../../lib/vapid";

export async function POST(req: Request) {
  if (!isVapidConfigured()) {
    return NextResponse.json(
      { error: "VAPID keys are not configured in environment variables." },
      { status: 503 }
    );
  }

  try {
    webpush.setVapidDetails(VAPID_SUBJECT, VAPID_PUBLIC_KEY, VAPID_PRIVATE_KEY);

    const { subscription, title, body, tag, data } = await req.json();
    if (!subscription || !subscription.endpoint) {
      return NextResponse.json({ error: "Missing subscription object" }, { status: 400 });
    }

    const payload = JSON.stringify({
      title: title || "Gnomcode Alert",
      body: body || "Agent notification received.",
      tag: tag || "gnomcode-push",
      data: data || { url: "/remote" },
      icon: "/icons/gnomcode-192.png",
      badge: "/icons/gnomcode-192.png",
    });

    await webpush.sendNotification(subscription, payload);
    return NextResponse.json({ ok: true, sent: true });
  } catch (error: unknown) {
    const message = error instanceof Error ? error.message : "Failed to send web push";
    console.error("[WebPush Server Error]", message);
    return NextResponse.json({ error: message }, { status: 500 });
  }
}
