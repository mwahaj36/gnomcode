"use client";

import { useState, useEffect, useCallback, useRef } from "react";
import Link from "next/link";
import { supabase } from "../lib/supabase";
import { disableDeviceNotifications, getNotificationPermission, requestNotificationPermission, triggerServerPushTest, type NotificationStatus } from "../lib/notifications";
import { AppLockSettings } from "./AppLockGate";

export default function SettingsPanel({ embedded = false }: { embedded?: boolean } = {}) {
  const [user, setUser] = useState<any>(null);
  const [displayName, setDisplayName] = useState("Developer");
  const [saving, setSaving] = useState(false);
  const [statusMessage, setStatusMessage] = useState("");
  const saveBusy = useRef(false);

  // Password update
  const [newPassword, setNewPassword] = useState("");
  const [showPassword, setShowPassword] = useState(false);
  const [passwordUpdating, setPasswordUpdating] = useState(false);
  const [passwordMessage, setPasswordMessage] = useState("");

  const [loadingPreferences, setLoadingPreferences] = useState(false);
  const [lastLoaded, setLastLoaded] = useState<Date | null>(null);
  const [signingOut, setSigningOut] = useState(false);
  const [notifStatus, setNotifStatus] = useState<NotificationStatus>("default");
  useEffect(() => {
    setNotifStatus(getNotificationPermission());
  }, []);

  const handleToggleNotifications = async () => {
    const granted = await requestNotificationPermission();
    setNotifStatus(granted ? "granted" : getNotificationPermission());
    if(granted){const result=await triggerServerPushTest();setStatusMessage(result.message);}
  };

  const handleTestNotification = async () => {
    const result=await triggerServerPushTest();
    setStatusMessage(result.message);
  };
  const loadBusy = useRef(false);
  const loadGeneration = useRef(0);
  const accountId = useRef<string | null>(null);
  const loadingAccount = useRef<string | null>(null);

  const handleUpdatePassword = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newPassword || newPassword.length < 6) {
      setPasswordMessage("Password must be at least 6 characters.");
      return;
    }
    setPasswordUpdating(true);
    try {
      const { error } = await supabase.auth.updateUser({ password: newPassword });
      if (error) setPasswordMessage(`Could not update password: ${error.message}`);
      else { setPasswordMessage("Password updated."); setNewPassword(""); setShowPassword(false); }
    } catch {
      setPasswordMessage("Could not update password. Check your connection and try again.");
    } finally { setPasswordUpdating(false); }
  };

  const handleSignOut = async () => {
    setSigningOut(true);
    try {
      await disableDeviceNotifications();
      const { error } = await supabase.auth.signOut();
      if (error) throw error;
      loadGeneration.current++;
      setUser(null); setLastLoaded(null);
      setNewPassword(""); setShowPassword(false); setPasswordMessage("");
      setStatusMessage("Signed out.");
    } catch {
      setStatusMessage("Could not sign out all sessions. Check your connection and account status, then try again.");
    } finally { setSigningOut(false); }
  };

  const loadSettings = useCallback(async (currentUser: any) => {
    if (!currentUser || (loadBusy.current && loadingAccount.current === currentUser.id) || saveBusy.current) return;
    loadingAccount.current = currentUser.id;
    loadBusy.current = true; setLoadingPreferences(true); setStatusMessage("Loading cloud preferences...");
    const generation = ++loadGeneration.current;
    try {
      const { data, error } = await supabase.from("gnomcode_sync_state").select("display_name").eq("user_id", currentUser.id).maybeSingle();
      if (generation !== loadGeneration.current) return;
      if (error) throw error;
      if (data) {
        setDisplayName(data.display_name || "Developer");
      }
      setLastLoaded(new Date()); setStatusMessage(data ? "Cloud preferences loaded." : "No saved cloud preferences. Suggested values are shown.");
    } catch (error) {
      if (generation === loadGeneration.current) setStatusMessage("Could not load cloud preferences. Retry Reload preferences.");
    } finally { if (generation === loadGeneration.current) { loadBusy.current = false; setLoadingPreferences(false); } }
  }, []);

  useEffect(() => {
    let mounted = true;
    const apply = (session: any) => {
      if (!mounted || accountId.current === (session?.user?.id ?? null)) return;
      accountId.current = session?.user?.id ?? null;
      loadGeneration.current++; loadBusy.current = false;
      setUser(session?.user ?? null); setLastLoaded(null);
      if (session?.user) setStatusMessage("");
      else { setNewPassword(""); setShowPassword(false); setPasswordMessage(""); }
      setDisplayName("Developer");
      if (session?.user) void loadSettings(session.user);
    };
    supabase.auth.getSession().then(({ data: { session } }) => apply(session));
    const { data: { subscription } } = supabase.auth.onAuthStateChange((_event, session) => { setTimeout(() => apply(session), 0); });
    const generationRef = loadGeneration;
    return () => { mounted = false; generationRef.current++; subscription.unsubscribe(); };
  }, [loadSettings]);

  const persist = async (values: Record<string, unknown>, success: string) => {
    if (!user) { setStatusMessage("Sign in to save cloud preferences. Configure local settings in the desktop console."); return false; }
    if (saveBusy.current || loadBusy.current) return false;
    saveBusy.current = true; setSaving(true); setStatusMessage("Saving cloud preferences...");
    try {
      const { error } = await supabase.from("gnomcode_sync_state").upsert({ user_id: user.id, email: user.email, ...values, updated_at: new Date().toISOString() });
      if (error) throw error;
      setStatusMessage(success); return true;
    } catch (error) {
      setStatusMessage("Save failed: " + (error instanceof Error ? error.message : (error as { message?: string })?.message || "Connection unavailable"));
      return false;
    } finally { saveBusy.current = false; setSaving(false); }
  };

  const saveProfile = async (e: React.FormEvent) => {
    e.preventDefault();
    await persist({display_name:displayName},"Account display name saved.");
  };

  const RootTag = embedded ? "div" : "main";

  return (
    <RootTag className={embedded ? "sp-settings-page font-mono" : "sp-console-shell sp-settings-page mx-auto px-6 py-10 font-mono"}>
      {!embedded && <header className="sp-settings-header">
        <div>
          <h1 className="text-2xl font-bold tracking-tight text-foreground font-sans">
            gnomcode / settings
          </h1>
          <p className="text-xs text-muted mt-1 font-sans">
            Manage your account and notification settings.
          </p>
        </div>
        <Link href="/console" className="sp-settings-back">← Return to Console</Link>
      </header>}

      {statusMessage && (
        <div className="sp-settings-message" role="status" aria-live="polite">
          {statusMessage}
        </div>
      )}

      <div className="sp-settings-submit">
        <button type="button" className="sp-settings-action" disabled={!user || loadingPreferences || saving} aria-busy={loadingPreferences} onClick={() => void loadSettings(user)}>{loadingPreferences ? "Loading preferences…" : "Reload preferences"}</button>
        <span className="sp-settings-hint">{lastLoaded ? "Last successful preference refresh: " + lastLoaded.toLocaleTimeString() : "Preferences have not been refreshed yet."}</span>
      </div>
      {/* Account & Sync Status */}
      <nav className="sp-settings-resources" aria-label="Resources"><Link href="/downloads">Download desktop app</Link><Link href="/docs">Documentation</Link><Link href="/pricing">Pricing</Link></nav>
      <section className="sp-settings-section" aria-labelledby="settings-account">
        <div className="sp-settings-section-heading">
          <h2 id="settings-account">Account</h2>
          {user ? (
            <button
              onClick={handleSignOut}
              disabled={signingOut}
              className="sp-settings-delete"
            >
              {signingOut ? "Signing out…" : "Sign out"}
            </button>
          ) : (
            <Link
              href="/auth"
              className="sp-settings-link"
            >
              Connect account
            </Link>
          )}
        </div>
        <dl className="sp-settings-facts">
          <div>
            <dt>Cloud account</dt>
            <dd>{user ? user.email : "Not signed in"}</dd>
          </div>
          <div>
            <dt>Data source</dt>
            <dd>Cloud preferences. Local settings remain on your desktop.</dd>
          </div>
          <div>
            <dt>Privacy</dt>
            <dd>Metadata sync excludes conversation content, code and tool output. Local chat stays on your desktop; paired Remote conversations use end-to-end encryption.</dd>
          </div>
        </dl>

        {user && (
          <div className="sp-settings-password">
            <h3>Update account password</h3>
            {passwordMessage && (
              <div className="sp-settings-message" role="status">
                {passwordMessage}
              </div>
            )}
            <form onSubmit={handleUpdatePassword} className="sp-settings-form-row">
              <label className="sp-settings-password-label" htmlFor="settings-password">New password (at least 6 characters)</label>
              <div className="sp-password-field"><input
                id="settings-password"
                type={showPassword ? "text" : "password"}
                value={newPassword}
                onChange={(e) => { setNewPassword(e.target.value); setPasswordMessage(""); }}
                autoComplete="new-password"
                className="sp-settings-input"
              />
              <button type="button" className="sp-password-toggle" aria-controls="settings-password" aria-pressed={showPassword} onClick={() => setShowPassword(!showPassword)}>{showPassword ? "Hide password" : "Show password"}</button></div>
              <button
                type="submit"
                disabled={passwordUpdating}
                className="sp-settings-action sp-settings-primary"
              >
                {passwordUpdating ? "Updating..." : "Update Password"}
              </button>
            </form>
          </div>
        )}

        <div className="sp-settings-notifications" style={{ marginTop: "24px", paddingTop: "20px", borderTop: "1px solid var(--sp-border)" }}>
          <h3>Notifications</h3>
          <p className="sp-settings-hint">{notifStatus === 'granted' ? 'Allowed on this device.' : notifStatus === 'denied' ? 'Blocked in browser settings.' : 'Not enabled on this device.'}</p>
          <div style={{ display: "flex", gap: "10px", flexWrap: "wrap", alignItems: "center" }}>
            {notifStatus !== "granted" ? (
              <button
                type="button"
                onClick={handleToggleNotifications}
                className="sp-settings-action sp-settings-primary"
              >
                Enable notifications
              </button>
            ) : (
              <span className="sp-trust-exact" style={{ padding: "8px 12px", border: "1px solid var(--sp-border)", borderRadius: "4px" }}>
                Notifications allowed
              </span>
            )}
            <button
              type="button"
              onClick={handleTestNotification}
              className="sp-settings-action"
            >
              {notifStatus === 'granted' ? 'Send test alert' : 'Enable and test'}
            </button>
            <button type="button" className="sp-settings-action" onClick={async()=>{setStatusMessage(await disableDeviceNotifications()?'Background alerts disabled on this device.':'Could not disable background alerts. Check your connection and try again.');}}>Disable background alerts on this device</button>
          </div>
          <details><summary>Setup help</summary><p>Your laptop must stay awake with account Remote access enabled. On iPhone, open Gnomcode from the Home Screen. Enable and test alerts separately on each device.</p></details>
        </div>
        <AppLockSettings />
      </section>

      {/* Account profile */}
      <section className="sp-settings-section" aria-labelledby="settings-profile">
        <div className="sp-settings-section-heading">
          <h2 id="settings-profile">Account profile</h2>
          <span>Your account display name</span>
        </div>
        <form onSubmit={saveProfile} className="sp-settings-plans-form">
          <div>
            <label htmlFor="display-name">Developer display name</label>
            <input id="display-name"
              type="text"
              value={displayName}
              onChange={(e) => setDisplayName(e.target.value)}
              className="sp-settings-input"
            />
          </div>

          <div className="sp-settings-submit">
            <button
              type="submit"
              disabled={saving || !user}
              className="sp-settings-action sp-settings-primary"
            >
              {saving ? "Saving..." : "Save display name"}
            </button>
          </div>
        </form>
      </section>

      <section className="sp-settings-section"><h2>Usage and pricing</h2><p>Subscription costs and model pricing are configured in Profile on your desktop. Reports here use that desktop’s Rust-computed values. Choose the desktop in the reporting view.</p><Link href="/docs">Open setup guide</Link></section>
    </RootTag>
  );
}
