"use client";

import { useState, useEffect, useCallback, useRef } from "react";
import Link from "next/link";
import SettingsPanel from "../../components/SettingsPanel";
import { supabase } from "../../lib/supabase";
import { updateActivity, updateProviders, updateSessions, renderQuota, renderProjects, updateSummary, updateOngoing } from "../../lib/console-presentation.js";

function SummaryPanel({ activeCount, report, quotaCount }: { activeCount: number | null; report: unknown; quotaCount: number | null }) {
  const element = useRef<HTMLDivElement>(null);
  useEffect(() => { if (element.current) updateSummary(element.current, { activeCount, report, quotaCount }); }, [activeCount, report, quotaCount]);
  return <div ref={element} />;
}

function OngoingPanel({ sessions }: { sessions: SessionItem[] | null }) {
  const element = useRef<HTMLDivElement>(null);
  useEffect(() => { if (element.current) updateOngoing(element.current, sessions); }, [sessions]);
  return <div ref={element} />;
}

function ActivityPanel({ report }: { report: unknown }) {
  const element = useRef<HTMLDivElement>(null);
  useEffect(() => { if (element.current) updateActivity(element.current, report); }, [report]);
  return <div ref={element} />;
}

function ProvidersPanel({ report }: { report: unknown }) {
  const element = useRef<HTMLDivElement>(null);
  useEffect(() => { if (element.current) updateProviders(element.current, report); }, [report]);
  return <div ref={element} />;
}

