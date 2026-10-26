"use client";

import { useState, useEffect, useCallback } from "react";
import Link from "next/link";
import { supabase } from "../../lib/supabase";
import { BRAND } from "@/lib/brand";

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
      setPasswordMessage("Password updated successfully.");
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
      setLocalDaemonStatus("Offline / Loopback unreached");
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
    setStatusMessage("Settings and subscription tiers saved successfully.");
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
    <div className="max-w-4xl mx-auto px-6 py-12 font-mono text-xs">
      <div className="flex justify-between items-center mb-8 border-b border-border pb-4">
        <div>
          <div className="text-[11px] text-muted uppercase tracking-wider mb-1 font-bold">
            SYSTEM // CONFIGURATION
          </div>
          <h1 className="text-2xl font-bold tracking-tight text-foreground font-mono">
            Settings & Preferences
          </h1>
          <p className="text-xs text-muted mt-1 font-sans">
            Manage your subscription benchmarks, custom token rates, and sync preferences.
          </p>
        </div>
        <Link
          href="/console"
          className="h-8 px-3 bg-surface-elevated border border-border rounded-[4px] text-xs hover:border-primary text-foreground flex items-center gap-1"
        >
          &larr; Return to Console
        </Link>
      </div>

      {statusMessage && (
        <div className="mb-6 text-xs bg-surface-elevated text-foreground border border-border rounded-[4px] p-3">
          {statusMessage}
        </div>
      )}

      {/* Account & Sync Status */}
      <div className="bg-surface border border-border rounded-[6px] p-6 mb-8">
        <div className="flex justify-between items-center mb-4">
          <div className="font-bold text-foreground uppercase tracking-wider text-xs">
            01 // ACCOUNT & CLOUD SYNC
          </div>
          {user ? (
            <button
              onClick={handleSignOut}
              className="text-xs text-primary hover:underline font-bold"
            >
              [SIGN OUT]
            </button>
          ) : (
            <Link
              href="/auth"
              className="px-3 py-1 bg-primary text-white rounded-[4px] text-xs font-bold hover:brightness-110"
            >
              Connect Cloud Account &rarr;
            </Link>
          )}
        </div>
        <div className="space-y-3">
          <div className="flex justify-between items-center py-2 border-b border-border">
            <span className="text-muted">Cloud User Account</span>
            <span className="font-bold text-foreground">{user ? user.email : "Local Anonymous (Unlinked)"}</span>
          </div>
          <div className="flex justify-between items-center py-2 border-b border-border">
            <span className="text-muted">Local Daemon Engine</span>
            <span className="font-bold text-primary">{localDaemonStatus}</span>
          </div>
          <div className="flex justify-between items-center py-2">
            <span className="text-muted">Telemetry & Privacy</span>
            <span className="text-foreground font-bold">[100% PRIVATE &bull; ZERO CONTENT STORED]</span>
          </div>
        </div>

        {user && (
          <div className="mt-6 pt-5 border-t border-border">
            <div className="text-xs font-bold uppercase text-muted mb-3">UPDATE PASSWORD //</div>
            {passwordMessage && (
              <div className="mb-3 text-xs p-2.5 rounded-[4px] bg-surface-elevated border border-border text-foreground">
                {passwordMessage}
              </div>
            )}
            <form onSubmit={handleUpdatePassword} className="flex gap-3 items-center">
              <input
                type="password"
                value={newPassword}
                onChange={(e) => setNewPassword(e.target.value)}
                placeholder="New password (min 6 chars)"
                className="flex-1 bg-surface-elevated border border-border rounded-[4px] px-3 py-2 text-xs text-foreground focus:outline-none focus:border-primary font-mono"
              />
              <button
                type="submit"
                disabled={passwordUpdating}
                className="px-4 py-2 bg-surface-elevated border border-border text-foreground hover:border-primary font-bold rounded-[4px] text-xs disabled:opacity-50"
              >
                {passwordUpdating ? "Updating..." : "Update Password"}
              </button>
            </form>
          </div>
        )}
      </div>

      {/* Monthly Subscription Plans */}
      <div className="bg-surface border border-border rounded-[6px] p-6 mb-8">
        <div className="flex justify-between items-center mb-4">
          <div className="font-bold text-foreground uppercase tracking-wider text-xs">
            02 // SUBSCRIPTION BENCHMARKS ($/MO)
          </div>
          <span className="text-[11px] text-muted">Used for ROI calculations</span>
        </div>
        <form onSubmit={savePlans} className="space-y-4">
          <div>
            <label className="block text-muted mb-1">DEVELOPER DISPLAY NAME // </label>
            <input
              type="text"
              value={displayName}
              onChange={(e) => setDisplayName(e.target.value)}
              className="w-full bg-surface-elevated border border-border rounded-[4px] px-3 py-2 text-xs text-foreground focus:outline-none focus:border-primary font-mono"
            />
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
            <div>
              <label className="block text-muted mb-1">CLAUDE PLAN ($/MO)</label>
              <input
                type="number"
                value={claudePlan}
                onChange={(e) => setClaudePlan(Number(e.target.value))}
                className="w-full bg-surface-elevated border border-border rounded-[4px] px-3 py-2 text-xs text-foreground focus:outline-none focus:border-primary font-mono"
              />
            </div>
            <div>
              <label className="block text-muted mb-1">OPENAI PLAN ($/MO)</label>
              <input
                type="number"
                value={openaiPlan}
                onChange={(e) => setOpenaiPlan(Number(e.target.value))}
                className="w-full bg-surface-elevated border border-border rounded-[4px] px-3 py-2 text-xs text-foreground focus:outline-none focus:border-primary font-mono"
              />
            </div>
            <div>
              <label className="block text-muted mb-1">GOOGLE PLAN ($/MO)</label>
              <input
                type="number"
                value={googlePlan}
                onChange={(e) => setGooglePlan(Number(e.target.value))}
                className="w-full bg-surface-elevated border border-border rounded-[4px] px-3 py-2 text-xs text-foreground focus:outline-none focus:border-primary font-mono"
              />
            </div>
          </div>

          <button
            type="submit"
            disabled={saving}
            className="px-5 py-2 bg-primary text-white font-bold rounded-[4px] text-xs hover:brightness-110 transition-all disabled:opacity-50"
          >
            {saving ? "Saving..." : "Save Benchmark Plans"}
          </button>
        </form>
      </div>

      {/* Custom Model Rates */}
      <div className="bg-surface border border-border rounded-[6px] p-6 mb-8">
        <div className="flex justify-between items-center mb-4">
          <div className="font-bold text-foreground uppercase tracking-wider text-xs">
            03 // CUSTOM MODEL PRICING OVERRIDES
          </div>
          <span className="text-[11px] text-muted">Per 1M tokens</span>
        </div>

        {customPricing.length > 0 ? (
          <div className="overflow-x-auto mb-6 border border-border rounded-[4px]">
            <table className="w-full text-left text-[11px]">
              <thead className="bg-surface-elevated text-muted border-b border-border">
                <tr>
                  <th className="py-2 px-3">PATTERN //</th>
                  <th className="py-2 px-3">INPUT / 1M</th>
                  <th className="py-2 px-3">OUTPUT / 1M</th>
                  <th className="py-2 px-3">CACHE READ</th>
                  <th className="py-2 px-3 text-right">ACTION</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-border/60">
                {customPricing.map((cp) => (
                  <tr key={cp.model_pattern}>
                    <td className="py-2 px-3 font-bold text-foreground">{cp.model_pattern}</td>
                    <td className="py-2 px-3 text-muted">\${cp.input_per_mtok_usd.toFixed(2)}</td>
                    <td className="py-2 px-3 text-muted">\${cp.output_per_mtok_usd.toFixed(2)}</td>
                    <td className="py-2 px-3 text-muted">\${cp.cache_read_per_mtok_usd.toFixed(2)}</td>
                    <td className="py-2 px-3 text-right">
                      <button
                        onClick={() => deletePricing(cp.model_pattern)}
                        className="text-primary hover:underline text-[10px]"
                      >
                        [REMOVE]
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <p className="text-muted text-xs font-sans mb-4">
            No custom overrides configured. Standard LiteLLM/Official pricing catalog is in effect.
          </p>
        )}

        {/* Add custom rate form */}
        <div className="grid grid-cols-1 sm:grid-cols-4 gap-3">
          <input
            type="text"
            placeholder="Model pattern (e.g. gpt-4o-custom)"
            value={newPattern}
            onChange={(e) => setNewPattern(e.target.value)}
            className="bg-surface-elevated border border-border rounded-[4px] px-3 py-1.5 text-xs text-foreground focus:outline-none focus:border-primary font-mono"
          />
          <input
            type="number"
            step="0.01"
            placeholder="Input / 1M USD"
            value={newInput}
            onChange={(e) => setNewInput(e.target.value)}
            className="bg-surface-elevated border border-border rounded-[4px] px-3 py-1.5 text-xs text-foreground focus:outline-none focus:border-primary font-mono"
          />
          <input
            type="number"
            step="0.01"
            placeholder="Output / 1M USD"
            value={newOutput}
            onChange={(e) => setNewOutput(e.target.value)}
            className="bg-surface-elevated border border-border rounded-[4px] px-3 py-1.5 text-xs text-foreground focus:outline-none focus:border-primary font-mono"
          />
          <button
            type="button"
            onClick={addPricing}
            className="px-4 py-1.5 bg-surface-elevated border border-border text-foreground hover:border-primary font-bold rounded-[4px] text-xs"
          >
            Add Override
          </button>
        </div>
      </div>
    </div>
  );
}
