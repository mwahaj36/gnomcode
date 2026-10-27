"use client";

import { useEffect, useRef, useState } from "react";
import { createClient, type Session, type SupabaseClient } from "@supabase/supabase-js";
import Link from "next/link";
import { supabaseUrl, supabasePublishableKey } from "../../lib/supabase";

type Request = { id: string; key: string; expires: number; name: string; project: string };
const requestKey = "fleethud_pair_request";
const authKey = "fleethud-pair-auth";
const hex = (bytes: Uint8Array) => Array.from(bytes, b => b.toString(16).padStart(2, "0")).join("");
const digest = async (bytes: Uint8Array) => hex(new Uint8Array(await crypto.subtle.digest("SHA-256", bytes as BufferSource)));
const base64 = (bytes: Uint8Array) => btoa(String.fromCharCode(...bytes));

export default function PairPage() {
  const client = useRef<SupabaseClient | null>(null);
  const [request, setRequest] = useState<Request | null>(null);
  const [session, setSession] = useState<Session | null>(null);
  const [code, setCode] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [approved, setApproved] = useState(false);
  const [expired, setExpired] = useState(false);
  const [confirmed, setConfirmed] = useState(false);

  useEffect(() => {
    let active = true;
    const reloadRequest = () => location.reload();
    window.addEventListener("hashchange", reloadRequest);
    try {
      const params = new URLSearchParams(location.hash.slice(1));
      let pending: Request;
      if (params.has("id")) {
        pending = { id: params.get("id") || "", key: params.get("key") || "", expires: Number(params.get("expires")), name: params.get("name") || "FleetHUD device", project: params.get("project") || "" };
        sessionStorage.setItem(requestKey, JSON.stringify(pending));
        history.replaceState(null, "", location.pathname);
      } else {
        pending = JSON.parse(sessionStorage.getItem(requestKey) || "null");
      }
      if (!pending) return () => { active = false; window.removeEventListener("hashchange", reloadRequest); };
      if (!/^[a-f0-9]{32}$/.test(pending.id) || !/^[a-f0-9]{64}$/.test(pending.key) || pending.project !== supabaseUrl || !Number.isFinite(pending.expires)) {
        throw new Error("Open the link shown by ‘fleethud pair’ in your cloud terminal or by Connect Account in the desktop app. Both must use the same Supabase project.");
      }
      if (pending.expires * 1000 <= Date.now() || pending.expires * 1000 > Date.now() + 600_000) throw new Error("This pairing link has expired. Start pairing again on the device you want to connect.");
      setRequest(pending);
      const bytes = Uint8Array.from(pending.key.match(/../g)!, v => parseInt(v, 16));
      void digest(bytes).then(value => { if (active) setCode(value.slice(0, 8).toUpperCase()); });
      client.current = createClient(supabaseUrl, supabasePublishableKey, { auth: { storageKey: authKey, storage: sessionStorage, persistSession: true, autoRefreshToken: false, flowType: "pkce", detectSessionInUrl: true } });
      void client.current.auth.getSession().then(({ data, error }) => {
        if (!active) return;
        if (error) setError(error.message);
        else setSession(data.session);
        history.replaceState(null, "", "/pair");
      });
    } catch (err) {
      sessionStorage.removeItem(requestKey);
      setError(err instanceof Error ? err.message : "Invalid pairing link.");
    }
    return () => { active = false; window.removeEventListener("hashchange", reloadRequest); };
  }, []);

  useEffect(() => {
    if (!request || approved) return;
    const timer = setInterval(() => setExpired(Date.now() >= request.expires * 1000), 1000);
    return () => clearInterval(timer);
  }, [request, approved]);

  async function signIn(provider?: "github") {
    if (!client.current) return;
    setBusy(true); setError("");
    try {
      if (provider) {
        // Reuse the existing allowed OAuth callback; /console forwards this isolated login to /pair.
        const { error } = await client.current.auth.signInWithOAuth({ provider, options: { redirectTo: `${location.origin}/console` } });
        if (error) throw error;
      } else {
        const { data, error } = await client.current.auth.signInWithPassword({ email, password });
        setPassword("");
        if (error) throw error;
        setSession(data.session);
      }
    } catch (err) { setError(err instanceof Error ? err.message : "Sign-in failed. Try again."); }
    finally { setBusy(false); }
  }

  async function approve() {
    if (!request || !session || !client.current || !confirmed || expired) return;
    setBusy(true); setError("");
    try {
      if (Date.now() >= request.expires * 1000) throw new Error("Pairing expired. Start again in your terminal.");
      const keyBytes = Uint8Array.from(request.key.match(/../g)!, v => parseInt(v, 16));
      const key = await crypto.subtle.importKey("raw", keyBytes, "AES-GCM", false, ["encrypt"]);
      const iv = crypto.getRandomValues(new Uint8Array(12));
      const collectorSession = { access_token: session.access_token, refresh_token: session.refresh_token, expires_at: session.expires_at, expires_in: session.expires_in, token_type: session.token_type, user: { id: session.user.id, email: session.user.email } };
      const ciphertext = await crypto.subtle.encrypt({ name: "AES-GCM", iv, additionalData: new TextEncoder().encode(request.id) }, key, new TextEncoder().encode(JSON.stringify(collectorSession)));
      const pollToken = await digest(keyBytes);
      const { error } = await client.current.rpc("fleethud_pair_approve", {
        p_request_id: request.id, p_poll_hash: await digest(new TextEncoder().encode(pollToken)), p_ciphertext: base64(new Uint8Array(ciphertext)), p_iv: base64(iv),
      });
      if (error) throw new Error(error.code === "PGRST202" ? "Pairing is not enabled yet. The project owner must apply supabase_device_pairing_v4.sql, then retry." : "Could not approve pairing. Check your connection or restart pairing if this link was already used.");
      // Do not sign out: that would revoke the independent session just sent to the collector.
      client.current.auth.stopAutoRefresh();
      sessionStorage.removeItem(authKey);
      sessionStorage.removeItem(`${authKey}-code-verifier`);
      sessionStorage.removeItem(requestKey);
      setSession(null); setRequest(null); setApproved(true);
    } catch (err) { setError(err instanceof Error ? err.message : "Pairing failed. Try again."); }
    finally { setBusy(false); }
  }

  async function cancel() {
    setBusy(true);
    try {
      await client.current?.auth.signOut({ scope: "local" });
      sessionStorage.removeItem(requestKey);
      location.replace("/console");
    } catch { setError("Could not cancel sign-in. Close this tab and restart pairing if needed."); setBusy(false); }
  }

  return (
    <div className="max-w-xl mx-auto px-5 py-12 font-sans">
      <div className="border border-border rounded-xl bg-card p-7 space-y-5">
        <p className="text-xs font-mono uppercase tracking-widest text-accent">Account pairing</p>
        <h1 className="text-2xl font-semibold">{approved ? "Approval sent" : "Connect a cloud workspace or device"}</h1>
        {approved ? <div role="status" className="space-y-3 text-sm text-muted">
          <p>Return to your terminal or desktop app and wait for “Account paired.” Keep the collector running. Your sessions will appear in All sessions within about 45 seconds.</p>
          <p>If the terminal reports an error, start pairing again. Approval can be collected once and expires after five minutes.</p>
          <Link className="text-accent underline inline-block" href="/console">Open your fleet</Link>
        </div> : <>
          <p className="text-sm text-muted">Updating Windows does not link a cloud workspace. Run <code className="text-foreground">fleethud pair</code> inside each new cloud environment, then open its link here. On your desktop, use Auth &amp; Cloud Sync → Connect Account. This works with supported collectors on Windows, macOS and Linux.</p>
          {request && <div className="rounded-lg border border-border bg-bg p-4 space-y-2">
            <p className="text-xs text-muted">Requesting workspace or device</p>
            <p className="font-semibold break-words">{request.name}</p>
            <p className="text-xs text-muted">Compare this code with the requesting terminal or app:</p>
            <p className="font-mono text-2xl tracking-widest text-accent">{code || "Loading…"}</p>
            <p className="text-xs text-muted">Only approve a request you started. The link expires after ten minutes.</p>
          </div>}
          {error && <p role="alert" className="text-sm text-red bg-red/10 border border-red/30 rounded p-3">{error}</p>}
          {expired && <p role="alert" className="text-sm text-red">This request expired. Run fleethud pair again to get a new link.</p>}
          {request && !expired && (session ? <div className="space-y-4">
            <p className="text-sm">Connect as <strong>{session.user.email || session.user.id}</strong>.</p>
            <label className="flex gap-3 text-sm"><input type="checkbox" checked={confirmed} onChange={e => setConfirmed(e.target.checked)} />I started this request and the verification code matches.</label>
            <button disabled={busy || !confirmed} onClick={approve} className="w-full bg-accent text-bg rounded px-4 py-3 font-semibold disabled:opacity-50">{busy ? "Sending approval…" : "Connect this workspace or device"}</button>
          </div> : <div className="space-y-4">
            <p className="text-sm text-muted">Sign in to the same FleetHUD account you use on Windows. This creates a separate login for the collector, so signing in on one device does not take over another device’s refresh token.</p>
            <button onClick={() => signIn("github")} disabled={busy} className="w-full border border-border rounded px-4 py-3 text-sm">Continue with GitHub</button>
            <form onSubmit={e => { e.preventDefault(); void signIn(); }} className="space-y-3">
              <label className="block text-xs">Email<input type="email" autoComplete="username" required value={email} onChange={e => setEmail(e.target.value)} className="block w-full mt-1 bg-bg border border-border rounded p-2 text-sm" /></label>
              <label className="block text-xs">Password<input type="password" autoComplete="current-password" required value={password} onChange={e => setPassword(e.target.value)} className="block w-full mt-1 bg-bg border border-border rounded p-2 text-sm" /></label>
              <button disabled={busy} className="w-full bg-accent text-bg rounded px-4 py-3 text-sm font-semibold disabled:opacity-50">{busy ? "Signing in…" : "Sign in with email"}</button>
            </form>
          </div>)}
          <p className="text-xs text-muted leading-relaxed">Pair once per persistent environment. Ephemeral workspaces need pairing again if their FleetHUD data is deleted. Pairing enables metadata sync; it does not give FleetHUD access to provider billing or hidden logs. Usage and estimated cost appear only when measured usage is available. Prompts, code and tool output are never synchronized.</p>
          {request && <button onClick={cancel} disabled={busy} className="text-sm text-muted underline">Cancel pairing</button>}
        </>}
      </div>
    </div>
  );
}
