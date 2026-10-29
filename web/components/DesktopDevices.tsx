"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { supabase } from "../lib/supabase";

type Offer = {room:string; key:string; relay:string; expires_at:string; name:string};

export default function DesktopDevices() {
  const [userId, setUserId] = useState<string | null>();
  const [offer, setOffer] = useState<Offer | null>(null);
  const [status, setStatus] = useState("Checking desktop availability...");
  useEffect(() => {
    const {data:{subscription}} = supabase.auth.onAuthStateChange((_event, session) => {
      setOffer(null); setUserId(session?.user.id || null);
    });
    return () => subscription.unsubscribe();
  }, []);
  useEffect(() => {
    let active = true;
    setOffer(null);
    if (userId === undefined) return;
    if (!userId) { setStatus("Sign in with the same account as your desktop."); return; }
    let busy = false;
    let controller: AbortController | null = null;
    async function refresh() {
      if (document.hidden || busy) return;
      busy = true;
      controller = new AbortController();
      const timeout = setTimeout(() => controller?.abort(), 10000);
      try {
        const {data, error} = await supabase.from("spawnt_sync_state").select("remote_offer:live_snapshot->remote_offer").eq("user_id", userId!).abortSignal(controller.signal).maybeSingle();
        if (!active) return;
        if (error) throw error;
        const value = data?.remote_offer as unknown as Offer | null;
        const valid = value && /^[a-f0-9]{32}$/.test(value.room) && /^[a-f0-9]{64}$/.test(value.key) && value.relay?.startsWith("wss://") && Date.parse(value.expires_at) > Date.now();
        setOffer(valid ? value : null);
        setStatus(valid ? "Ready to pair. Compare and confirm the code on both devices." : "No desktop is offering pairing. On your desktop, open Profile and choose Start pairing.");
      } catch {
        if (active) { setOffer(null); setStatus("Desktop discovery unavailable. Retry here or use a pairing link."); }
      } finally { clearTimeout(timeout); busy = false; }
    }
    void refresh();
    const timer = setInterval(() => void refresh(), 5000);
    return () => { active = false; clearInterval(timer); controller?.abort(); };
  }, [userId]);
  return <section className="web-profile-section" aria-label="Desktop devices"><h2>Desktop devices</h2><p role="status">{status}</p>{offer && <div className="desktop-device-row"><div><strong>My desktop</strong><p>Pairing expires {new Date(offer.expires_at).toLocaleTimeString()}</p></div><Link className="sp-console-action" href={`/remote#${new URLSearchParams({room:offer.room,key:offer.key,relay:offer.relay})}`}>Connect securely</Link></div>}{userId === null && <Link className="sp-console-action" href="/auth">Sign in</Link>}<p>Signing in finds your desktop. Access still requires matching codes and confirmation on both devices.</p></section>;
}
