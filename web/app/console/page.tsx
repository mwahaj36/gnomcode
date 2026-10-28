"use client";

import { useState, useEffect, useCallback, useRef } from "react";
import Link from "next/link";
import { supabase } from "../../lib/supabase";

interface SessionItem {
  id: string;
  agent: string;
  surface: string;
  project_name: string;
  model: string;
  state: string;
  is_active: boolean;
  started_at: string;
  last_event_at: string;
  input_tokens?: number;
  output_tokens?: number;
  cache_read_tokens?: number;
  context_tokens?: number;
  context_window?: number;
  total_cost_usd?: number;
  title?: string;
  device_name?: string;
  usage_available?: boolean;
  cost_basis?: string;
}

interface QuotaItem {
  provider: string;
  limit_id: string;
  window: string;
  used_pct: number;
  resets_at: string | null;
  plan: string | null;
  basis: string;
  observed_at: string;
}

interface ProviderModelSpend {
  model: string;
  input_tokens: number;
  output_tokens: number;
  cost_usd: number;
}

interface ProviderSpend {
  provider: string;
  total_cost_usd: number;
  total_cache_savings_usd?: number;
  cache_savings_usd?: number;
  models: ProviderModelSpend[];
}

interface TimelineReport {
  total_cost_usd: number;
  total_cache_savings_usd: number;
  total_input_tokens: number;
  total_cache_read_tokens: number;
  total_output_tokens: number;
  providers: ProviderSpend[];
  range_label?: string;
  earliest_at?: string | null;
  latest_at?: string | null;
}

interface PlanRoiInfo {
  provider: string;
  plan_usd: number;
  monthly_plan_usd?: number;
  spend_usd: number;
  multiplier: number;
  net_savings_usd: number;
  period_label?: string;
}

interface UserProfile {
  id: string;
  username: string;
  display_name: string;
  openai_plan_usd: number;
  claude_plan_usd: number;
  google_plan_usd: number;
  other_plan_usd: number;
  updated_at: string;
}

function formatResetCountdown(resetsAtIso?: string | null, _windowKind?: string) {
  if (!resetsAtIso) return "Rolling";
  const target = new Date(resetsAtIso);
  const now = new Date();
  const diffMs = target.getTime() - now.getTime();
  if (diffMs <= 0) return "Resetting now";

  const diffHours = Math.floor(diffMs / (1000 * 60 * 60));
  const diffMinutes = Math.floor((diffMs % (1000 * 60 * 60)) / (1000 * 60));
  const diffDays = Math.floor(diffHours / 24);
  const remHours = diffHours % 24;

  const timeStr = target.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
  const dateStr = target.toLocaleDateString([], { month: "short", day: "numeric" });

  if (diffDays > 0) {
    return `in ${diffDays}d ${remHours}h (${dateStr}, ${timeStr})`;
  } else if (diffHours > 0) {
    return `in ${diffHours}h ${diffMinutes}m (${timeStr})`;
  } else {
    return `in ${diffMinutes}m (${timeStr})`;
  }
}

