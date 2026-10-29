"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { supabase } from "../lib/supabase";

type Offer = {room:string; key:string; relay:string; expires_at:string; name:string; pairing_code:string; protocol_version?:number};

export default function DesktopDevices({onPair}: {onPair?:(offer:Offer)=>void} = {}) {
  const [userId, setUserId] = useState<string | null>();
  const [offer, setOffer] = useState<Offer | null>(null);
  const [pairingCode, setPairingCode] = useState('');
  const [codeError, setCodeError] = useState('');
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
        if (active) { setOffer(null); setStatus("Desktop discovery unavailable. Check your connection and retry."); }
      } finally { clearTimeout(timeout); busy = false; }
    }
    void refresh();
    const timer = setInterval(() => void refresh(), 5000);
    return () => { active = false; clearInterval(timer); controller?.abort(); };
  }, [userId]);
  function pair(event: React.FormEvent) {
    event.preventDefault();
    if (!offer || Date.parse(offer.expires_at) <= Date.now()) { setCodeError('This code expired. Start pairing again on your desktop.'); return; }
    if (pairingCode.replace(/\s/g, '') !== offer.pairing_code) { setCodeError('Code does not match this desktop. Check the eight digits and retry.'); return; }
    if (onPair) onPair(offer);
    else { sessionStorage.setItem('spawnt-pairing-offer', JSON.stringify(offer)); window.location.assign('/remote'); }
  }
  return <section className="web-profile-section" aria-label="Desktop devices"><h2>Pair your desktop</h2><p role="status">{status}</p>{userId && <form className="pairing-code-form" onSubmit={pair}><label htmlFor="pairing-code">Code shown in desktop Profile</label><div><input id="pairing-code" inputMode="numeric" autoComplete="one-time-code" value={pairingCode} onChange={event => {setPairingCode(event.target.value.replace(/\D/g, '').slice(0,8));setCodeError('');}} maxLength={8} placeholder="00000000" required /><button className="sp-console-action" type="submit" disabled={pairingCode.length !== 8}>Pair desktop</button></div>{codeError && <p role="alert">{codeError}</p>}</form>}{userId === null && <Link className="sp-console-action" href="/auth">Sign in</Link>}<p>The code lasts five minutes. You will then confirm the matching security code on both devices.</p></section>;
}