function SessionsPanel({ sessions, onRotate }: { sessions: SessionItem[]; onRotate: (session: SessionItem) => Promise<void> }) {
  const element = useRef<HTMLDivElement>(null);
  useEffect(() => { if (element.current) updateSessions(element.current, sessions, { rotate: true }); }, [sessions]);
  return <div ref={element} className="overflow-x-auto" role="region" aria-label="Agent sessions" tabIndex={0} onClick={event => {
    const button = (event.target as Element).closest<HTMLButtonElement>('[data-session-action="rotate"]');
    const session = sessions.find(session => session.id === button?.dataset.sessionId);
    if (session) void onRotate(session);
  }} />;
}

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
  cost_basis?: string;
  cache_share_pct?: number | null;
  total_tokens?: number;
  daily?: unknown[];
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
  const [projectFilter, setProjectFilter] = useState("all");
  const [projectNames, setProjectNames] = useState<string[]>([]);
  const [agentFilter, setAgentFilter] = useState("all");
  const [statusFilter, setStatusFilter] = useState("all");
  const [page, setPage] = useState(1);
  const [copyFeedback, setCopyFeedback] = useState("");

  // Live Metrics
  const [report, setReport] = useState<TimelineReport | null>(null);
  const [ongoing, setOngoing] = useState<SessionItem[] | null>(null);
  const [sessions, setSessions] = useState<SessionItem[]>([]);
  const [quota, setQuota] = useState<QuotaItem[]>([]);
  const [snapshotGeneratedAt, setSnapshotGeneratedAt] = useState<string | null>(null);
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
  const metricsGeneration = useRef(0);
  const manualRefreshInFlight = useRef(false);
  const [activeView, setActiveView] = useState("overview");
  const [viewError, setViewError] = useState("");
  const lastDesktopSnapshot = useRef<any>(null);

  useEffect(() => {
    lastDesktopSnapshot.current = null;
    setProjectNames([]); setProjectFilter('all');
    setOngoing(null); setSessions([]); setQuota([]); setReport(null); setRoiByProvider([]);
    setCombinedRoi(null); setUserProfile(null); setSnapshotGeneratedAt(null);
    setSnapshotLoaded(false); setActiveCount(null); setLastSuccessfulUpdate(null);
    setRefreshMessage(""); setConnectedSource(user ? "Loading account snapshot" : "Sign in to monitor your desktop remotely");
  }, [user?.id]);

  useEffect(() => {
    const restore = () => {
      const view = new URLSearchParams(window.location.search).get("view") || "overview";
      if (["overview", "sessions", "usage", "limits", "projects", "automations", "settings"].includes(view)) setActiveView(view);
    };
    restore();
    window.addEventListener("popstate", restore);
    return () => window.removeEventListener("popstate", restore);
  }, []);

  const changeConsoleView = (view: string) => {
    setActiveView(view);
    window.history.pushState(null, "", view === "overview" ? "/console" : `/console?view=${view}`);
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

  // Live metrics polling
  const fetchLiveMetrics = useCallback(async (force = false) => {
    const generation = metricsGeneration.current;
    if (refreshInFlight.current && !force) return false;
    if (refreshInFlight.current && force) await new Promise<void>((resolve) => {
      const check = () => refreshInFlight.current ? window.setTimeout(check, 50) : resolve();
      check();
    });
    if (refreshInFlight.current || generation !== metricsGeneration.current) return false;
    refreshInFlight.current = true;
    setViewError("");
    let refreshed = false;
    try {
    if (user) {
      try {
        const desktop = await supabase.from("fleethud_sync_state").select("*").eq("user_id", user.id).maybeSingle();
        if (generation !== metricsGeneration.current) return false;
        if (!desktop.error) lastDesktopSnapshot.current = desktop.data?.live_snapshot || null;
        const snap = lastDesktopSnapshot.current;
        if (desktop.error) setViewError("Desktop snapshot unavailable. Last received data remains visible.");
        refreshed = !desktop.error;
        if (refreshed) setLastSuccessfulUpdate(new Date());
        const allSessions: SessionItem[] = [...(snap?.sessions || [])];
        setProjectNames([...new Set(allSessions.map((session: SessionItem) => session.project_name))].sort());
        setOngoing(allSessions.filter((session: SessionItem) => session.is_active));
        const filtered = allSessions.sort((a, b) => Date.parse(b.last_event_at) - Date.parse(a.last_event_at)).filter((session) =>
          (projectFilter === "all" || session.project_name === projectFilter) &&
          (agentFilter === "all" || session.agent === agentFilter) &&
          (statusFilter === "all" || (statusFilter === "active" ? session.is_active : !session.is_active)));
        setSessions(filtered.slice((page-1)*10,page*10));
        setReport(snap?.timelines?.[timeline] || (timeline === "today" ? snap?.timeline : null) || null);
        setQuota(snap?.quota || []);
        setSnapshotGeneratedAt(snap?.generated_at || null);
        setRoiByProvider(snap?.roi_by_provider || []);
        setCombinedRoi(snap?.combined_roi || null);
        setUserProfile(snap?.user_profile || null);
        setSnapshotLoaded(Boolean(snap));
        setActiveCount(snap?.active_count ?? null);
        setTotalCount(filtered.length);
        setTotalPages(Math.max(1, Math.ceil(filtered.length / 10)));
        if (snap) {
          const ageMs = snap.generated_at ? Date.now() - Date.parse(snap.generated_at) : Infinity;
          setConnectedSource(ageMs < 120000 ? "Desktop connected" : "Desktop data stale");
        } else setConnectedSource(desktop.error ? "Desktop unavailable" : "No paired desktop snapshot received");
      } catch {
        if (generation !== metricsGeneration.current) return false;
        setConnectedSource("Offline");
        setViewError("Connection failed. Showing the last data held in this page.");
      }
    } else {
      setConnectedSource("Sign in to monitor your desktop remotely");
      setSnapshotLoaded(false);
      setActiveCount(null);
      setSessions([]);
      setQuota([]);
      setReport(null);
      setRoiByProvider([]);
      setCombinedRoi(null);
      setUserProfile(null);
      setSnapshotGeneratedAt(null);
    }
    } finally {
    refreshInFlight.current = false;
    }
    return refreshed;
  }, [user, timeline, projectFilter, agentFilter, statusFilter, page]);

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
    metricsGeneration.current += 1;
    void fetchLiveMetrics(true);
    const timer = setInterval(() => { if (!document.hidden) void fetchLiveMetrics(); }, 15000);
    return () => { clearInterval(timer); metricsGeneration.current += 1; };
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

  const relayBusy = useRef(false);
  const handleRemoteRelay = async (session: SessionItem) => {
    if (relayBusy.current || !window.confirm("Request a clean session on the paired desktop? The daemon applies your existing permissions.")) return;
    relayBusy.current = true;
    // 2. If remote (away from laptop), communicate via Supabase sync state to local daemon
    if (!user) {
      setRefreshMessage("Sign in to request a session rotation.");
      relayBusy.current = false;
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
      setRefreshMessage("Rotation requested. Awaiting execution by the paired desktop.");
    } catch (err: any) {
      setRefreshMessage("Rotation request failed. Check account connectivity and try again.");
    } finally {
      relayBusy.current = false;
    }
  };

  const activeRoi = roiByProvider.filter((roi) => (roi.spend_usd ?? 0) > 0);

  if (!authChecked) {
    return (
      <div className="flex h-96 items-center justify-center font-mono text-xs text-muted">
        Connecting to spawnt Mission Control...
      </div>
    );
  }

  return (
    <main className="sp-console-shell sp-console-app mx-auto py-10 font-sans">
      {/* Header Controls */}
      <header className="sp-console-heading">
        <div className="sp-console-brand-lockup">
          <span className="sp-console-mark sp-brand-mark" aria-hidden="true" />
          <div>
            <h1 className="sp-console-title">{activeView === "overview" ? "Your agent fleet" : activeView[0].toUpperCase() + activeView.slice(1)}</h1>
            <div className="sp-console-status">
              <span className={`w-2 h-2 ${connectedSource.startsWith("Remote daemon") ? "bg-green animate-pulse" : "bg-red"}`}></span>
              <span>{connectedSource}</span>
            </div>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <button type="button" onClick={refreshCloudSnapshot} disabled={refreshing || !user || activeView === "settings"} title={activeView === "settings" ? "Use Reload preferences in Settings" : "Read the latest received cloud snapshot"} className="sp-console-action" aria-busy={refreshing}>{refreshing ? "Refreshing…" : "Reload view"}</button>
          {user && <button type="button" className="sp-console-action" onClick={copyQuickToken}>Copy desktop pairing token</button>}
          <button type="button" className="sp-console-action" onClick={() => changeConsoleView("settings")}>Settings</button>
        </div>
      </header>

      {!user && <section className="sp-console-connect" aria-label="Desktop connection">
        <p>Sign in to view your paired desktop snapshots. Hosted cloud-session tracking is unavailable.</p>
        <div className="sp-console-connect-actions">
          <Link href="/auth" className="sp-console-link sp-console-link-primary">Sign in</Link>
        </div>
      </section>}

      {copyFeedback && <p role="status" className="sp-console-notice">{copyFeedback}</p>}
      <div className="sp-console-refresh-state" aria-live="polite">
        <span>{refreshMessage || connectedSource}</span>
        <span>{lastSuccessfulUpdate ? `Last successful view refresh: ${lastSuccessfulUpdate.toLocaleTimeString()}` : "Last successful view refresh: not yet"}</span>
      </div>

      <div className="sp-console-workspace">
      <nav className="sp-console-nav" aria-label="Console views">
        {["overview", "sessions", "usage", "limits", "projects", "automations", "settings"].map((view) => <button key={view} type="button" aria-current={activeView === view ? "page" : undefined} onClick={() => changeConsoleView(view)}>{view}</button>)}
      </nav>
      <div className="sp-console-content">
      {activeView === "settings" && <SettingsPanel embedded />}
      {activeView === "projects" && <div dangerouslySetInnerHTML={{ __html: renderProjects(undefined) }} />}
      {activeView === "automations" && <div className="sp-console-notice" role="status">Resume queues and guardrails are configured on the paired local console. Session rotation is available from session details.</div>}

      {/* Timeline Filter Bar */}
      {!["settings", "automations"].includes(activeView) && <div className="sp-console-timeline-filter">
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
          {["today", "month", "7d", "30d", "all"].map((t) => (
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
              {t === "today" ? "Today" : t === "month" ? "This month" : t === "7d" ? "7 Days" : t === "30d" ? "30 Days" : "All Time"}
            </button>
          ))}
        </div>
      </div>

      }
      {(activeView === "overview" || activeView === "usage") && <ActivityPanel report={report} />}
      {activeView === "overview" && <SummaryPanel activeCount={activeCount} report={report} quotaCount={snapshotLoaded ? quota.length : null} />}
      {activeView === "overview" && <div className="sp-overview-live"><details className="sp-overview-limits" open><summary>Provider limits</summary><div dangerouslySetInnerHTML={{ __html: renderQuota(quota, snapshotGeneratedAt) }} /></details><OngoingPanel sessions={ongoing} /></div>}

      {activeView === "usage" && <ProvidersPanel report={report} />}

      {/* Subscription ROI Breakdown */}
      {activeView === "usage" && <div className="bg-card border border-border rounded-lg p-5 mb-6 font-mono">
        <div className="flex justify-between items-center mb-3">
          <div className="text-xs text-muted uppercase font-semibold">Subscription ROI Breakdown</div>
          <button type="button" onClick={() => changeConsoleView("settings")} className="text-xs text-accent hover:underline">
            Edit plans
          </button>
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

      {activeView === "limits" && <div dangerouslySetInnerHTML={{ __html: renderQuota(quota, snapshotGeneratedAt) }} />}
      {viewError && <div className="sp-console-notice" role="alert">{viewError}</div>}
      <p className="text-xs text-muted mb-3">Session rows and charts show metadata received from your paired desktop.</p>
      {/* Live & Historical Agent Sessions */}
      {activeView === "sessions" && <div className="bg-card border border-border rounded-lg p-5 font-mono">
        <div className="flex justify-between items-center mb-4 flex-wrap gap-3">
          <div className="text-xs text-muted uppercase font-semibold">Live & Historical Agent Sessions</div>
          <div className="flex gap-2">
            <select aria-label="Project" value={projectFilter} onChange={event => { setProjectFilter(event.target.value); setPage(1); }}><option value="all">All projects</option>{projectNames.map(project => <option key={project} value={project}>{project}</option>)}</select>
            <select aria-label="Agent"
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
            <select aria-label="Session status"
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

        <SessionsPanel sessions={sessions} onRotate={handleRemoteRelay} />
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
      </div></div>
    </main>
  );
}
