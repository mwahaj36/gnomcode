"use client";

import { useState, useEffect } from "react";
import Link from "next/link";

export default function MissionControl() {
  const [timeline, setTimeline] = useState("all");
  const [expandedProvider, setExpandedProvider] = useState<string | null>(null);
  const [agentFilter, setAgentFilter] = useState("all");
  const [statusFilter, setStatusFilter] = useState("all");
  const [page, setPage] = useState(1);
  const [showSettings, setShowSettings] = useState(false);

  // User subscription settings
  const [displayName, setDisplayName] = useState("Developer");
  const [openaiPlan, setOpenaiPlan] = useState(20);
  const [claudePlan, setClaudePlan] = useState(20);
  const [googlePlan, setGooglePlan] = useState(0);

  // Dummy/live metrics state
  const totalSpend = timeline === "today" ? 34.20 : timeline === "7d" ? 186.40 : timeline === "30d" ? 642.10 : 935.97;
  const totalSavings = timeline === "today" ? 142.00 : timeline === "7d" ? 890.50 : timeline === "30d" ? 3120.00 : 4579.71;
  const activeCount = 1;

  const combinedPlan = openaiPlan + claudePlan + googlePlan;
  const multiplier = combinedPlan > 0 ? (totalSpend / combinedPlan).toFixed(1) : "0.0";
  const netSavings = (totalSpend - combinedPlan).toFixed(2);

  const providers = [
    {
      name: "Anthropic",
      spend: timeline === "today" ? 18.40 : 517.32,
      savings: timeline === "today" ? 88.00 : 3456.69,
      pct: 55,
      models: [
        { name: "claude-opus-5", inTok: "121.6M", outTok: "447.8k", cost: 261.48 },
        { name: "Claude Opus 5.5 (Medium)", inTok: "4.1M", outTok: "123.1k", cost: 19.60 },
        { name: "Claude Opus 5.5 (Low)", inTok: "305.3k", outTok: "7.6k", cost: 2.03 },
      ],
    },
    {
      name: "OpenAI",
      spend: timeline === "today" ? 12.80 : 318.57,
      savings: timeline === "today" ? 42.00 : 967.87,
      pct: 34,
      models: [
        { name: "gpt-6.1-sol (Codex Desktop)", inTok: "508.7M", outTok: "1.9M", cost: 289.36 },
        { name: "gpt-4o", inTok: "28.5M", outTok: "118.2k", cost: 29.21 },
      ],
    },
    {
      name: "Google",
      spend: timeline === "today" ? 3.00 : 100.08,
      savings: timeline === "today" ? 12.00 : 155.15,
      pct: 11,
      models: [
        { name: "Gemini 3.8 Flash (Medium)", inTok: "222.5M", outTok: "544.5k", cost: 24.07 },
        { name: "Gemini 4 Argon (Medium)", inTok: "133.4M", outTok: "730.5k", cost: 15.46 },
        { name: "Gemini 3.8 Flash (High)", inTok: "142.8M", outTok: "486.4k", cost: 15.50 },
      ],
    },
  ];

  const sessions = [
    { project: "fleethud", agent: "antigravity", surface: "ide", model: "Gemini 3.8 Flash (High)", state: "tool", active: true, time: "Just now" },
    { project: "notii", agent: "claude-code", surface: "ide", model: "claude-opus-5", state: "idle", active: false, time: "Sep 2 (54d ago)" },
    { project: "notii", agent: "codex", surface: "desktop_app", model: "gpt-6.1-sol", state: "idle", active: false, time: "13:57 (2h ago)" },
    { project: "ngl", agent: "codex", surface: "desktop_app", model: "gpt-6.1-sol", state: "idle", active: false, time: "Yesterday 22:17" },
    { project: "c", agent: "codex", surface: "desktop_app", model: "gpt-6.1-sol", state: "idle", active: false, time: "Yesterday 12:46" },
    { project: "recon-reconstruct", agent: "codex", surface: "desktop_app", model: "gpt-6.1-sol", state: "idle", active: false, time: "Oct 24 16:11" },
  ];

  return (
    <div className="max-w-7xl mx-auto">
      {/* Top Header */}
      <header className="flex justify-between items-center pb-5 border-b border-border mb-5 flex-wrap gap-4">
        <div className="flex items-center gap-3">
          <h1 className="text-xl font-semibold tracking-tight">FleetHUD</h1>
          <span className="text-[11px] font-mono px-2 py-0.5 rounded bg-accent/15 text-accent border border-accent/30">
            MISSION CONTROL
          </span>
          <button
            onClick={() => setShowSettings(true)}
            className="text-[11px] font-mono px-2 py-0.5 rounded bg-green/15 text-green border border-green/30 hover:bg-green/25"
          >
            [PRO USER] {displayName}
          </button>
        </div>
        <div className="flex items-center gap-4">
          <Link
            href="/downloads"
            className="text-xs font-mono px-3 py-1 bg-green text-bg font-semibold rounded hover:opacity-90"
          >
            Download Native App &darr;
          </Link>
          <div className="text-xs font-mono text-muted">[CLOUD SYNC] Connected</div>
        </div>
      </header>

      {/* Timeline Filter Bar */}
      <div className="flex justify-between items-center bg-card border border-border rounded-md px-4 py-2.5 mb-5 flex-wrap gap-3">
        <div className="text-xs font-mono text-muted">
          Showing: {timeline === "today" ? "Today (Oct 26, 2026)" : timeline === "7d" ? "Last 7 Days" : timeline === "30d" ? "Last 30 Days" : "All Time (Sep 2 – Oct 26, 2026)"}
        </div>
        <div className="flex gap-2">
          {["today", "7d", "30d", "all"].map((t) => (
            <button
              key={t}
              onClick={() => setTimeline(t)}
              className={`px-3 py-1 text-xs font-mono rounded border transition-colors ${
                timeline === t
                  ? "bg-accent text-bg border-accent font-semibold"
                  : "border-border text-muted hover:border-accent"
              }`}
            >
              {t === "today" ? "Today" : t === "7d" ? "Last 7 Days" : t === "30d" ? "Last 30 Days" : "All Time"}
            </button>
          ))}
        </div>
      </div>

      {/* Summary Cards */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-5">
        <div className="bg-card border border-border rounded-md p-4">
          <div className="text-xs text-muted font-mono uppercase tracking-wider mb-2">API-Equivalent Spend</div>
          <div className="text-2xl font-bold font-mono">${totalSpend.toFixed(2)}</div>
          <div className="text-xs text-muted font-mono mt-1">{multiplier}x Plan Value ROI</div>
        </div>
        <div className="bg-card border border-border rounded-md p-4">
          <div className="text-xs text-muted font-mono uppercase tracking-wider mb-2">Prompt Cache Savings</div>
          <div className="text-2xl font-bold font-mono text-green">${totalSavings.toFixed(2)}</div>
          <div className="text-xs text-muted font-mono mt-1">Saved via prompt caching</div>
        </div>
        <div className="bg-card border border-border rounded-md p-4">
          <div className="text-xs text-muted font-mono uppercase tracking-wider mb-2">Active Fleet Agents</div>
          <div className="text-2xl font-bold font-mono">{activeCount}</div>
          <div className="text-xs text-muted font-mono mt-1">Active in last 15 min</div>
        </div>
        <div className="bg-card border border-border rounded-md p-4">
          <div className="text-xs text-muted font-mono uppercase tracking-wider mb-2">Quota Pools</div>
          <div className="text-2xl font-bold font-mono">2</div>
          <div className="text-xs text-muted font-mono mt-1">[CONFIRMED] Codex rate limits</div>
        </div>
      </div>

      {/* Multi-Plan ROI Calculator Card */}
      <div className="bg-card border border-border rounded-md p-4 mb-5">
        <div className="flex justify-between items-center mb-3">
          <span className="text-xs font-semibold text-muted font-mono uppercase tracking-wider">
            Subscription ROI Breakdown
          </span>
          <button onClick={() => setShowSettings(true)} className="text-xs text-accent font-mono hover:underline">
            [Edit Plans]
          </button>
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
          <div className="bg-bg border border-border rounded p-3 font-mono">
            <div className="text-[11px] text-muted uppercase">Anthropic (Claude)</div>
            <div className="text-lg font-bold my-1">25.8x Return</div>
            <div className="text-xs text-green">+$497.32 vs API</div>
            <div className="text-[10px] text-muted mt-1">Paid: ${claudePlan}/mo | Value: $517.32</div>
          </div>
          <div className="bg-bg border border-border rounded p-3 font-mono">
            <div className="text-[11px] text-muted uppercase">OpenAI (Codex)</div>
            <div className="text-lg font-bold my-1">15.9x Return</div>
            <div className="text-xs text-green">+$298.57 vs API</div>
            <div className="text-[10px] text-muted mt-1">Paid: ${openaiPlan}/mo | Value: $318.57</div>
          </div>
          <div className="bg-bg border border-border rounded p-3 font-mono">
            <div className="text-[11px] text-muted uppercase">Google (Antigravity)</div>
            <div className="text-lg font-bold my-1">{googlePlan > 0 ? (100.08 / googlePlan).toFixed(1) + "x Return" : "Free Tier"}</div>
            <div className="text-xs text-green">+$100.08 vs API</div>
            <div className="text-[10px] text-muted mt-1">Paid: ${googlePlan}/mo | Value: $100.08</div>
          </div>
          <div className="bg-bg border border-accent/40 rounded p-3 font-mono bg-accent/5">
            <div className="text-[11px] text-accent uppercase font-semibold">Combined Fleet ROI</div>
            <div className="text-lg font-bold my-1">{multiplier}x Return</div>
            <div className="text-xs text-green">+${netSavings} Net</div>
            <div className="text-[10px] text-muted mt-1">Total Paid: ${combinedPlan}/mo</div>
          </div>
        </div>
      </div>

      {/* Provider Spend Cards */}
      <div className="text-xs font-semibold text-muted font-mono uppercase tracking-wider mb-3">
        Spend by Provider (Click to View Models)
      </div>
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-5">
        {providers.map((p) => {
          const isOpen = expandedProvider === p.name;
          return (
            <div
              key={p.name}
              onClick={() => setExpandedProvider(isOpen ? null : p.name)}
              className="bg-card border border-border rounded-md p-4 cursor-pointer hover:border-accent transition-colors"
            >
              <div className="flex justify-between items-center mb-2">
                <span className="font-semibold text-base">{p.name}</span>
                <span className="font-mono text-sm font-bold">
                  ${p.spend.toFixed(2)}{" "}
                  <span className="text-xs text-muted font-normal">({p.pct}%)</span>
                </span>
              </div>
              <div className="text-xs text-muted font-mono">
                {p.models.length} model(s) • ${p.savings.toFixed(2)} cached
              </div>

              {isOpen && (
                <div className="mt-3 pt-3 border-t border-border/60 border-dashed space-y-2">
                  {p.models.map((m) => (
                    <div key={m.name} className="flex justify-between text-xs font-mono py-1 border-b border-border/30">
                      <span className="text-muted">{m.name}</span>
                      <span>${m.cost.toFixed(2)}</span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          );
        })}
      </div>

      {/* Quota Pools Table */}
      <div className="bg-card border border-border rounded-md p-4 mb-6">
        <div className="flex justify-between items-center mb-3">
          <span className="text-xs font-semibold text-muted font-mono uppercase tracking-wider">
            Per-Model Quota Pools
          </span>
          <span className="text-[11px] font-mono text-muted">
            [CONFIRMED] OpenAI Codex exposes exact rate limits.
          </span>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full text-left font-mono text-xs border-collapse">
            <thead>
              <tr className="border-b border-border text-muted">
                <th className="py-2.5 px-3">Provider</th>
                <th className="py-2.5 px-3">Pool ID</th>
                <th className="py-2.5 px-3">Window</th>
                <th className="py-2.5 px-3">Used %</th>
                <th className="py-2.5 px-3">Resets At</th>
                <th className="py-2.5 px-3">Basis</th>
              </tr>
            </thead>
            <tbody>
              <tr className="border-b border-border/50">
                <td className="py-2.5 px-3">codex</td>
                <td className="py-2.5 px-3">codex</td>
                <td className="py-2.5 px-3">5-hour</td>
                <td className="py-2.5 px-3 font-bold text-red">99.0%</td>
                <td className="py-2.5 px-3">16:51:32</td>
                <td className="py-2.5 px-3"><span className="px-1.5 py-0.5 rounded bg-accent/20 text-accent text-[10px]">EXACT</span></td>
              </tr>
              <tr className="border-b border-border/50">
                <td className="py-2.5 px-3">codex</td>
                <td className="py-2.5 px-3">codex</td>
                <td className="py-2.5 px-3">weekly</td>
                <td className="py-2.5 px-3 font-bold text-green">16.0%</td>
                <td className="py-2.5 px-3">11:51:32</td>
                <td className="py-2.5 px-3"><span className="px-1.5 py-0.5 rounded bg-accent/20 text-accent text-[10px]">EXACT</span></td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      {/* Sessions Table */}
      <div className="bg-card border border-border rounded-md p-4 mb-6">
        <div className="flex justify-between items-center mb-3 flex-wrap gap-2">
          <span className="text-xs font-semibold text-muted font-mono uppercase tracking-wider">
            Live & Historical Agent Sessions
          </span>
          <div className="flex gap-2">
            <select
              value={agentFilter}
              onChange={(e) => setAgentFilter(e.target.value)}
              className="bg-bg border border-border rounded px-2.5 py-1 text-xs font-mono"
            >
              <option value="all">All Agents</option>
              <option value="antigravity">Antigravity</option>
              <option value="codex">Codex</option>
              <option value="claude-code">Claude Code</option>
            </select>
            <select
              value={statusFilter}
              onChange={(e) => setStatusFilter(e.target.value)}
              className="bg-bg border border-border rounded px-2.5 py-1 text-xs font-mono"
            >
              <option value="all">All Statuses</option>
              <option value="active">Active Only</option>
              <option value="idle">Idle Only</option>
            </select>
          </div>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full text-left font-mono text-xs border-collapse">
            <thead>
              <tr className="border-b border-border text-muted">
                <th className="py-2.5 px-3">Project</th>
                <th className="py-2.5 px-3">Agent</th>
                <th className="py-2.5 px-3">Surface</th>
                <th className="py-2.5 px-3">Model</th>
                <th className="py-2.5 px-3">Status</th>
                <th className="py-2.5 px-3">Last Active</th>
              </tr>
            </thead>
            <tbody>
              {sessions.map((s, idx) => (
                <tr key={idx} className="border-b border-border/50">
                  <td className="py-2.5 px-3 font-semibold">{s.project}</td>
                  <td className="py-2.5 px-3">{s.agent}</td>
                  <td className="py-2.5 px-3">{s.surface}</td>
                  <td className="py-2.5 px-3">{s.model}</td>
                  <td className="py-2.5 px-3">
                    <span
                      className={`inline-block w-2 h-2 rounded-full mr-2 ${
                        s.active ? "bg-green shadow-[0_0_8px_rgba(63,185,80,0.4)]" : "bg-muted"
                      }`}
                    />
                    {s.state}
                  </td>
                  <td className="py-2.5 px-3 text-muted">{s.time}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* Settings Modal */}
      {showSettings && (
        <div className="fixed inset-0 bg-black/70 flex items-center justify-center p-4 z-50">
          <div className="bg-card border border-border rounded-lg max-w-md w-full p-6">
            <div className="flex justify-between items-center mb-4">
              <h3 className="font-semibold text-base">User Profile & Subscriptions</h3>
              <button onClick={() => setShowSettings(false)} className="text-muted hover:text-white">
                &times;
              </button>
            </div>
            <div className="space-y-4 font-mono text-xs">
              <div>
                <label className="block text-muted mb-1">Display Name</label>
                <input
                  type="text"
                  value={displayName}
                  onChange={(e) => setDisplayName(e.target.value)}
                  className="w-full bg-bg border border-border rounded p-2 text-white"
                />
              </div>
              <div>
                <label className="block text-muted mb-1">OpenAI Monthly Plan ($/mo)</label>
                <input
                  type="number"
                  value={openaiPlan}
                  onChange={(e) => setOpenaiPlan(Number(e.target.value))}
                  className="w-full bg-bg border border-border rounded p-2 text-white"
                />
              </div>
              <div>
                <label className="block text-muted mb-1">Claude Monthly Plan ($/mo)</label>
                <input
                  type="number"
                  value={claudePlan}
                  onChange={(e) => setClaudePlan(Number(e.target.value))}
                  className="w-full bg-bg border border-border rounded p-2 text-white"
                />
              </div>
              <div>
                <label className="block text-muted mb-1">Google Monthly Plan ($/mo)</label>
                <input
                  type="number"
                  value={googlePlan}
                  onChange={(e) => setGooglePlan(Number(e.target.value))}
                  className="w-full bg-bg border border-border rounded p-2 text-white"
                />
              </div>
            </div>
            <div className="flex justify-end gap-3 mt-6">
              <button
                onClick={() => setShowSettings(false)}
                className="px-4 py-2 border border-border rounded font-mono text-xs"
              >
                Close
              </button>
              <button
                onClick={() => setShowSettings(false)}
                className="px-4 py-2 bg-green text-bg font-semibold font-mono text-xs rounded"
              >
                Save Preferences
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
