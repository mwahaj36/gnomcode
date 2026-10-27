"use client";

import { useState, useEffect, useCallback } from "react";
import Link from "next/link";
import { supabase } from "../../lib/supabase";

interface CustomModelPricing {
  model_pattern: string;
  input_per_mtok_usd: number;
  output_per_mtok_usd: number;
  cache_read_per_mtok_usd: number;
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
    <div className="sp-console-shell max-w-4xl mx-auto px-6 py-10 font-mono">
      <div className="flex justify-between items-center mb-8 border-b border-border pb-4">
        <div>
          <h1 className="text-2xl font-bold tracking-tight text-foreground font-sans">
            spawnt / settings
          </h1>
          <p className="text-xs text-muted mt-1 font-sans">
            Manage your subscription tiers, custom model rate overrides, and sync connection.
          </p>
        </div>
        <Link
          href="/console"
          className="px-3 py-1.5 bg-bg border border-border rounded text-xs hover:border-accent"
        >
          &larr; Return to Console
        </Link>
      </div>

      {statusMessage && (
        <div className="mb-6 text-xs bg-green/15 text-green border border-green/30 rounded p-3">
          {statusMessage}
        </div>
      )}

      {/* Account & Sync Status */}
      <div className="bg-card border border-border rounded-xl p-6 mb-8">
        <div className="flex justify-between items-center mb-4">
          <h2 className="text-sm font-bold uppercase text-foreground">Account & Sync Connectivity</h2>
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
              className="sp-on-primary-text px-2.5 py-1 bg-accent rounded text-xs font-semibold hover:opacity-90"
            >
              Connect Cloud Account &rarr;
            </Link>
          )}
        </div>
        <div className="space-y-3 text-xs">
          <div className="flex justify-between items-center py-1.5 border-b border-border/40">
            <span className="text-muted">Cloud User Account</span>
            <span className="font-semibold text-foreground">{user ? user.email : "Local Anonymous (Unlinked)"}</span>
          </div>
          <div className="flex justify-between items-center py-1.5 border-b border-border/40">
            <span className="text-muted">Local Daemon Engine</span>
            <span className="font-semibold text-accent">{localDaemonStatus}</span>
          </div>
          <div className="flex justify-between items-center py-1.5">
            <span className="text-muted">Telemetry & Privacy</span>
            <span className="text-green font-semibold">100% Private &bull; Zero Content Stored</span>
          </div>
        </div>

        {user && (
          <div className="mt-6 pt-5 border-t border-border/60">
            <h3 className="text-xs font-semibold uppercase text-muted mb-3">Update Account Password</h3>
            {passwordMessage && (
              <div className="mb-3 text-xs p-2.5 rounded bg-bg border border-border text-foreground">
                {passwordMessage}
              </div>
            )}
            <form onSubmit={handleUpdatePassword} className="flex gap-3 items-center">
              <input
                type="password"
                value={newPassword}
                onChange={(e) => setNewPassword(e.target.value)}
                placeholder="New password (min 6 chars)"
                className="flex-1 bg-bg border border-border rounded px-3 py-1.5 text-xs text-foreground focus:outline-none focus:border-accent"
              />
              <button
                type="submit"
                disabled={passwordUpdating}
                className="px-4 py-1.5 bg-accent text-bg text-xs font-semibold rounded hover:opacity-90 disabled:opacity-50"
              >
                {passwordUpdating ? "Updating..." : "Update Password"}
              </button>
            </form>
          </div>
        )}
      </div>

      {/* Monthly Subscription Plans */}
      <div className="bg-card border border-border rounded-xl p-6 mb-8">
        <div className="flex justify-between items-center mb-4">
          <h2 className="text-sm font-bold uppercase text-foreground">Monthly Subscription Plans ($/mo)</h2>
          <span className="text-[11px] text-muted">Used for ROI multiplier calculations</span>
        </div>
        <form onSubmit={savePlans} className="space-y-4 text-xs">
          <div>
            <label className="block text-muted mb-1">Developer Display Name</label>
            <input
              type="text"
              value={displayName}
              onChange={(e) => setDisplayName(e.target.value)}
              className="w-full bg-bg border border-border rounded px-3 py-2 text-foreground focus:outline-none focus:border-accent"
            />
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
            <div>
              <label className="block text-muted mb-1">OpenAI Plan ($/mo)</label>
              <input
                type="number"
                step="1"
                value={openaiPlan}
                onChange={(e) => setOpenaiPlan(Number(e.target.value))}
                className="w-full bg-bg border border-border rounded px-3 py-2 text-foreground focus:outline-none focus:border-accent"
              />
              <span className="text-[10px] text-muted mt-1 block">Plus: $20 &bull; Pro: $200</span>
            </div>

            <div>
              <label className="block text-muted mb-1">Claude Plan ($/mo)</label>
              <input
                type="number"
                step="1"
                value={claudePlan}
                onChange={(e) => setClaudePlan(Number(e.target.value))}
                className="w-full bg-bg border border-border rounded px-3 py-2 text-foreground focus:outline-none focus:border-accent"
              />
              <span className="text-[10px] text-muted mt-1 block">Pro: $20 &bull; Max 5x: $100</span>
            </div>

            <div>
              <label className="block text-muted mb-1">Google AI Plan ($/mo)</label>
              <input
                type="number"
                step="1"
                value={googlePlan}
                onChange={(e) => setGooglePlan(Number(e.target.value))}
                className="w-full bg-bg border border-border rounded px-3 py-2 text-foreground focus:outline-none focus:border-accent"
              />
              <span className="text-[10px] text-muted mt-1 block">Pro: $20 &bull; Free: $0</span>
            </div>
          </div>

          <div className="pt-2 flex justify-end">
            <button
              type="submit"
              disabled={saving}
              className="px-5 py-2 bg-accent text-bg font-semibold rounded text-xs hover:opacity-90 transition-opacity"
            >
              {saving ? "Saving..." : "Save Subscription Preferences"}
            </button>
          </div>
        </form>
      </div>

      {/* Custom Model Pricing Overrides */}
      <div className="bg-card border border-border rounded-xl p-6">
        <h2 className="text-sm font-bold uppercase text-foreground mb-2">Custom Model Pricing Overrides</h2>
        <p className="text-xs text-muted mb-5 font-sans">
          Override default catalog token rates with custom negotiated pricing or enterprise discount tiers.
        </p>

        <div className="grid grid-cols-1 sm:grid-cols-5 gap-2 mb-4 text-xs">
          <input
            type="text"
            placeholder="Pattern (e.g. gpt-4o)"
            value={newPattern}
            onChange={(e) => setNewPattern(e.target.value)}
            className="sm:col-span-2 bg-bg border border-border rounded px-3 py-1.5 focus:outline-none focus:border-accent"
          />
          <input
            type="number"
            step="0.01"
            placeholder="Input $/M"
            value={newInput}
            onChange={(e) => setNewInput(e.target.value)}
            className="bg-bg border border-border rounded px-3 py-1.5 focus:outline-none focus:border-accent"
          />
          <input
            type="number"
            step="0.01"
            placeholder="Output $/M"
            value={newOutput}
            onChange={(e) => setNewOutput(e.target.value)}
            className="bg-bg border border-border rounded px-3 py-1.5 focus:outline-none focus:border-accent"
          />
          <input
            type="number"
            step="0.01"
            placeholder="Cache $/M"
            value={newCache}
            onChange={(e) => setNewCache(e.target.value)}
            className="bg-bg border border-border rounded px-3 py-1.5 focus:outline-none focus:border-accent"
          />
        </div>
        <button
          onClick={addPricing}
          className="px-4 py-1.5 bg-green text-bg font-semibold rounded text-xs hover:opacity-90 mb-5"
        >
          Add / Update Price Override
        </button>

        <div className="border border-border rounded-lg overflow-hidden text-xs">
          <table className="w-full text-left">
            <thead className="bg-bg text-[11px] text-muted border-b border-border uppercase">
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
                <tr key={p.model_pattern} className="hover:bg-bg/40">
                  <td className="p-3 font-semibold text-foreground">{p.model_pattern}</td>
                  <td className="p-3">${(p.input_per_mtok_usd ?? 0).toFixed(2)}</td>
                  <td className="p-3">${(p.output_per_mtok_usd ?? 0).toFixed(2)}</td>
                  <td className="p-3">${(p.cache_read_per_mtok_usd ?? 0).toFixed(2)}</td>
                  <td className="p-3 text-right">
                    <button
                      onClick={() => deletePricing(p.model_pattern)}
                      className="text-red hover:underline"
                    >
                      Delete
                    </button>
                  </td>
                </tr>
              ))}
              {customPricing.length === 0 && (
                <tr>
                  <td colSpan={5} className="p-4 text-center text-muted">
                    No custom pricing overrides configured. Default provider rates apply.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}