export default function ConsolePage() {
  const [user, setUser] = useState<any>(null);
  const [authChecked, setAuthChecked] = useState(false);

  // Filters & Pagination
  const [timeline, setTimeline] = useState("all");
  const [agentFilter, setAgentFilter] = useState("all");
  const [statusFilter, setStatusFilter] = useState("all");
  const [page, setPage] = useState(1);
  const [expandedAgents, setExpandedAgents] = useState<Record<string, boolean>>({});

  // Live Metrics
  const [report, setReport] = useState<TimelineReport | null>(null);
  const [sessions, setSessions] = useState<SessionItem[]>([]);
  const [quota, setQuota] = useState<QuotaItem[]>([]);
  const [roiByProvider, setRoiByProvider] = useState<PlanRoiInfo[]>([]);
  const [combinedRoi, setCombinedRoi] = useState<PlanRoiInfo | null>(null);
  const [userProfile, setUserProfile] = useState<UserProfile | null>(null);
  const [activeCount, setActiveCount] = useState<number>(0);
  const [totalCount, setTotalCount] = useState<number>(0);
  const [totalPages, setTotalPages] = useState<number>(1);
  const [copyFeedback, setCopyFeedback] = useState("");
  const [connectedSource, setConnectedSource] = useState<string>("Scanning...");
  const [refreshing, setRefreshing] = useState(false);
  const [lastSuccessfulUpdate, setLastSuccessfulUpdate] = useState<Date | null>(null);
  const [refreshMessage, setRefreshMessage] = useState("");
  const refreshInFlight = useRef(false);
  const [activeView, setActiveView] = useState("overview");
  const [viewError, setViewError] = useState("");

  useEffect(() => {
    const path = window.location.pathname.split("/").filter(Boolean).at(-1);
    if (path && ["overview", "sessions", "usage", "limits", "projects", "automations"].includes(path)) setActiveView(path);
  }, []);

  const changeConsoleView = (view: string) => {
    setActiveView(view);
    window.history.replaceState(null, "", view === "overview" ? "/console" : `/console/${view}`);
  };

  useEffect(() => {
    supabase.auth.getSession().then(({ data: { session } }) => {
      setUser(session?.user ?? null);
      setAuthChecked(true);

    });

    const { data: { subscription } } = supabase.auth.onAuthStateChange((_event, session) => {
      setUser(session?.user ?? null);
    });

    return () => subscription.unsubscribe();
  }, []);

  // Live 2-second fetch loop
  const fetchLiveMetrics = useCallback(async (force = false) => {
    if (refreshInFlight.current && !force) return;
    if (refreshInFlight.current && force) await new Promise<void>((resolve) => {
      const check = () => refreshInFlight.current ? window.setTimeout(check, 50) : resolve();
      check();
    });
    if (refreshInFlight.current) return;
    refreshInFlight.current = true;
    setViewError("");
    try {
    if (user) {
      try {
        const [desktop, cloud] = await Promise.all([
          supabase.from("fleethud_sync_state").select("*").eq("user_id", user.id).maybeSingle(),
          supabase.from("fleethud_cloud_sessions_live").select("session,updated_at").eq("user_id", user.id).order("updated_at", { ascending: false }).limit(1000),
        ]);
        const snap = desktop.data?.live_snapshot;
        if (desktop.error && cloud.error) setViewError("Cloud data is unavailable. Last received snapshot remains visible when available.");
        const cloudSessions: SessionItem[] = (cloud.data || []).map((row) => row.session as SessionItem);
        const allSessions: SessionItem[] = Array.from(new Map<string, SessionItem>([...(snap?.sessions || []), ...cloudSessions].map((session: SessionItem) => [session.id, session])).values());
        const filtered = allSessions.sort((a, b) => Date.parse(b.last_event_at) - Date.parse(a.last_event_at)).filter((session) =>
          (agentFilter === "all" || session.agent === agentFilter) &&
          (statusFilter === "all" || (statusFilter === "active" ? session.is_active : !session.is_active)));
        setSessions(filtered.slice((page-1)*10,page*10));
        setReport(snap?.timelines?.[timeline] || snap?.timeline || null);
        setQuota(snap?.quota || []);
        setRoiByProvider(snap?.roi_by_provider || []);
        setCombinedRoi(snap?.combined_roi || null);
        setUserProfile(snap?.user_profile || null);
        setActiveCount(snap?.active_count || 0);
        setTotalCount(filtered.length);
        setTotalPages(Math.max(1,Math.ceil(filtered.length/10)));
        const fresh = data && Date.now()-Date.parse(data.updated_at)<120000;
        setConnectedSource(error ? "Desktop sync unavailable" : !snap ? "No paired desktop data yet" : fresh ? "Desktop connected" : "Desktop data stale");
      } catch {
        setConnectedSource("Offline");
        setViewError("Connection failed. Showing the last data held in this page.");
      }
    } else {
      setConnectedSource("Sign in to monitor your desktop remotely");
    }
    } finally {
    refreshInFlight.current = false;
    }
  }, [user, timeline, agentFilter, statusFilter, page]);

  const refreshCloudSnapshot = async () => {
    if (refreshInFlight.current || refreshing) return;
    setRefreshing(true);
    setRefreshMessage("Refreshing cloud view...");
    try {
      await fetchLiveMetrics(true);
      setLastSuccessfulUpdate(new Date());
      setRefreshMessage("Cloud view refreshed. Cloud snapshot publishing is controlled by the paired desktop daemon.");
    } catch {
      setRefreshMessage("Refresh failed. Showing the last available data.");
    } finally {
      setRefreshing(false);
    }
  };

  useEffect(() => {
    fetchLiveMetrics();
    const timer = setInterval(fetchLiveMetrics, 15000);
    return () => clearInterval(timer);
  }, [fetchLiveMetrics]);

  const formatTokens = (num: number | undefined | null) => {
    const val = num ?? 0;
    if (val >= 1_000_000) return (val / 1_000_000).toFixed(1) + "M";
    if (val >= 1_000) return (val / 1_000).toFixed(1) + "k";
    return val.toString();
  };

  const formatTimeAgo = (iso: string) => {
    const diff = (Date.now() - new Date(iso).getTime()) / 1000;
    if (diff < 60) return "Just now";
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
    return `${Math.floor(diff / 86400)}d ago`;
  };

  const handleRemoteRelay = async (session: SessionItem) => {
    // 1. Try local daemon endpoint if running on this device
    try {
      const res = await fetch("http://127.0.0.1:41738/api/v1/pro/relay/execute", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id: `relay-${session.id}` }),
      });
      if (res.ok) {
        alert("Context Relay handoff executed locally! Clean session starting with 0 tokens.");
        fetchLiveMetrics();
        return;
      }
    } catch {}

    // 2. If remote (away from laptop), communicate via Supabase sync state to local daemon
    if (!user) {
      alert("Sign in to execute remote relay handoff to your desktop daemon.");
      return;
    }

    try {
      const { data: syncRow } = await supabase
        .from("fleethud_sync_state")
        .select("live_snapshot")
        .eq("user_id", user.id)
        .maybeSingle();

      const existingSnap = (syncRow?.live_snapshot as any) || {};
      const pendingCmds = existingSnap.relay_commands || [];
      const updatedSnap = {
        ...existingSnap,
        relay_commands: [
          ...pendingCmds,
          {
            id: `relay-${session.id}`,
            action: "execute",
            session_id: session.id,
            requested_at: new Date().toISOString(),
          },
        ],
      };

      const { error } = await supabase
        .from("fleethud_sync_state")
        .update({
          live_snapshot: updatedSnap,
          updated_at: new Date().toISOString(),
        })
        .eq("user_id", user.id);

      if (error) throw error;
      alert("Handoff command transmitted to your desktop daemon! Rotation in progress.");
    } catch (err: any) {
      alert("Failed to transmit handoff command: " + err.message);
    }
  };

  const activeAgents = (report?.providers || []).filter(
    (p) => (p.total_cost_usd > 0) || (p.models && p.models.some((m) => (m.input_tokens ?? 0) > 0 || (m.cost_usd ?? 0) > 0))
  );

  const activeRoi = roiByProvider.filter((roi) => (roi.spend_usd ?? 0) > 0);
  const tokenTotal = report ? report.total_input_tokens + report.total_cache_read_tokens + report.total_output_tokens : null;
  const chartValues = report?.providers?.map((provider) => ({ label: provider.provider, value: provider.total_cost_usd })) || [];
  const maxChartValue = Math.max(0, ...chartValues.map((item) => item.value));

  if (!authChecked) {
    return (
      <div className="flex h-96 items-center justify-center font-mono text-xs text-muted">
        Connecting to spawnt Mission Control...
      </div>
    );
  }

  return (
    <div className="sp-console-shell max-w-7xl mx-auto px-6 py-8 font-sans">
      {/* Top Banner if not authenticated */}
      {!user ? (
        <div className="mb-6 p-4 rounded-lg bg-card border border-accent/40 font-mono text-xs flex justify-between items-center flex-wrap gap-3">
          <div>
            <span className="text-accent font-semibold">[CLOUD CONSOLE]</span> Sign in to view reports from your paired desktop.
          </div>
          <div className="flex items-center gap-2">
            <Link
              href="/auth"
              className="px-3 py-1.5 bg-accent text-bg font-semibold rounded hover:opacity-90 transition-opacity text-xs"
            >
              Sign In with GitHub &rarr;
            </Link>
          </div>
        </div>
      ) : (
        <div className="mb-6 p-3 rounded-lg bg-card border border-border font-mono text-xs flex justify-between items-center flex-wrap gap-3">
          <div className="flex items-center gap-2">
            <span className="w-2 h-2 bg-green animate-pulse"></span>
            <span>Cloud Account: <strong className="text-foreground">{user.email}</strong></span>
          </div>
          <button
            onClick={async () => {
              const { data: { session } } = await supabase.auth.getSession();
              if (session) {
                await navigator.clipboard.writeText(JSON.stringify(session));
                setCopyFeedback("Pairing token copied! Paste into Desktop App -> Auth tab.");
                setTimeout(() => setCopyFeedback(""), 4000);
              }
            }}
            className="px-3 py-1 bg-accent/20 border border-accent/40 text-accent font-semibold rounded hover:bg-accent hover:text-bg transition-colors text-xs"
          >
            {copyFeedback || "[Copy Pairing Token for Desktop App]"}
          </button>
        </div>
      )}

      {/* Pairing help when no desktop has uploaded a snapshot */}
      {!report && (
        <div className="mb-6 p-4 rounded-lg bg-card border border-border font-mono text-xs flex justify-between items-center flex-wrap gap-3">
          <div>
            <span className="text-accent font-semibold">[NOTICE]</span> Pair the spawnt desktop app with this account to monitor it from this device. The desktop daemon uploads a private snapshot every minute while running.
          </div>
          <a
            href="/downloads"
            className="px-3 py-1.5 bg-accent text-bg font-semibold rounded hover:opacity-90 transition-opacity text-xs"
          >
            Download Desktop App &rarr;
          </a>
        </div>
      )}

      {/* Header Controls */}
      <div className="flex justify-between items-center mb-6 flex-wrap gap-4 font-mono sp-console-heading">
        <div className="flex items-center gap-3">
          <span className="sp-console-mark sp-brand-mark" aria-hidden="true" />
          <div>
            <div className="text-xl font-bold text-foreground">spawnt / {activeView}</div>
            <div className="text-xs text-muted flex items-center gap-2 mt-0.5">
              <span className={`w-2 h-2 ${connectedSource.startsWith("Remote daemon") ? "bg-green animate-pulse" : "bg-red"}`}></span>
              <span>Status: {connectedSource}</span>
            </div>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <button type="button" onClick={refreshCloudSnapshot} disabled={refreshing} className="px-3 py-2 bg-card border border-border text-xs text-foreground hover:border-accent disabled:opacity-50">{refreshing ? "Refreshing..." : "Reload view"}</button>
          <Link
            href="/settings"
            className="px-3 py-1.5 bg-card border border-border rounded text-xs text-foreground hover:border-accent"
          >
            Settings
          </Link>
        </div>
      </div>

      <div className="mb-4 flex flex-wrap items-center justify-between gap-2 font-mono text-[11px] text-muted" aria-live="polite">
        <span>{refreshMessage || connectedSource}</span>
        <span>{lastSuccessfulUpdate ? `Last successful view refresh: ${lastSuccessfulUpdate.toLocaleTimeString()}` : "Last successful view refresh: not yet"}</span>
      </div>

      <nav className="sp-console-nav" aria-label="Console views">
        {["overview", "sessions", "usage", "limits", "projects", "automations", "settings"].map((view) => <button key={view} type="button" aria-current={activeView === view ? "page" : undefined} onClick={() => view === "settings" ? window.location.assign("/settings") : changeConsoleView(view)}>{view}</button>)}
      </nav>
      {activeView !== "overview" && activeView !== "sessions" && activeView !== "usage" && activeView !== "limits" && <div className="sp-console-notice" role="status">{activeView === "projects" ? (sessions.some((s) => s.project_name) ? "Project names are available for the sessions shown below." : "Project identity is not available in the current cloud snapshot.") : "Automation controls are managed by the paired local daemon. This account view does not execute local actions."}</div>}

      {/* Timeline Filter Bar */}
      <div className="flex justify-between items-center bg-card border border-border rounded-md px-4 py-2.5 mb-6 flex-wrap gap-3 font-mono">
        <div className="text-xs text-muted">
          Scope: <span className="text-foreground uppercase">{timeline}</span>
          {report?.earliest_at && report?.latest_at && (
            <span className="ml-2 text-muted">
              {(() => {
                const e = new Date(report.earliest_at).toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" });
                const l = new Date(report.latest_at).toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" });
                return e === l ? `(${e})` : `(${e} — ${l})`;
              })()}
            </span>
          )}
        </div>
        <div className="flex gap-2">
          {["today", "7d", "30d", "all"].map((t) => (
            <button
              key={t}
              onClick={() => { setTimeline(t); setPage(1); }}
              className={`px-3 py-1 text-xs rounded border transition-colors ${
                timeline === t
                  ? "bg-accent text-bg border-accent font-semibold"
                  : "border-border text-muted hover:border-accent"
              }`}
            >
              {t === "today" ? "Today" : t === "7d" ? "7 Days" : t === "30d" ? "30 Days" : "All Time"}
            </button>
          ))}
        </div>
      </div>

      {/* 4 KPI Cards */}
      {activeView === "overview" && <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-6 font-mono">
        <div className="bg-card border border-border rounded-lg p-5">
          <div className="text-xs text-muted uppercase">Active Fleet Agents</div>
          <div className="text-2xl font-bold mt-1 text-foreground">{activeCount} <span className="sp-trust">EXACT</span></div>
          <div className="text-[11px] text-muted mt-1">Active in last 15 min</div>
        </div>

        <div className="bg-card border border-border rounded-lg p-5">
          <div className="text-xs text-muted uppercase">API-Equivalent Spend</div>
          <div className="text-2xl font-bold mt-1 text-accent">
            ${report ? report.total_cost_usd.toFixed(2) : "Unknown"} <span className="sp-trust">{report ? "EST" : "UNKNOWN"}</span>
          </div>
          <div className="text-[11px] text-muted mt-1">API-equivalent cost basis</div>
        </div>

        <div className="bg-card border border-border rounded-lg p-5">
          <div className="text-xs text-muted uppercase">Prompt Cache Savings</div>
          <div className="text-2xl font-bold mt-1 text-green">
            ${report ? report.total_cache_savings_usd.toFixed(2) : "Unknown"} <span className="sp-trust">{report ? "EST" : "UNKNOWN"}</span>
          </div>
          <div className="text-[11px] text-green mt-1">Direct Cost Avoided</div>
        </div>

        <div className="bg-card border border-border rounded-lg p-5">
          <div className="text-xs text-muted uppercase">Quota Pools Monitored</div>
          <div className="text-2xl font-bold mt-1 text-foreground">{report ? quota.length : "Unknown"} <span className="sp-trust">{report ? "EXACT" : "UNKNOWN"}</span></div>
          <div className="text-[11px] text-muted mt-1">5h & 7d Window Bounds</div>
        </div>
      </div>}

      {/* Subscription ROI Breakdown */}
      {activeView === "overview" && <div className="bg-card border border-border rounded-lg p-5 mb-6 font-mono">
        <div className="flex justify-between items-center mb-3">
          <div className="text-xs text-muted uppercase font-semibold">Subscription ROI Breakdown</div>
          <Link href="/settings" className="text-xs text-accent hover:underline">
            [Edit Plans]
          </Link>
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 items-start">
          {activeRoi.map((roi) => (
            <div key={roi.provider} className="bg-bg border border-border rounded p-3.5">
              <div className="text-[11px] text-muted uppercase font-semibold">{roi.provider}</div>
              <div className="text-lg font-bold text-foreground my-1">
                {(roi.multiplier ?? 0).toFixed(1)}x Return
              </div>
              <div className={`text-[11px] font-semibold ${(roi.net_savings_usd ?? 0) >= 0 ? "text-green" : "text-red"}`}>
                {(roi.net_savings_usd ?? 0) >= 0 ? "+" : ""}${(roi.net_savings_usd ?? 0).toFixed(2)} vs API
              </div>
              <div className="text-[10px] text-muted mt-2 border-t border-border/40 pt-1.5 space-y-0.5">
                <div>Plan: <span className="text-foreground">${(roi.monthly_plan_usd ?? 20).toFixed(0)}/mo</span> &bull; ${(roi.plan_usd ?? 0).toFixed(2)} ({roi.period_label || "period"})</div>
                <div>Delivered Value: <span className="text-accent font-semibold">${(roi.spend_usd ?? 0).toFixed(2)}</span></div>
              </div>
            </div>
          ))}
          {combinedRoi && (combinedRoi.spend_usd ?? 0) > 0 && (
            <div className="bg-bg border border-accent/40 rounded p-3.5">
              <div className="text-[11px] text-accent uppercase font-semibold">Combined Fleet ROI</div>
              <div className="text-lg font-bold text-accent my-1">
                {(combinedRoi.multiplier ?? 0).toFixed(1)}x Return
              </div>
              <div className={`text-[11px] font-semibold ${(combinedRoi.net_savings_usd ?? 0) >= 0 ? "text-green" : "text-red"}`}>
                {(combinedRoi.net_savings_usd ?? 0) >= 0 ? "+" : ""}${(combinedRoi.net_savings_usd ?? 0).toFixed(2)} Net
              </div>
              <div className="text-[10px] text-muted mt-2 border-t border-border/40 pt-1.5 space-y-0.5">
                <div>Total Plan: <span className="text-foreground">${(combinedRoi.monthly_plan_usd ?? 60).toFixed(0)}/mo</span></div>
                <div>Fleet Value: <span className="text-accent font-semibold">${(combinedRoi.spend_usd ?? 0).toFixed(2)}</span></div>
              </div>
            </div>
          )}
          {activeRoi.length === 0 && (!combinedRoi || (combinedRoi.spend_usd ?? 0) === 0) && (
            <div className="col-span-full text-xs text-muted py-2">
              No subscription usage recorded for this timeframe.
            </div>
          )}
        </div>
      </div>}

      {/* Spend By Agent (Click to View Models) */}
      {(activeView === "overview" || activeView === "usage") && <div className="mb-6 font-mono">
        <div className="text-xs text-muted uppercase mb-3 font-semibold">
          Spend by Agent (Click to View Models)
        </div>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4 items-start">
          {activeAgents.map((prov) => {
            const isExpanded = !!expandedAgents[prov.provider];
            const savingsUsd = prov.total_cache_savings_usd ?? prov.cache_savings_usd ?? 0;
            return (
              <div
                key={prov.provider}
                onClick={() => setExpandedAgents(prev => ({ ...prev, [prov.provider]: !prev[prov.provider] }))}
                className={`bg-card border rounded-lg p-4 cursor-pointer transition-colors ${
                  isExpanded ? "border-accent" : "border-border hover:border-accent/60"
                }`}
              >
                <div className="flex justify-between items-center mb-1">
                  <div className="text-sm font-semibold">{prov.provider}</div>
                  <div className="text-sm font-bold text-foreground">
                    ${(prov.total_cost_usd ?? 0).toFixed(2)}
                  </div>
                </div>
                <div className="text-[11px] text-muted">
                  {prov.models?.length || 0} model(s) &bull; ${savingsUsd.toFixed(2)} cached
                </div>

                {isExpanded && (
                  <div className="mt-3 pt-3 border-t border-border space-y-2">
                    {prov.models?.map((m) => (
                      <div key={m.model} className="text-[11px] flex justify-between items-center bg-bg p-2 rounded">
                        <div>
                          <div className="font-semibold text-foreground">{m.model}</div>
                          <div className="text-muted text-[10px]">
                            In: {formatTokens(m.input_tokens)} | Out: {formatTokens(m.output_tokens)}
                          </div>
                        </div>
                        <div className="font-bold text-foreground">${(m.cost_usd ?? 0).toFixed(2)}</div>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            );
          })}
          {activeAgents.length === 0 && (
            <div className="col-span-3 bg-card border border-border rounded-lg p-6 text-center text-xs text-muted">
              No agent spending recorded for the selected scope ({timeline.toUpperCase()}).
            </div>
          )}
        </div>
      </div>}

      {/* Per-Model Quota Pools */}
      {(activeView === "overview" || activeView === "limits") && <div className="bg-card border border-border rounded-lg p-5 mb-6 font-mono">
        <div className="flex justify-between items-center mb-3">
          <div className="text-xs text-muted uppercase font-semibold">Per-Model Quota Pools</div>
          <div className="text-[11px] text-muted">Source and trust are shown per pool.</div>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full text-xs text-left">
            <thead className="text-[11px] text-muted border-b border-border uppercase">
              <tr>
                <th className="pb-2">Provider</th>
                <th className="pb-2">Pool ID</th>
                <th className="pb-2">Window</th>
                <th className="pb-2">Used %</th>
                <th className="pb-2">Resets At</th>
                <th className="pb-2 text-right">Basis</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-border/40">
              {quota.map((q, idx) => (
                <tr key={`${q.provider}-${q.limit_id}-${idx}`} className="hover:bg-bg/40">
                  <td className="py-2.5 font-semibold">{q.provider}</td>
                  <td className="py-2.5">{q.limit_id}</td>
                  <td className="py-2.5 capitalize">{q.window ? q.window.replace("_", "-") : "rolling"}</td>
                  <td className="py-2.5">
                    <span className={`font-bold ${(q.used_pct ?? 0) >= 90 ? "text-red" : (q.used_pct ?? 0) >= 60 ? "text-accent" : "text-green"}`}>
                      {typeof q.used_pct === "number" ? q.used_pct.toFixed(1) : "Unknown"}% <span className="sp-trust">{q.basis?.toUpperCase() || "UNKNOWN"}</span>
                    </span>
                  </td>
                  <td className="py-2.5 text-muted">
                    {formatResetCountdown(q.resets_at, q.window)}
                  </td>
                  <td className="py-2.5 text-right">
                    <span
                      className={`px-1.5 py-0.5 rounded text-[10px] font-bold ${
                        q.basis?.toLowerCase() === "estimated"
                          ? "bg-card text-muted border border-border"
                          : "bg-accent/15 text-accent"
                      }`}
                    >
                      {q.basis?.toUpperCase() || "UNKNOWN"}
                    </span>
                  </td>
                </tr>
              ))}
              {quota.length === 0 && (
                <tr>
                  <td colSpan={6} className="py-4 text-center text-muted">
                    No quota pools are present in the latest account snapshot.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </div>}

      {viewError && <div className="sp-console-notice" role="status">{viewError}</div>}
      {activeView === "usage" && <section className="sp-usage-panel" aria-labelledby="usage-title">
        <div className="sp-panel-heading"><div><span className="sp-console-eyebrow">USAGE / {timeline.toUpperCase()}</span><h2 id="usage-title">Usage breakdown</h2></div><span className="sp-trust">{report ? "EST" : "UNKNOWN"}</span></div>
        {report ? <><div className="sp-usage-summary"><strong>{formatTokens(tokenTotal)} <span className="sp-trust">EXACT</span></strong><span>API-equivalent cost: ${report.total_cost_usd.toFixed(2)} <span className="sp-trust">EST</span></span></div><div className="sp-usage-bars" role="img" aria-label={`Cost by provider, maximum $${maxChartValue.toFixed(2)}`}>{chartValues.map((item) => <div className="sp-usage-bar-row" key={item.label}><span>{item.label}</span><div><i style={{ "--bar-width": `${maxChartValue ? item.value / maxChartValue * 100 : 0}%` } as React.CSSProperties} /></div><b>${item.value.toFixed(2)} <span className="sp-trust">EST</span></b></div>)}</div><p className="sp-console-caption">Provider totals are API-equivalent estimates from the Rust timeline report. Token counts are read from the report.</p></> : <div className="sp-console-empty">Usage data is unavailable in the current cloud snapshot. Values are unknown, not zero.</div>}
      </section>}
      {activeView === "limits" && <section className="sp-usage-panel" aria-labelledby="limits-title">
        <div className="sp-panel-heading"><div><span className="sp-console-eyebrow">LIMITS / QUOTA POOLS</span><h2 id="limits-title">Remaining capacity</h2></div><span className="sp-trust">PER POOL</span></div>
        {quota.length ? <div className="sp-limit-list">{quota.map((pool) => { const used = typeof pool.used_pct === "number" ? Math.min(100, Math.max(0, pool.used_pct)) : null; return <div className="sp-limit-row" key={`${pool.provider}-${pool.limit_id}-${pool.window}`}><div><strong>{pool.provider} / {pool.limit_id}</strong><span>{pool.window.replace("_", " ")} · resets {formatResetCountdown(pool.resets_at, pool.window)}</span></div><div className="sp-limit-meter" role="img" aria-label={used === null ? "Quota remaining unknown" : `${100 - used}% remaining`}><i style={{ "--bar-width": `${used === null ? 0 : 100 - used}%` } as React.CSSProperties} /></div><b>{used === null ? "Unknown" : `${(100 - used).toFixed(1)}% left`} <span className="sp-trust">{pool.basis?.toUpperCase() || "UNKNOWN"}</span></b></div>; })}</div> : <div className="sp-console-empty">No quota pools are available in the current account snapshot. Remaining capacity and reset time are unknown.</div>}
      </section>}
      <p className="text-xs text-muted mb-3">Cloud usage and estimated cost appear in session rows. Summary charts show desktop reports.</p>
      {/* Live & Historical Agent Sessions */}
      {(activeView === "overview" || activeView === "sessions" || activeView === "projects") && <div className="bg-card border border-border rounded-lg p-5 font-mono">
        <div className="flex justify-between items-center mb-4 flex-wrap gap-3">
          <div className="text-xs text-muted uppercase font-semibold">Live & Historical Agent Sessions</div>
          <div className="flex gap-2">
            <select
              value={agentFilter}
              onChange={(e) => { setAgentFilter(e.target.value); setPage(1); }}
              className="bg-bg border border-border text-xs rounded px-2.5 py-1 text-foreground focus:outline-none"
            >
              <option value="all">All Agents</option>
              <option value="claude-code">Claude Code</option>
              <option value="codex">Codex</option>
              <option value="antigravity">Antigravity</option>
              <option value="gemini-cli">Gemini CLI</option>
            </select>
            <select
              value={statusFilter}
              onChange={(e) => { setStatusFilter(e.target.value); setPage(1); }}
              className="bg-bg border border-border text-xs rounded px-2.5 py-1 text-foreground focus:outline-none"
            >
              <option value="all">All Statuses</option>
              <option value="active">Active Only</option>
              <option value="idle">Idle Only</option>
            </select>
          </div>
        </div>

        {sessions.some((s) => s.context_window && s.context_tokens && (s.context_tokens / s.context_window) >= 0.9) ? (
          <div className="mb-3 px-3 py-2 bg-red/10 border border-red/30 rounded text-xs text-red flex items-center justify-between flex-wrap gap-2">
            <div className="flex items-center gap-2">
              <span className="font-mono font-bold">[PRO LABS]</span>
              <span>Context saturation reserve active (&gt;=90%). Sessions are eligible for zero-token handoff via <code className="font-mono text-[11px] bg-red/20 px-1 py-0.5 rounded">.fleethud/HANDOFF.md</code>.</span>
            </div>
            {sessions.find((s) => s.context_window && s.context_tokens && (s.context_tokens / s.context_window) >= 0.9) ? (
              <button
                onClick={() => {
                  const target = sessions.find((s) => s.context_window && s.context_tokens && (s.context_tokens / s.context_window) >= 0.9);
                  if (target) handleRemoteRelay(target);
                }}
                className="px-2.5 py-1 bg-red text-white text-[11px] font-semibold rounded hover:opacity-90 transition-opacity"
              >
                Handoff & Rotate Saturated Session
              </button>
            ) : null}
          </div>
        ) : null}

        <div className="overflow-x-auto">
          <table className="w-full text-xs text-left">
            <thead className="text-[11px] text-muted border-b border-border uppercase">
              <tr>
                <th className="pb-2">Project</th>
                <th className="pb-2">Agent</th>
                <th className="pb-2">Surface</th>
                <th className="pb-2">Model</th>
                <th className="pb-2">Tokens</th>
                <th className="pb-2">Context</th>
                <th className="pb-2">Cost</th>
                <th className="pb-2">Status</th>
                <th className="pb-2 text-right">Last Active</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-border/40">
              {sessions.map((s) => {
                const totalTokens = (s.input_tokens || 0) + (s.output_tokens || 0) + (s.cache_read_tokens || 0);
                const ctxTokens = s.context_tokens || 0;
                const ctxWin = s.context_window || 0;
                const fillPct = ctxWin > 0 ? Math.min(100, Math.round((ctxTokens / ctxWin) * 100)) : 0;
                const barColor = fillPct > 85 ? "bg-red" : fillPct > 60 ? "bg-yellow" : "bg-accent";

                return (
                  <tr key={s.id} className="hover:bg-bg/40">
                    <td className="py-2.5">
                      <span className="font-semibold text-foreground block">{s.project_name}</span>
                      {s.title ? (
                        <span className="text-[11px] text-muted block truncate max-w-[200px]" title={s.title}>
                          {s.title}
                        </span>
                      ) : null}
                    </td>
                    <td className="py-2.5 capitalize">{s.agent}</td>
                    <td className="py-2.5">
                      <span className="capitalize block">{s.surface.replace("_", " ")}</span>
                      <span className="text-[10px] text-muted block font-mono">
                        {s.device_name || "local"}
                      </span>
                    </td>
                    <td className="py-2.5">{s.model}</td>
                    <td className="py-2.5">
                      <span className="font-semibold text-foreground">
                        {s.usage_available === false ? <>Unknown <span className="sp-trust">UNKNOWN</span></> : <>{formatTokens(totalTokens)} <span className="sp-trust">EXACT</span></>}
                      </span>
                      <span className="text-[10px] text-muted block">
                        {s.usage_available === false ? "No measured usage" : `${formatTokens(s.input_tokens)} in · ${formatTokens(s.output_tokens)} out`}
                      </span>
                      {s.cache_read_tokens && s.cache_read_tokens > 0 ? (
                        <span className="text-[10px] text-green block font-semibold">
                          + {formatTokens(s.cache_read_tokens)} cache
                        </span>
                      ) : null}
                    </td>
                    <td className="py-2.5">
                      {ctxWin > 0 ? (
                        <div>
                          <div className="flex items-center gap-1 font-semibold text-foreground">
                            <span>{formatTokens(ctxTokens)}</span>
                            <span className="text-[10px] text-muted font-normal">/ {formatTokens(ctxWin)}</span>
                            <span className="text-[10px] text-muted font-normal">({fillPct}% <span className="sp-trust">EXACT</span>)</span>
                          </div>
                          <div className="w-20 bg-border h-1.5 rounded-sm overflow-hidden mt-1">
                            <div className={`h-full ${barColor}`} style={{ width: `${fillPct}%` }} />
                          </div>
                          {fillPct >= 90 ? (
                            <button
                              onClick={() => handleRemoteRelay(s)}
                              className="mt-1 text-[10px] font-semibold text-accent hover:underline block"
                              title="Handoff to clean session with 0 context tokens via .fleethud/HANDOFF.md"
                            >
                              [Handoff & Rotate]
                            </button>
                          ) : null}
                        </div>
                      ) : ctxTokens > 0 ? (
                        <span className="text-foreground font-semibold">{formatTokens(ctxTokens)}</span>
                      ) : (
                        <span className="text-muted">-</span>
                      )}
                    </td>
                    <td className="py-2.5 font-bold text-foreground">
                      {s.usage_available === false || s.cost_basis === "unknown" ? <>Unknown <span className="sp-trust">UNKNOWN</span></> : <>${(s.total_cost_usd ?? 0).toFixed(2)} <span className="sp-trust">{s.cost_basis?.toUpperCase() || "UNKNOWN"}</span></>}
                    </td>
                    <td className="py-2.5">
                      <span className="flex items-center gap-1.5">
                        <span className={`w-2 h-2 ${s.is_active ? "bg-green" : "bg-muted"}`}></span>
                        <span className={s.is_active ? "text-green font-semibold capitalize" : "text-muted capitalize"}>
                          {s.state} <span className="sp-trust">EXACT</span>
                        </span>
                      </span>
                    </td>
                    <td className="py-2.5 text-right text-muted">{formatTimeAgo(s.last_event_at)}</td>
                  </tr>
                );
              })}
              {sessions.length === 0 && (
                <tr>
                  <td colSpan={9} className="py-6 text-center text-muted">
                    No matching agent sessions found. Run Claude Code, Codex, or Antigravity on your machine.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>

        {/* Pagination Controls */}
        <div className="flex justify-between items-center mt-4 pt-3 border-t border-border text-xs text-muted">
          <div>
            Page {page} of {totalPages} ({totalCount} sessions)
          </div>
          <div className="flex gap-2">
            <button
              disabled={page <= 1}
              onClick={() => setPage((p) => Math.max(1, p - 1))}
              className="px-3 py-1 bg-bg border border-border rounded disabled:opacity-40"
            >
              Previous
            </button>
            <button
              disabled={page >= totalPages}
              onClick={() => setPage((p) => p + 1)}
              className="px-3 py-1 bg-bg border border-border rounded disabled:opacity-40"
            >
              Next
            </button>
          </div>
        </div>
      </div>}
    </div>
  );
}
