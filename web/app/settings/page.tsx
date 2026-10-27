"use client";

import { useState, useEffect, useCallback } from "react";
import Link from "next/link";
import { supabase } from "../../lib/supabase";

interface CustomModelPricing {
  model_pattern: string;
  input_per_mtok_usd: number | null;
  output_per_mtok_usd: number | null;
  cache_read_per_mtok_usd: number | null;
  updated_at: string;
}

export default function SettingsPage() {
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
  const [localDaemonStatus, setLocalDaemonStatus] = useState("Checking...");

  // Password update
  const [newPassword, setNewPassword] = useState("");
  const [passwordUpdating, setPasswordUpdating] = useState(false);
  const [passwordMessage, setPasswordMessage] = useState("");

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
    await supabase.auth.signOut();
    setUser(null);
  };

  const loadSettings = useCallback(async (currentUser: any) => {
    // 1. Check local daemon
    try {
      const params = typeof window !== "undefined" ? new URLSearchParams(window.location.search) : null;
      const daemonPort = params?.get("port") || "41738";
      const bearerToken = params?.get("token") || "";
      const headers: Record<string, string> = bearerToken ? { Authorization: `Bearer ${bearerToken}` } : {};

      const res = await fetch(`http://127.0.0.1:${daemonPort}/api/v1/live`, { headers });
      if (res.ok) {
        const data = await res.json();
        setLocalDaemonStatus(`Connected (127.0.0.1:${daemonPort})`);
        if (data.user_profile) {
          setDisplayName(data.user_profile.display_name || "Developer");
          setOpenaiPlan(data.user_profile.openai_plan_usd || 20);
          setClaudePlan(data.user_profile.claude_plan_usd || 20);
          setGooglePlan(data.user_profile.google_plan_usd || 20);
        }
        if (data.custom_pricing) {
          setCustomPricing(data.custom_pricing);
        }
        return;
      }
    } catch {
      setLocalDaemonStatus("Offline / Not on this machine");
    }

    // 2. Fall back to Supabase
    if (currentUser) {
      const { data } = await supabase
        .from("fleethud_sync_state")
        .select("*")
        .eq("user_id", currentUser.id)
        .single();
      if (data) {
        setDisplayName(data.display_name || "Developer");
        setOpenaiPlan(data.openai_plan_usd || 20);
        setClaudePlan(data.claude_plan_usd || 20);
        setGooglePlan(data.google_plan_usd || 20);
        setCustomPricing(data.custom_pricing || []);
      }
    }
  }, []);

  useEffect(() => {
    supabase.auth.getSession().then(({ data: { session } }) => {
      setUser(session?.user ?? null);
      loadSettings(session?.user ?? null);
    });
  }, [loadSettings]);

  const savePlans = async (e: React.FormEvent) => {
    e.preventDefault();
    setSaving(true);
    setStatusMessage("");

    const payload = {
      display_name: displayName,
      openai_plan_usd: Number(openaiPlan),
      claude_plan_usd: Number(claudePlan),
      google_plan_usd: Number(googlePlan),
      updated_at: new Date().toISOString(),
    };

    try {
      await fetch("http://127.0.0.1:41738/api/v1/user", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload),
      });
    } catch {
      // ignore
    }

    if (user) {
      await supabase.from("fleethud_sync_state").upsert({
        user_id: user.id,
        email: user.email,
        ...payload,
      });
    }

    setSaving(false);
    setStatusMessage("Settings and subscription plans saved successfully!");
  };

  const addPricing = async () => {
    if (!newPattern.trim()) return;
    const entry: CustomModelPricing = {
      model_pattern: newPattern.trim(),
      input_per_mtok_usd: parseFloat(newInput) || 0.0,
      output_per_mtok_usd: parseFloat(newOutput) || 0.0,
      cache_read_per_mtok_usd: parseFloat(newCache) || 0.0,
      updated_at: new Date().toISOString(),
    };

    try {
      await fetch("http://127.0.0.1:41738/api/v1/pricing/custom", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(entry),
      });
    } catch {
      // ignore
    }

    const nextPricing = [...customPricing.filter((p) => p.model_pattern !== entry.model_pattern), entry];
    setCustomPricing(nextPricing);
    if (user) {
      await supabase.from("fleethud_sync_state").upsert({
        user_id: user.id,
        custom_pricing: nextPricing,
        updated_at: new Date().toISOString(),
      });
    }

    setNewPattern("");
    setNewInput("");
    setNewOutput("");
    setNewCache("");
  };

  const deletePricing = async (pattern: string) => {
    try {
      await fetch(`http://127.0.0.1:41738/api/v1/pricing/custom?pattern=${encodeURIComponent(pattern)}`, {
        method: "DELETE",
      });
    } catch {
      // ignore
    }

    const nextPricing = customPricing.filter((p) => p.model_pattern !== pattern);
    setCustomPricing(nextPricing);
    if (user) {
      await supabase.from("fleethud_sync_state").upsert({
        user_id: user.id,
        custom_pricing: nextPricing,
        updated_at: new Date().toISOString(),
      });
    }
  };

  return (
    <main className="sp-console-shell sp-settings-page mx-auto px-6 py-10 font-mono">
      <header className="sp-settings-header">
        <div>
          <h1 className="text-2xl font-bold tracking-tight text-foreground font-sans">
            spawnt / settings
          </h1>
          <p className="text-xs text-muted mt-1 font-sans">
            Manage your account, plan costs, and custom pricing overrides.
          </p>
        </div>
        <Link href="/console" className="sp-settings-back">← Return to Console</Link>
      </header>

      {statusMessage && (
        <div className="sp-settings-message" role="status" aria-live="polite">
          {statusMessage}
        </div>
      )}

      {/* Account & Sync Status */}
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
              className="sp-settings-action sp-settings-primary"
            >
              Connect account
            </Link>
          )}
        </div>
        <dl className="sp-settings-facts">
          <div>
            <dt>Cloud account</dt>
            <dd>{user ? user.email : "Local anonymous · unlinked"}</dd>
          </div>
          <div>
            <dt>Local daemon</dt>
            <dd>{localDaemonStatus}</dd>
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
            <label>Developer display name</label>
            <input
              type="text"
              value={displayName}
              onChange={(e) => setDisplayName(e.target.value)}
              className="sp-settings-input"
            />
          </div>

          <div className="sp-settings-plan-grid">
            <div>
              <label>OpenAI plan ($/mo)</label>
              <input
                type="number"
                step="1"
                value={openaiPlan}
                onChange={(e) => setOpenaiPlan(Number(e.target.value))}
                className="sp-settings-input"
              />
              <span className="sp-settings-hint">Plus: $20 · Pro: $200</span>
            </div>

            <div>
              <label>Claude plan ($/mo)</label>
              <input
                type="number"
                step="1"
                value={claudePlan}
                onChange={(e) => setClaudePlan(Number(e.target.value))}
                className="sp-settings-input"
              />
              <span className="sp-settings-hint">Pro: $20 · Max 5x: $100</span>
            </div>

            <div>
              <label>Google AI plan ($/mo)</label>
              <input
                type="number"
                step="1"
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
              disabled={saving}
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
            placeholder="Pattern (e.g. gpt-4o)"
            value={newPattern}
            onChange={(e) => setNewPattern(e.target.value)}
            className="sp-settings-input sp-settings-pattern"
          />
          <input
            type="number"
            step="0.01"
            placeholder="Input $/M"
            value={newInput}
            onChange={(e) => setNewInput(e.target.value)}
            className="sp-settings-input"
          />
          <input
            type="number"
            step="0.01"
            placeholder="Output $/M"
            value={newOutput}
            onChange={(e) => setNewOutput(e.target.value)}
            className="sp-settings-input"
          />
          <input
            type="number"
            step="0.01"
            placeholder="Cache $/M"
            value={newCache}
            onChange={(e) => setNewCache(e.target.value)}
            className="sp-settings-input"
          />
        </div>
        <button
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
