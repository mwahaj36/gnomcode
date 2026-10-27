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
  if (!Number.isFinite(target.getTime())) return "Unknown";
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
  const [quickTokenInput, setQuickTokenInput] = useState("");
  const [showTokenModal, setShowTokenModal] = useState(false);
  const [copyFeedback, setCopyFeedback] = useState("");

  // Live Metrics
  const [report, setReport] = useState<TimelineReport | null>(null);
  const [sessions, setSessions] = useState<SessionItem[]>([]);
  const [quota, setQuota] = useState<QuotaItem[]>([]);
  const [roiByProvider, setRoiByProvider] = useState<PlanRoiInfo[]>([]);
  const [combinedRoi, setCombinedRoi] = useState<PlanRoiInfo | null>(null);
  const [userProfile, setUserProfile] = useState<UserProfile | null>(null);
  const [activeCount, setActiveCount] = useState<number | null>(null);
  const [snapshotLoaded, setSnapshotLoaded] = useState(false);
  const [totalCount, setTotalCount] = useState<number>(0);
  const [totalPages, setTotalPages] = useState<number>(1);
  const [connectedSource, setConnectedSource] = useState<string>("Scanning...");
  const [refreshing, setRefreshing] = useState(false);
  const [lastSuccessfulUpdate, setLastSuccessfulUpdate] = useState<Date | null>(null);
  const [refreshMessage, setRefreshMessage] = useState("");
  const refreshInFlight = useRef(false);
  const manualRefreshInFlight = useRef(false);
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
      if (session && typeof window !== "undefined") {
        const cb = localStorage.getItem("fleethud_auth_callback");
        if (cb) {
          localStorage.removeItem("fleethud_auth_callback");
          window.location.href = `${cb}?session=${encodeURIComponent(JSON.stringify(session))}`;
        }
      }
    });

    const { data: { subscription } } = supabase.auth.onAuthStateChange((_event, session) => {
      setUser(session?.user ?? null);
    });

    return () => subscription.unsubscribe();
  }, []);

  // Live metrics polling
  const fetchLiveMetrics = useCallback(async (force = false) => {
    if (refreshInFlight.current && !force) return false;
    if (refreshInFlight.current && force) await new Promise<void>((resolve) => {
      const check = () => refreshInFlight.current ? window.setTimeout(check, 50) : resolve();
      check();
    });
    if (refreshInFlight.current) return false;
    refreshInFlight.current = true;
    setViewError("");
    let refreshed = false;
    try {
    if (user) {
      try {
        const [desktop, cloud] = await Promise.all([
          supabase.from("fleethud_sync_state").select("*").eq("user_id", user.id).maybeSingle(),
          supabase.from("fleethud_cloud_sessions_live").select("session,updated_at").eq("user_id", user.id).order("updated_at", { ascending: false }).limit(1000),
        ]);
        const snap = desktop.data?.live_snapshot;
        if (desktop.error || cloud.error) setViewError("Some cloud data is unavailable. Last received data remains visible where available.");
        refreshed = !desktop.error && !cloud.error;
        if (refreshed) setLastSuccessfulUpdate(new Date());
        const cloudSessions: SessionItem[] = (cloud.data || []).map((row) => row.session as SessionItem);
        const allSessions: SessionItem[] = Array.from(new Map<string, SessionItem>([...(snap?.sessions || []), ...cloudSessions].map((session: SessionItem) => [session.id, session])).values());
        const filtered = allSessions.sort((a, b) => Date.parse(b.last_event_at) - Date.parse(a.last_event_at)).filter((session) =>
          (agentFilter === "all" || session.agent === agentFilter) &&
          (statusFilter === "all" || (statusFilter === "active" ? session.is_active : !session.is_active))
        );
        setSessions(filtered.slice((page - 1) * 10, page * 10));
        setReport(snap?.timelines?.[timeline] || snap?.timeline || null);
        setQuota(snap?.quota || []);
        setRoiByProvider(snap?.roi_by_provider || []);
        setCombinedRoi(snap?.combined_roi || null);
        setUserProfile(snap?.user_profile || null);
        setSnapshotLoaded(Boolean(snap));
        setActiveCount(snap?.active_count ?? null);
        setTotalCount(filtered.length);
        setTotalPages(Math.max(1, Math.ceil(filtered.length / 10)));
        const cloudStatus = cloud.error ? "Cloud sessions unavailable" : `${cloudSessions.length} cloud sessions · EXACT`;
        if (snap) {
          const ageMs = Date.now() - Date.parse(desktop.data.updated_at);
          setConnectedSource(`${ageMs < 120000 ? "Desktop connected" : "Desktop data stale"} · ${cloudStatus}`);
        } else {
          setConnectedSource(desktop.error ? `Desktop unavailable · ${cloudStatus}` : cloudStatus);
        }
      } catch {
        setConnectedSource("Offline");
        setViewError("Connection failed. Showing the last data held in this page.");
      }
    } else {
      setConnectedSource("Sign in to monitor your desktop remotely");
      setSnapshotLoaded(false);
      setActiveCount(null);
    }
    } finally {
    refreshInFlight.current = false;
    }
    return refreshed;
  }, [user, timeline, agentFilter, statusFilter, page]);

  const refreshCloudSnapshot = async () => {
    if (manualRefreshInFlight.current || refreshing) return;
    if (!user) {
      setRefreshMessage("Sign in to reload the cloud view.");
      return;
    }
    manualRefreshInFlight.current = true;
    setRefreshing(true);
    setRefreshMessage("Refreshing cloud view...");
    try {
      const refreshed = await fetchLiveMetrics(true);
      if (refreshed) {
        setLastSuccessfulUpdate(new Date());
        setRefreshMessage("Cloud view refreshed. New desktop snapshots are published by the paired daemon.");
      } else {
        setRefreshMessage("Refresh failed. One or more cloud reads did not complete; showing available data.");
      }
    } catch {
      setRefreshMessage("Refresh failed. Showing the last available data.");
    } finally {
      manualRefreshInFlight.current = false;
      setRefreshing(false);
    }
  };

  useEffect(() => {
    fetchLiveMetrics();
    const timer = setInterval(fetchLiveMetrics, 15000);
    return () => clearInterval(timer);
  }, [fetchLiveMetrics]);

  const formatTokens = (num: number | undefined | null) => {
    if (typeof num !== "number" || !Number.isFinite(num)) return "Unknown";
    const val = num;
    if (val >= 1_000_000) return (val / 1_000_000).toFixed(1) + "M";
    if (val >= 1_000) return (val / 1_000).toFixed(1) + "k";
    return val.toString();
  };

  const formatTimeAgo = (iso: string) => {
    const timestamp = new Date(iso).getTime();
    if (!Number.isFinite(timestamp)) return "Unknown";
    const diff = (Date.now() - timestamp) / 1000;
    if (diff < 60) return "Just now";
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
    return `${Math.floor(diff / 86400)}d ago`;
  };

  const copyQuickToken = async () => {
    try {
      const { data } = await supabase.auth.getSession();
      if (data?.session) {
        const tokenStr = btoa(JSON.stringify(data.session));
        await navigator.clipboard.writeText(tokenStr);
        setCopyFeedback("Copied pairing token!");
        setTimeout(() => setCopyFeedback(""), 3500);
      }
    } catch {
      setCopyFeedback("Failed to copy");
    }
  };

  const applyQuickToken = async () => {
    try {
      const raw = atob(quickTokenInput.trim());
      const session = JSON.parse(raw);
      await supabase.auth.setSession(session);
      setUser(session.user);
      setShowTokenModal(false);
      setQuickTokenInput("");
    } catch {
      alert("Invalid pairing token format. Please copy fresh from your web browser session.");
    }
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
    <main className="sp-console-shell sp-console-app mx-auto py-10 font-sans">
      {/* Quick Token Pairing Modal */}
      {showTokenModal && (
        <div className="fixed inset-0 z-50 bg-bg/80 backdrop-blur-sm flex items-center justify-center p-4">
          <div className="bg-card border border-border rounded-xl p-6 max-w-md w-full font-mono text-xs shadow-2xl">
            <div className="text-sm font-bold text-foreground mb-1">Quick Pair Desktop App</div>
            <p className="text-muted mb-3 text-[11px]">
              If you signed into GitHub in your browser, choose <strong>Copy desktop token</strong> there and paste it here to authenticate without typing your password.
            </p>
            <textarea
              rows={4}
              value={quickTokenInput}
              onChange={(e) => setQuickTokenInput(e.target.value)}
              placeholder="Paste pairing token here..."
              className="w-full bg-bg border border-border rounded p-2 text-foreground mb-3 focus:outline-none focus:border-accent"
            />
            <div className="flex justify-end gap-2">
              <button
                onClick={() => setShowTokenModal(false)}
                className="px-3 py-1.5 bg-card border border-border rounded text-muted hover:text-foreground"
              >
                Cancel
              </button>
              <button
                onClick={applyQuickToken}
                className="px-3 py-1.5 bg-accent text-bg font-semibold rounded hover:opacity-90"
              >
                Connect Session
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Header Controls */}
      <header className="sp-console-heading">
        <div className="sp-console-brand-lockup">
          <span className="sp-console-mark sp-brand-mark" aria-hidden="true" />
          <div>
            <div className="sp-console-title">{activeView === "overview" ? "Your agent fleet" : activeView[0].toUpperCase() + activeView.slice(1)}</div>
            <div className="sp-console-status">
              <span className={`w-2 h-2 ${connectedSource.startsWith("Remote daemon") ? "bg-green animate-pulse" : "bg-red"}`}></span>
              <span>{connectedSource}</span>
            </div>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <button type="button" onClick={refreshCloudSnapshot} disabled={refreshing || !user} className="sp-console-action" aria-busy={refreshing}>{refreshing ? "Refreshing…" : "Reload view"}</button>
          <Link
            href="/settings"
            className="sp-console-action"
          >
            Settings
          </Link>
        </div>
      </header>

      {!user && <section className="sp-console-connect" aria-label="Desktop connection">
        <p>Sign in to view synced desktop data, or pair a desktop app with an existing browser account.</p>
        <div className="sp-console-connect-actions">
          <button onClick={() => setShowTokenModal(true)} className="sp-console-link">Pair desktop</button>
          <Link href="/auth" className="sp-console-link sp-console-link-primary">Sign in</Link>
        </div>
      </section>}

      <div className="sp-console-refresh-state" aria-live="polite">
        <span>{refreshMessage || connectedSource}</span>
        <span>{lastSuccessfulUpdate ? `Last successful view refresh: ${lastSuccessfulUpdate.toLocaleTimeString()}` : "Last successful view refresh: not yet"}</span>
      </div>

      <nav className="sp-console-nav" aria-label="Console views">
        {["overview", "sessions", "usage", "limits", "projects", "automations", "settings"].map((view) => <button key={view} type="button" aria-current={activeView === view ? "page" : undefined} onClick={() => view === "settings" ? window.location.assign("/settings") : changeConsoleView(view)}>{view}</button>)}
      </nav>
      {activeView !== "overview" && activeView !== "sessions" && activeView !== "usage" && activeView !== "limits" && <div className="sp-console-notice" role="status">{activeView === "projects" ? (sessions.some((s) => s.project_name) ? "Project names are available for the sessions shown below." : "Project identity is not available in the current cloud snapshot.") : "Automation controls are managed by the paired local daemon. This account view does not execute local actions."}</div>}

      {/* Timeline Filter Bar */}
      <div className="sp-console-timeline-filter">
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
              aria-pressed={timeline === t}
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
          <div className="text-2xl font-bold mt-1 text-foreground">{activeCount === null ? "Unknown" : activeCount} <span className="sp-trust">{activeCount === null ? "UNKNOWN" : "EXACT"}</span></div>
          <div className="text-[11px] text-muted mt-1">Active in last 15 min</div>
        </div>

        <div className="bg-card border border-border rounded-lg p-5">
          <div className="text-xs text-muted uppercase">API-Equivalent Spend</div>
          <div className="text-2xl font-bold mt-1 text-accent">
            {report ? `$${report.total_cost_usd.toFixed(2)}` : "Unknown"} <span className="sp-trust">{report ? "EST" : "UNKNOWN"}</span>
          </div>
          <div className="text-[11px] text-muted mt-1">API-equivalent cost basis</div>
        </div>

        <div className="bg-card border border-border rounded-lg p-5">
          <div className="text-xs text-muted uppercase">Prompt Cache Savings</div>
          <div className="text-2xl font-bold mt-1 text-green">
            {report ? `$${report.total_cache_savings_usd.toFixed(2)}` : "Unknown"} <span className="sp-trust">{report ? "EST" : "UNKNOWN"}</span>
          </div>
          <div className="text-[11px] text-green mt-1">Direct Cost Avoided</div>
        </div>

        <div className="bg-card border border-border rounded-lg p-5">
          <div className="text-xs text-muted uppercase">Quota Pools Monitored</div>
          <div className="text-2xl font-bold mt-1 text-foreground">{snapshotLoaded ? quota.length : "Unknown"} <span className="sp-trust">{snapshotLoaded ? "EXACT" : "UNKNOWN"}</span></div>
          <div className="text-[11px] text-muted mt-1">5h & 7d Window Bounds</div>
        </div>
      </div>}

      {/* Subscription ROI Breakdown */}
      {activeView === "overview" && <div className="bg-card border border-border rounded-lg p-5 mb-6 font-mono">
        <div className="flex justify-between items-center mb-3">
          <div className="text-xs text-muted uppercase font-semibold">Subscription ROI Breakdown</div>
          <Link href="/settings" className="text-xs text-accent hover:underline">
            Edit plans
          </Link>
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 items-start">
          {activeRoi.map((roi) => (
              <div key={roi.provider} className="bg-bg border border-border rounded p-3.5">
              <div className="text-[11px] text-muted uppercase font-semibold">{roi.provider}</div>
              <div className="text-lg font-bold text-foreground my-1">
                {typeof roi.multiplier === "number" ? `${roi.multiplier.toFixed(1)}x` : "Unknown"} <span className="sp-trust">{typeof roi.multiplier === "number" ? "EST" : "UNKNOWN"}</span> Return
              </div>
              <div className={`text-[11px] font-semibold ${(roi.net_savings_usd ?? 0) >= 0 ? "text-green" : "text-red"}`}>
                {typeof roi.net_savings_usd === "number" ? `${roi.net_savings_usd >= 0 ? "+" : ""}$${roi.net_savings_usd.toFixed(2)}` : "Unknown"} <span className="sp-trust">{typeof roi.net_savings_usd === "number" ? "EST" : "UNKNOWN"}</span> vs API
              </div>
              <div className="text-[10px] text-muted mt-2 border-t border-border/40 pt-1.5 space-y-0.5">
                <div>Plan: <span className="text-foreground">{typeof roi.monthly_plan_usd === "number" ? `$${roi.monthly_plan_usd.toFixed(0)}/mo` : "Unknown"} <span className="sp-trust">{typeof roi.monthly_plan_usd === "number" ? "EXACT" : "UNKNOWN"}</span></span> · {typeof roi.plan_usd === "number" ? `$${roi.plan_usd.toFixed(2)}` : "Unknown"} <span className="sp-trust">{typeof roi.plan_usd === "number" ? "EXACT" : "UNKNOWN"}</span> ({roi.period_label || "period"})</div>
                <div>Delivered value: <span className="text-accent font-semibold">{typeof roi.spend_usd === "number" ? `$${roi.spend_usd.toFixed(2)}` : "Unknown"} <span className="sp-trust">{typeof roi.spend_usd === "number" ? "EST" : "UNKNOWN"}</span></span></div>
              </div>
            </div>
          ))}
          {combinedRoi && (combinedRoi.spend_usd ?? 0) > 0 && (
            <div className="bg-bg border border-accent/40 rounded p-3.5">
              <div className="text-[11px] text-accent uppercase font-semibold">Combined Fleet ROI</div>
              <div className="text-lg font-bold text-accent my-1">
                {typeof combinedRoi.multiplier === "number" ? `${combinedRoi.multiplier.toFixed(1)}x` : "Unknown"} <span className="sp-trust">{typeof combinedRoi.multiplier === "number" ? "EST" : "UNKNOWN"}</span> Return
              </div>
              <div className={`text-[11px] font-semibold ${typeof combinedRoi.net_savings_usd !== "number" ? "text-muted" : combinedRoi.net_savings_usd >= 0 ? "text-green" : "text-red"}`}>
                {typeof combinedRoi.net_savings_usd === "number" ? `${combinedRoi.net_savings_usd >= 0 ? "+" : ""}$${combinedRoi.net_savings_usd.toFixed(2)}` : "Unknown"} <span className="sp-trust">{typeof combinedRoi.net_savings_usd === "number" ? "EST" : "UNKNOWN"}</span> Net
              </div>
              <div className="text-[10px] text-muted mt-2 border-t border-border/40 pt-1.5 space-y-0.5">
                <div>Total plan: <span className="text-foreground">{typeof combinedRoi.monthly_plan_usd === "number" ? `$${combinedRoi.monthly_plan_usd.toFixed(0)}/mo` : "Unknown"} <span className="sp-trust">{typeof combinedRoi.monthly_plan_usd === "number" ? "EXACT" : "UNKNOWN"}</span></span></div>
                <div>Fleet value: <span className="text-accent font-semibold">{typeof combinedRoi.spend_usd === "number" ? `$${combinedRoi.spend_usd.toFixed(2)}` : "Unknown"} <span className="sp-trust">{typeof combinedRoi.spend_usd === "number" ? "EST" : "UNKNOWN"}</span></span></div>
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
            const savingsUsd = prov.total_cache_savings_usd ?? prov.cache_savings_usd;
            return (
              <div
                key={prov.provider}
                onClick={() => setExpandedAgents(prev => ({ ...prev, [prov.provider]: !prev[prov.provider] }))}
                onKeyDown={(event) => { if (event.key === "Enter" || event.key === " ") { event.preventDefault(); setExpandedAgents(prev => ({ ...prev, [prov.provider]: !prev[prov.provider] })); } }}
                role="button"
                tabIndex={0}
                aria-expanded={isExpanded}
                aria-label={`${prov.provider} spend details`}
                className={`bg-card border rounded-lg p-4 cursor-pointer transition-colors ${
                  isExpanded ? "border-accent" : "border-border hover:border-accent/60"
                }`}
              >
                <div className="flex justify-between items-center mb-1">
                  <div className="text-sm font-semibold">{prov.provider}</div>
                  <div className="text-sm font-bold text-foreground">
                    {typeof prov.total_cost_usd === "number" ? `$${prov.total_cost_usd.toFixed(2)}` : "Unknown"} <span className="sp-trust">{typeof prov.total_cost_usd === "number" ? "EST" : "UNKNOWN"}</span>
                  </div>
                </div>
                <div className="text-[11px] text-muted">
                  {prov.models?.length ?? 0} model(s) <span className="sp-trust">EXACT</span> · {typeof savingsUsd === "number" ? `$${savingsUsd.toFixed(2)}` : "Unknown"} cached <span className="sp-trust">{typeof savingsUsd === "number" ? "EST" : "UNKNOWN"}</span>
                </div>

                {isExpanded && (
                  <div className="mt-3 pt-3 border-t border-border space-y-2">
                    {prov.models?.map((m) => (
                      <div key={m.model} className="text-[11px] flex justify-between items-center bg-bg p-2 rounded">
                        <div>
                          <div className="font-semibold text-foreground">{m.model}</div>
                          <div className="text-muted text-[10px]">
                            In: {formatTokens(m.input_tokens)} | Out: {formatTokens(m.output_tokens)} <span className="sp-trust">EXACT</span>
                          </div>
                        </div>
                        <div className="font-bold text-foreground">{typeof m.cost_usd === "number" ? `$${m.cost_usd.toFixed(2)}` : "Unknown"} <span className="sp-trust">{typeof m.cost_usd === "number" ? "EST" : "UNKNOWN"}</span></div>
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
      {activeView === "overview" && <div className="bg-card border border-border rounded-lg p-5 mb-6 font-mono">
        <div className="flex justify-between items-center mb-3">
          <div className="text-xs text-muted uppercase font-semibold">Per-Model Quota Pools</div>
          <div className="text-[11px] text-muted">Source and trust are shown per pool.</div>
        </div>
        <div className="overflow-x-auto" role="region" aria-label="Quota pool table" tabIndex={0}>
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
                    <span className={`font-bold ${typeof q.used_pct !== "number" ? "text-muted" : q.used_pct >= 90 ? "text-red" : q.used_pct >= 60 ? "text-accent" : "text-green"}`}>
                      {typeof q.used_pct === "number" ? q.used_pct.toFixed(1) : "Unknown"}% <span className="sp-trust">{q.basis?.toUpperCase() || "UNKNOWN"}</span>
                    </span>
                  </td>
                  <td className="py-2.5 text-muted">
                    {formatResetCountdown(q.resets_at, q.window)} {q.resets_at ? <span className="sp-trust">EST</span> : null}
                  </td>
                  <td className="py-2.5 text-right">
                    <span className="text-[10px] font-mono text-muted">
                      {q.basis?.toUpperCase() || "UNKNOWN"}
                    </span>
                  </td>
                </tr>
              ))}
              {quota.length === 0 && (
                <tr>
                  <td colSpan={6} className="py-4 text-center text-muted">
                    {snapshotLoaded ? "No quota pools are present in the latest account snapshot." : "Quota data is unavailable; capacity and reset times are unknown."}
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </div>}

      {viewError && <div className="sp-console-notice" role="alert">{viewError}</div>}
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

        {sessions.some((s) => s.surface !== "cloud" && s.context_window && s.context_tokens && (s.context_tokens / s.context_window) >= 0.9) ? (
          <div className="mb-3 px-3 py-2 bg-red/10 border border-red/30 rounded text-xs text-red flex items-center justify-between flex-wrap gap-2">
            <div className="flex items-center gap-2">
              <span className="font-mono font-bold">PRO LABS</span>
              <span>Context saturation reserve active (&gt;=90%). Sessions are eligible for zero-token handoff via <code className="font-mono text-[11px] bg-red/20 px-1 py-0.5 rounded">.fleethud/HANDOFF.md</code>.</span>
            </div>
            {sessions.find((s) => s.surface !== "cloud" && s.context_window && s.context_tokens && (s.context_tokens / s.context_window) >= 0.9) ? (
              <button
                onClick={() => {
                  const target = sessions.find((s) => s.surface !== "cloud" && s.context_window && s.context_tokens && (s.context_tokens / s.context_window) >= 0.9);
                  if (target) handleRemoteRelay(target);
                }}
                className="px-2.5 py-1 bg-red text-white text-[11px] font-semibold rounded hover:opacity-90 transition-opacity"
              >
                Handoff & Rotate Saturated Session
              </button>
            ) : null}
          </div>
        ) : null}

        <div className="overflow-x-auto" role="region" aria-label="Agent sessions table" tabIndex={0}>
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
                const hasTokenUsage = s.usage_available !== false && typeof s.input_tokens === "number" && typeof s.output_tokens === "number";
                const totalTokens = hasTokenUsage ? s.input_tokens! + s.output_tokens! + (s.cache_read_tokens ?? 0) : null;
                const ctxTokens = s.context_tokens;
                const ctxWin = s.context_window;
                const hasContext = typeof ctxTokens === "number" && typeof ctxWin === "number" && ctxWin > 0;
                const fillPct = hasContext ? Math.min(100, Math.round((ctxTokens / ctxWin) * 100)) : null;
                const barColor = (fillPct ?? 0) > 85 ? "bg-red" : (fillPct ?? 0) > 60 ? "bg-yellow" : "bg-accent";

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
                        {!hasTokenUsage ? <>Unknown <span className="sp-trust">UNKNOWN</span></> : <>{formatTokens(totalTokens)} <span className="sp-trust">EXACT</span></>}
                      </span>
                      <span className="text-[10px] text-muted block">
                        {!hasTokenUsage ? "Presence only" : `${formatTokens(s.input_tokens)} in · ${formatTokens(s.output_tokens)} out`}
                      </span>
                      {s.cache_read_tokens && s.cache_read_tokens > 0 ? (
                        <span className="text-[10px] text-green block font-semibold">
                          + {formatTokens(s.cache_read_tokens)} cache <span className="sp-trust">EXACT</span>
                        </span>
                      ) : null}
                    </td>
                    <td className="py-2.5">
                      {hasContext ? (
                        <div>
                          <div className="flex items-center gap-1 font-semibold text-foreground">
                            <span>{formatTokens(ctxTokens)} <span className="sp-trust">EXACT</span></span>
                            <span className="text-[10px] text-muted font-normal">/ {formatTokens(ctxWin)} <span className="sp-trust">EXACT</span></span>
                            <span className="text-[10px] text-muted font-normal">({fillPct}% <span className="sp-trust">EST</span>)</span>
                          </div>
                          <div className="w-20 bg-border h-1.5 rounded-sm overflow-hidden mt-1">
                            <div className={`h-full ${barColor}`} style={{ width: `${fillPct}%` }} />
                          </div>
                          {s.surface !== "cloud" && fillPct !== null && fillPct >= 90 ? (
                            <button
                              onClick={() => handleRemoteRelay(s)}
                              className="mt-1 text-[10px] font-semibold text-accent hover:underline block"
                              title="Handoff to clean session with 0 context tokens via .fleethud/HANDOFF.md"
                            >
                              [Handoff & Rotate]
                            </button>
                          ) : null}
                        </div>
                      ) : typeof ctxTokens === "number" ? (
                        <span className="text-foreground font-semibold">{formatTokens(ctxTokens)} <span className="sp-trust">EXACT</span> / window <span className="sp-trust">UNKNOWN</span></span>
                      ) : (
                        <span className="text-muted">Unknown <span className="sp-trust">UNKNOWN</span></span>
                      )}
                    </td>
                    <td className="py-2.5 font-bold text-foreground">
                      {typeof s.total_cost_usd !== "number" || !s.cost_basis || s.cost_basis === "unknown" ? <>Unknown <span className="sp-trust">UNKNOWN</span></> : <>${s.total_cost_usd.toFixed(2)} <span className="sp-trust">{s.cost_basis.toUpperCase()}</span></>}
                    </td>
                    <td className="py-2.5">
                      <span className="flex items-center gap-1.5">
                        <span className={`w-2 h-2 ${s.is_active ? "bg-green" : "bg-muted"}`}></span>
                        <span className={s.is_active ? "text-green font-semibold capitalize" : "text-muted capitalize"}>
                          {s.state} <span className="sp-trust">EXACT</span>
                        </span>
                      </span>
                    </td>
                    <td className="py-2.5 text-right text-muted">{formatTimeAgo(s.last_event_at)} <span className="sp-trust">EST</span></td>
                  </tr>
                );
              })}
                    {sessions.length === 0 && (
                <tr>
                  <td colSpan={9} className="py-6 text-center text-muted">
                    {viewError ? "Session data could not be refreshed. Check the cloud connection and reload the view." : !snapshotLoaded ? "No desktop snapshot is connected to this account yet." : totalCount === 0 ? "No sessions match these filters." : "No session rows are available on this page."}
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>

        {/* Pagination Controls */}
        <div className="flex justify-between items-center mt-4 pt-3 border-t border-border text-xs text-muted">
          <div>
            Page {page} of {totalPages} <span className="sp-trust">EXACT</span> ({totalCount} sessions <span className="sp-trust">EXACT</span>)
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
    </main>
  );
}
