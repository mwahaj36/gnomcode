"use client";

import { useState, useEffect, useCallback, useRef } from "react";
import Link from "next/link";
import { supabase } from "../lib/supabase";

interface CustomModelPricing {
  model_pattern: string;
  input_per_mtok_usd: number | null;
  output_per_mtok_usd: number | null;
  cache_read_per_mtok_usd: number | null;
  updated_at: string;
}

export default function SettingsPanel({ embedded = false }: { embedded?: boolean } = {}) {
  const [user, setUser] = useState<any>(null);
  const [displayName, setDisplayName] = useState("Developer");
  const [openaiPlan, setOpenaiPlan] = useState(20);
  const [claudePlan, setClaudePlan] = useState(20);
  const [googlePlan, setGooglePlan] = useState(20);
  const [customPricing, setCustomPricing] = useState<CustomModelPricing[]>([]);

  // Add form
  const [newPattern, setNewPattern] = useState("");
  const [newInput, setNewInput] = useState("");
  const [newOutput, setNewOutput] = useState("");
  const [newCache, setNewCache] = useState("");

  const [saving, setSaving] = useState(false);
  const [statusMessage, setStatusMessage] = useState("");
  const saveBusy = useRef(false);

  // Password update
  const [newPassword, setNewPassword] = useState("");
  const [passwordUpdating, setPasswordUpdating] = useState(false);
  const [passwordMessage, setPasswordMessage] = useState("");

  const [loadingPreferences, setLoadingPreferences] = useState(false);
  const [lastLoaded, setLastLoaded] = useState<Date | null>(null);
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
    const { error } = await supabase.auth.updateUser({ password: newPassword });
    setPasswordUpdating(false);
    if (error) {
      setPasswordMessage(`Error: ${error.message}`);
    } else {
      setPasswordMessage("Password updated successfully!");
      setNewPassword("");
    }
  };

  const handleSignOut = async () => {
    loadGeneration.current++;
    await supabase.auth.signOut();
    setUser(null); setCustomPricing([]); setLastLoaded(null);
  };

  const loadSettings = useCallback(async (currentUser: any) => {
    if (!currentUser || (loadBusy.current && loadingAccount.current === currentUser.id) || saveBusy.current) return;
    loadingAccount.current = currentUser.id;
    loadBusy.current = true; setLoadingPreferences(true); setStatusMessage("Loading cloud preferences...");
    const generation = ++loadGeneration.current;
    try {
      const { data, error } = await supabase.from("spawnt_sync_state").select("display_name,openai_plan_usd,claude_plan_usd,google_plan_usd,custom_pricing").eq("user_id", currentUser.id).maybeSingle();
      if (generation !== loadGeneration.current) return;
      if (error) throw error;
      if (data) {
        setDisplayName(data.display_name || "Developer");
        setOpenaiPlan(data.openai_plan_usd ?? 20); setClaudePlan(data.claude_plan_usd ?? 20); setGooglePlan(data.google_plan_usd ?? 20);
        setCustomPricing(data.custom_pricing || []);
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
      setUser(session?.user ?? null); setCustomPricing([]); setLastLoaded(null); setStatusMessage("");
      setDisplayName("Developer"); setOpenaiPlan(20); setClaudePlan(20); setGooglePlan(20);
      if (session?.user) void loadSettings(session.user);
    };
    supabase.auth.getSession().then(({ data: { session } }) => apply(session));
    const { data: { subscription } } = supabase.auth.onAuthStateChange((_event, session) => { setTimeout(() => apply(session), 0); });
    return () => { mounted = false; loadGeneration.current++; subscription.unsubscribe(); };
  }, [loadSettings]);

  const persist = async (values: Record<string, unknown>, success: string) => {
    if (!user) { setStatusMessage("Sign in to save cloud preferences. Configure local settings in the desktop console."); return false; }
    if (saveBusy.current || loadBusy.current) return false;
    saveBusy.current = true; setSaving(true); setStatusMessage("Saving cloud preferences...");
    try {
      const { error } = await supabase.from("spawnt_sync_state").upsert({ user_id: user.id, email: user.email, ...values, updated_at: new Date().toISOString() });
      if (error) throw error;
      setStatusMessage(success); return true;
    } catch (error) {
      setStatusMessage("Save failed: " + (error instanceof Error ? error.message : (error as { message?: string })?.message || "Connection unavailable"));
      return false;
    } finally { saveBusy.current = false; setSaving(false); }
  };

  const savePlans = async (e: React.FormEvent) => {
    e.preventDefault();
    await persist({ display_name: displayName, openai_plan_usd: openaiPlan, claude_plan_usd: claudePlan, google_plan_usd: googlePlan }, "Cloud preferences saved. Local desktop configuration is managed separately.");
  };

  const addPricing = async () => {
    if (!newPattern.trim()) return;
    const rates = [newInput, newOutput, newCache].map(value => value.trim() === "" ? null : Number(value));
    if (rates.some(value => value !== null && (!Number.isFinite(value) || value < 0))) { setStatusMessage("Enter nonnegative rates; leave unknown rates blank."); return; }
    const entry: CustomModelPricing = { model_pattern: newPattern.trim(), input_per_mtok_usd: rates[0], output_per_mtok_usd: rates[1], cache_read_per_mtok_usd: rates[2], updated_at: new Date().toISOString() };
    const next = [...customPricing.filter(p => p.model_pattern !== entry.model_pattern), entry];
    if (await persist({ custom_pricing: next }, "Cloud pricing preferences saved. Desktop pricing is configured locally.")) {
      setCustomPricing(next); setNewPattern(""); setNewInput(""); setNewOutput(""); setNewCache("");
    }
  };

  const deletePricing = async (pattern: string) => {
    const next = customPricing.filter(p => p.model_pattern !== pattern);
    if (await persist({ custom_pricing: next }, "Cloud pricing preference removed.")) setCustomPricing(next);
  };

  return (
    <main className={embedded ? "sp-settings-page font-mono" : "sp-console-shell sp-settings-page mx-auto px-6 py-10 font-mono"}>
      {!embedded && <header className="sp-settings-header">
        <div>
          <h1 className="text-2xl font-bold tracking-tight text-foreground font-sans">
            spawnt / settings
          </h1>
          <p className="text-xs text-muted mt-1 font-sans">
            Manage your account, plan costs, and custom pricing overrides.
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
      <nav className="sp-settings-resources" aria-label="Resources"><Link href="/docs">Documentation</Link><Link href="/pricing">Pricing</Link></nav>
      <section className="sp-settings-section" aria-labelledby="settings-account">
        <div className="sp-settings-section-heading">
          <h2 id="settings-account">Account</h2>
          {user ? (
            <button
              onClick={handleSignOut}
              className="text-xs text-red hover:underline"
            >
              Sign Out
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
            <dd>Prompts, code, and tool output are never stored or sent.</dd>
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
              <input
                type="password"
                value={newPassword}
                onChange={(e) => setNewPassword(e.target.value)}
                aria-label="New account password"
                autoComplete="new-password"
                placeholder="New password (min 6 chars)"
                className="sp-settings-input"
              />
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
      </section>

      {/* Monthly Subscription Plans */}
      <section className="sp-settings-section" aria-labelledby="settings-plans">
        <div className="sp-settings-section-heading">
          <h2 id="settings-plans">Subscription costs</h2>
          <span>Suggested monthly costs · edit to match your plans for ROI estimates</span>
        </div>
        <form onSubmit={savePlans} className="sp-settings-plans-form">
          <div>
            <label htmlFor="display-name">Developer display name</label>
            <input id="display-name"
              type="text"
              value={displayName}
              onChange={(e) => setDisplayName(e.target.value)}
              className="sp-settings-input"
            />
          </div>

          <div className="sp-settings-plan-grid">
            <div>
              <label htmlFor="openai-plan">OpenAI plan ($/mo)</label>
              <input id="openai-plan"
                type="number"
                min="0" step="1"
                value={openaiPlan}
                onChange={(e) => setOpenaiPlan(Number(e.target.value))}
                className="sp-settings-input"
              />
              <span className="sp-settings-hint">Plus: $20 · Pro: $200</span>
            </div>

            <div>
              <label htmlFor="claude-plan">Claude plan ($/mo)</label>
              <input id="claude-plan"
                type="number"
                min="0" step="1"
                value={claudePlan}
                onChange={(e) => setClaudePlan(Number(e.target.value))}
                className="sp-settings-input"
              />
              <span className="sp-settings-hint">Pro: $20 · Max 5x: $100</span>
            </div>

            <div>
              <label htmlFor="google-plan">Google AI plan ($/mo)</label>
              <input id="google-plan"
                type="number"
                min="0" step="1"
                value={googlePlan}
                onChange={(e) => setGooglePlan(Number(e.target.value))}
                className="sp-settings-input"
              />
              <span className="sp-settings-hint">Pro: $20 · Free: $0</span>
            </div>
          </div>

          <div className="sp-settings-submit">
            <button
              type="submit"
              disabled={saving || !user}
              className="sp-settings-action sp-settings-primary"
            >
              {saving ? "Saving..." : "Save Subscription Preferences"}
            </button>
          </div>
        </form>
      </section>

      {/* Custom Model Pricing Overrides */}
      <section className="sp-settings-section" aria-labelledby="settings-custom-pricing">
        <div className="sp-settings-section-heading sp-settings-stack-heading">
        <h2 id="settings-custom-pricing">Custom model pricing</h2>
        <p>
          Override default catalog token rates with custom negotiated pricing or enterprise discount tiers.
        </p>
        </div>

        <div className="sp-settings-pricing-form">
          <input
            type="text"
            aria-label="Model pattern" placeholder="Pattern (e.g. gpt-4o)"
            value={newPattern}
            onChange={(e) => setNewPattern(e.target.value)}
            className="sp-settings-input sp-settings-pattern"
          />
          <input
            type="number"
            step="0.01"
            aria-label="Input dollars per million tokens" placeholder="Input $/M"
            value={newInput}
            onChange={(e) => setNewInput(e.target.value)}
            className="sp-settings-input"
          />
          <input
            type="number"
            step="0.01"
            aria-label="Output dollars per million tokens" placeholder="Output $/M"
            value={newOutput}
            onChange={(e) => setNewOutput(e.target.value)}
            className="sp-settings-input"
          />
          <input
            type="number"
            step="0.01"
            aria-label="Cache dollars per million tokens" placeholder="Cache $/M"
            value={newCache}
            onChange={(e) => setNewCache(e.target.value)}
            className="sp-settings-input"
          />
        </div>
        <button
          disabled={saving || !user}
          onClick={addPricing}
          className="sp-settings-action sp-settings-add"
        >
          Add / Update Price Override
        </button>

        <div className="sp-settings-table-scroll" role="region" aria-label="Custom pricing overrides" tabIndex={0}>
          <table className="w-full text-left">
            <thead>
              <tr>
                <th className="p-3">Model Pattern</th>
                <th className="p-3">Input $/M</th>
                <th className="p-3">Output $/M</th>
                <th className="p-3">Cache Read $/M</th>
                <th className="p-3 text-right">Action</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-border/40">
              {customPricing.map((p) => (
              <tr key={p.model_pattern}>
                  <td data-label="Model pattern">{p.model_pattern}</td>
                  <td data-label="Input $/M">{p.input_per_mtok_usd == null ? <span className="sp-trust-unknown">UNKNOWN</span> : <>{`$${p.input_per_mtok_usd.toFixed(2)}`} <span className="sp-trust-exact">USER</span></>}</td>
                  <td data-label="Output $/M">{p.output_per_mtok_usd == null ? <span className="sp-trust-unknown">UNKNOWN</span> : <>{`$${p.output_per_mtok_usd.toFixed(2)}`} <span className="sp-trust-exact">USER</span></>}</td>
                  <td data-label="Cache read $/M">{p.cache_read_per_mtok_usd == null ? <span className="sp-trust-unknown">UNKNOWN</span> : <>{`$${p.cache_read_per_mtok_usd.toFixed(2)}`} <span className="sp-trust-exact">USER</span></>}</td>
                  <td data-label="Action">
                    <button
                      disabled={saving || !user}
                      onClick={() => deletePricing(p.model_pattern)}
                      className="sp-settings-delete"
                    >
                      Delete
                    </button>
                  </td>
                </tr>
              ))}
              {customPricing.length === 0 && (
                <tr>
                  <td colSpan={5} className="sp-settings-empty">
                    No custom pricing overrides configured. Default provider rates apply.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </section>
    </main>
  );
}
