"use client";

import { CONSOLE_ICONS } from '../../lib/console-navigation';
import { useState, useEffect, useCallback, useRef } from "react";
import Link from "next/link";
import RemoteCompanion from "../../components/RemoteCompanion";
import DesktopDevices from "../../components/DesktopDevices";
import ThemeControl from "../../components/ThemeControl";
import SettingsPanel from "../../components/SettingsPanel";
import AppLockGate from "../../components/AppLockGate";
import { supabase } from "../../lib/supabase";
import { publishWidgetSnapshot } from "../../lib/widget-bridge";
import { updateActivity, updateHeatmap, updateProviders, updateSessions, updateQuota, updateHomeSessions, updateTodayTiles, updatePeriodTiles, updateValue, mountProjectList, projectsFromSessions, type ActivityHeatmap } from "../../lib/console-presentation.js";

// The four places of the console. `sessions` is the synced history reached from Home, so it keeps Home highlighted.
const PLACES: [string, string][] = [["overview", "Home"], ["chat", "Workspace"], ["usage", "Usage"], ["settings", "Settings"]];
const PLACE_ICONS: Record<string, string> = { ...CONSOLE_ICONS, chat: CONSOLE_ICONS.sessions };
const VIEW_ALIASES: Record<string, string> = { limits: "usage", projects: "usage", automations: "settings" };
const VIEW_KEYS = ["overview", "sessions", "chat", "usage", "settings"];
const VIEW_TITLES: Record<string, string> = { overview: "Home", sessions: "Sessions", chat: "Workspace", usage: "Usage", settings: "Settings" };
const normalizeView = (view: string | null) => { const key = VIEW_ALIASES[view || ""] || view || ""; return VIEW_KEYS.includes(key) ? key : ""; };

// Presenter panels only hand synced data to the shared formatters; every state, label and group comes from the desktop.
function HomeSessionsPanel({ sessions, onAction }: { sessions: SessionItem[] | null; onAction: (action: string) => void }) {
  const element = useRef<HTMLDivElement>(null);
  useEffect(() => { if (element.current) updateHomeSessions(element.current, sessions); }, [sessions]);
  return <div ref={element} onClick={event => { const target = (event.target as HTMLElement).closest<HTMLElement>("[data-action]"); if (target?.dataset.action) onAction(target.dataset.action); }} />;
}

function TodayPanel({ report, activeCount }: { report: unknown; activeCount: number | null }) {
  const element = useRef<HTMLDivElement>(null);
  useEffect(() => { if (element.current) updateTodayTiles(element.current, { report, activeCount }); }, [report, activeCount]);
  return <div ref={element} />;
}

function PeriodTilesPanel({ report }: { report: unknown }) {
  const element = useRef<HTMLDivElement>(null);
  useEffect(() => { if (element.current) updatePeriodTiles(element.current, report); }, [report]);
  return <div ref={element} />;
}

function ActivityPanel({ report }: { report: unknown }) {
  const element = useRef<HTMLDivElement>(null);
  useEffect(() => { if (element.current) updateActivity(element.current, report, { totals: false }); }, [report]);
  return <div ref={element} />;
}

function HeatmapPanel({ heatmap }: { heatmap: ActivityHeatmap | null }) {
  const element = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!element.current) return;
    updateHeatmap(element.current, heatmap, { weeks: 53, stats: true, legend: true, months: true });
    const scroller = element.current.querySelector<HTMLElement>(".sp-heatmap-scroll");
    if (scroller) scroller.scrollLeft = scroller.scrollWidth;
  }, [heatmap]);
  return <div ref={element} />;
}

function ValuePanel({ byProvider, combined, onManage }: { byProvider: unknown[]; combined: unknown; onManage: () => void }) {
  const element = useRef<HTMLElement>(null);
  useEffect(() => { if (element.current) updateValue(element.current, byProvider, combined); }, [byProvider, combined]);
  return <section ref={element} className="sp-section" onClick={event => { if ((event.target as HTMLElement).closest("[data-manage-plans]")) onManage(); }} />;
}

function QuotaPanel({ pools, publishedAt }: { pools: QuotaItem[]; publishedAt: string | null }) {
  const element = useRef<HTMLDivElement>(null);
  useEffect(() => { if (element.current) updateQuota(element.current, pools, publishedAt); }, [pools,publishedAt]);
  return <div ref={element} />;
}

function ProvidersPanel({ report }: { report: unknown }) {
  const element = useRef<HTMLDivElement>(null);
  useEffect(() => { if (element.current) updateProviders(element.current, report); }, [report]);
  return <div ref={element} />;
}

function ProjectsPanel({ sessions, onOpen }: { sessions: SessionItem[] | null; onOpen: (project: string) => void }) {
  const element = useRef<HTMLDivElement>(null);
  useEffect(() => { if (element.current) mountProjectList(element.current, projectsFromSessions(sessions), { heading: true, helper: "Most recent first. Open a project to see its synced sessions." }); }, [sessions]);
  return <div ref={element} onClick={event => { const row = (event.target as HTMLElement).closest<HTMLElement>("[data-project-id]"); if (row?.dataset.projectId) onOpen(row.dataset.projectId); }} />;
}

function SessionsPanel({ sessions }: { sessions: SessionItem[] }) {
  const element = useRef<HTMLDivElement>(null);
  useEffect(() => { if (element.current) updateSessions(element.current, sessions, { rotate: false }); }, [sessions]);
  return <div ref={element} className="overflow-x-auto" role="region" aria-label="Agent sessions" tabIndex={0} />;
}

interface SessionItem {
  id: string;
  agent: string;
  surface: string;
  project_name: string;
  model: string;
  state: string;
  is_active: boolean;
  is_ongoing?: boolean;
  home_group?: "needs_you" | "running" | "recent" | null;
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

export default function ConsolePage({ initialView = "overview" }: { initialView?: string }) {
  const [user, setUser] = useState<any>(null);
  const [authChecked, setAuthChecked] = useState(false);

  // Filters & Pagination
  const [desktopId,setDesktopId]=useState('');
  const [desktopChoices,setDesktopChoices]=useState<{id:string;name:string}[]>([]);
  const [timeline, setTimeline] = useState("all");
  const [projectFilter, setProjectFilter] = useState("all");
  const [projectNames, setProjectNames] = useState<string[]>([]);
  const [agentFilter, setAgentFilter] = useState("all");
  const [statusFilter, setStatusFilter] = useState("all");
  const [page, setPage] = useState(1);
  const [copyFeedback, setCopyFeedback] = useState("");

  // Live Metrics
  const [report, setReport] = useState<TimelineReport | null>(null);
  const [homeSessions, setHomeSessions] = useState<SessionItem[] | null>(null);
  const [todayReport, setTodayReport] = useState<TimelineReport | null>(null);
  const [activityHeatmap, setActivityHeatmap] = useState<ActivityHeatmap | null>(null);
  const [sessions, setSessions] = useState<SessionItem[]>([]);
  const [quota, setQuota] = useState<QuotaItem[]>([]);
  const [snapshotGeneratedAt, setSnapshotGeneratedAt] = useState<string | null>(null);
  const [roiByProvider, setRoiByProvider] = useState<PlanRoiInfo[]>([]);
  const [combinedRoi, setCombinedRoi] = useState<PlanRoiInfo | null>(null);
  const [activeCount, setActiveCount] = useState<number | null>(null);
  const [snapshotLoaded, setSnapshotLoaded] = useState(false);
  const [totalCount, setTotalCount] = useState<number>(0);
  const [snapshotTotal, setSnapshotTotal] = useState<number | null>(null);
  const [totalPages, setTotalPages] = useState<number>(1);
  const [connectedSource, setConnectedSource] = useState<string>("Scanning...");
  const [refreshing, setRefreshing] = useState(false);
  const [lastSuccessfulUpdate, setLastSuccessfulUpdate] = useState<Date | null>(null);
  const [refreshMessage, setRefreshMessage] = useState("");
  const refreshInFlight = useRef(false);
  const metricsGeneration = useRef(0);
  const manualRefreshInFlight = useRef(false);
  const [activeView, setActiveView] = useState(initialView);
  const [navigationExpanded, setNavigationExpanded] = useState(false);
  const [viewError, setViewError] = useState("");
  const lastDesktopSnapshot = useRef<any>(null);

  useEffect(() => {
    lastDesktopSnapshot.current = null;
    setProjectNames([]); setProjectFilter('all');
    setHomeSessions(null); setTodayReport(null); setSessions([]); setQuota([]); setReport(null); setRoiByProvider([]);
    setCombinedRoi(null); setSnapshotGeneratedAt(null);
    setSnapshotLoaded(false); setActiveCount(null); setLastSuccessfulUpdate(null);
    setRefreshMessage(""); setConnectedSource(user?.id ? "Loading account snapshot" : "Sign in to monitor your desktop remotely");
    setSnapshotTotal(null);
  }, [user?.id]);

  useEffect(() => {
    const restore = () => {
      const view = normalizeView(new URLSearchParams(window.location.search).get("view")) || (window.location.pathname === '/remote' ? 'chat' : 'overview');
      setActiveView(view);
    };
    restore();
    window.addEventListener("popstate", restore);
    return () => window.removeEventListener("popstate", restore);
  }, []);

  const changeConsoleView = (requested: string) => {
    const view = normalizeView(requested) || "overview";
    setActiveView(view);
    window.history.pushState(null, "", view === "overview" ? "/console" : `/console?view=${view}`);
    window.scrollTo?.({ top: 0 });
  };
  const homeAction = (action: string) => {
    if (action === "view:sessions") changeConsoleView("sessions");
    else changeConsoleView("chat");
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
        const directory=await supabase.from("spawnt_desktops").select("id,name,last_seen_at,live_snapshot").eq("user_id",user.id).order("last_seen_at",{ascending:false});
        const choices=directory.data || [];
        setDesktopChoices(choices.map(row=>({id:row.id,name:row.name})));
        const chosen=choices.find(row=>row.id===desktopId) || choices[0];
        const desktop={data:chosen,error:directory.error};
        if (generation !== metricsGeneration.current) return false;
        if (!desktop.error) lastDesktopSnapshot.current = desktop.data?.live_snapshot || null;
        const snap = lastDesktopSnapshot.current;
        if (desktop.error) setViewError("Desktop snapshot unavailable. Last received data remains visible.");
        else if (chosen && !lastDesktopSnapshot.current)
          // The desktop is registered but has published nothing, which is what an empty page
          // here always means: say so instead of showing a page of "Unknown".
          setViewError(
            `${chosen.name || "This desktop"} has not published a snapshot yet. Open Gnomcode on that computer and turn on "Share this desktop's snapshot with your account" in Settings.`,
          );
        refreshed = !desktop.error;
        if (refreshed) setLastSuccessfulUpdate(new Date());
        const allSessions: SessionItem[] = [...(snap?.sessions || [])];
        setProjectNames([...new Set(allSessions.map((session: SessionItem) => session.project_name))].sort());
        allSessions.sort((a, b) => Date.parse(b.last_event_at) - Date.parse(a.last_event_at));
        setHomeSessions(snap ? allSessions : null);
        setTodayReport(snap?.timelines?.today || snap?.timeline || null);
        setActivityHeatmap(snap?.activity_heatmap || null);
        const filtered = allSessions.filter((session) =>
          (projectFilter === "all" || session.project_name === projectFilter) &&
          (agentFilter === "all" || session.agent === agentFilter) &&
          (statusFilter === "all" || (statusFilter === "ongoing" ? (session.is_ongoing ?? session.is_active) : statusFilter === "active" ? session.is_active : statusFilter === "ended" ? session.state === "ended" : session.state === "idle")));
        setSessions(filtered.slice((page-1)*10,page*10));
        setReport(snap?.timelines?.[timeline] || (timeline === "today" ? snap?.timeline : null) || null);
        setQuota(snap?.quota || []);
        setSnapshotGeneratedAt(snap?.generated_at || null);
        setRoiByProvider(snap?.roi_by_timeline?.[timeline]?.roi_by_provider || []);
        setCombinedRoi(snap?.roi_by_timeline?.[timeline]?.combined_roi || null);
        setSnapshotLoaded(Boolean(snap));
        if (snap) publishWidgetSnapshot(snap.generated_at, snap.activity_heatmap, snap.quota || [], allSessions);
        // Sessions carry the desktop's own home_group; desktops that predate it fall back to their reported count.
        const grouped = allSessions.some((session: SessionItem) => session.home_group);
        setActiveCount(grouped ? allSessions.filter((session: SessionItem) => session.home_group === "needs_you" || session.home_group === "running").length : snap?.ongoing_count ?? snap?.active_count ?? null);
        setSnapshotTotal(typeof snap?.total_count === "number" ? snap.total_count : null);
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
      setSnapshotTotal(null);
      setActiveCount(null);
      setSessions([]);
      setHomeSessions(null);
      setTodayReport(null);
      setQuota([]);
      setReport(null);
      setRoiByProvider([]);
      setCombinedRoi(null);
      setSnapshotGeneratedAt(null);
    }
    } finally {
    refreshInFlight.current = false;
    }
    return refreshed;
  }, [user, desktopId, timeline, projectFilter, agentFilter, statusFilter, page]);

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
        const {access_token,refresh_token,expires_at,expires_in,user}=data.session;
        const tokenStr=btoa(JSON.stringify({access_token,refresh_token,expires_at,expires_in,user:{id:user.id,email:user.email}}));
        const ok = await (await import("../../lib/console-presentation")).copyTextToClipboard(tokenStr);
        setCopyFeedback(ok ? "Copied account connection" : "Failed to copy");
        if (ok) setTimeout(() => setCopyFeedback(""), 3500);
      }
    } catch {
      setCopyFeedback("Failed to copy");
    }
  };


  if (!authChecked) {
    return (
      <div className="flex h-96 items-center justify-center font-mono text-xs text-muted">
        Connecting to gnomcode Mission Control...
      </div>
    );
  }

  return (
    <AppLockGate>
    <div className="sp-console-app gc-web">
      <div className="sp-console-workspace gc-console" data-navigation={navigationExpanded ? "expanded" : "collapsed"} data-view={activeView}>
      <nav className="sp-console-nav" aria-label="Console views">
        <Link href="/console" className="app-brand" aria-label="Gnomcode dashboard"><span className="sp-brand-mark" /></Link>
        <div className="sp-rail-tabs">{PLACES.map(([view, label]) => <button key={view} type="button" className={`tab-btn${activeView === view ? " active" : ""}`} data-place={view} aria-label={label} title={label} aria-current={(activeView === "sessions" ? "overview" : activeView) === view ? "page" : undefined} onClick={() => changeConsoleView(view)}><svg viewBox="0 0 24 24" aria-hidden="true"><path d={PLACE_ICONS[view]} /></svg><span className="rail-label">{label}</span></button>)}</div><div className="sp-nav-footer">
        <Link className="web-download-link tab-btn" href="/downloads" aria-label="Download desktop app" title="Download app"><svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true"><path d="M12 3v12m-5-5 5 5 5-5M4 17v4h16v-4"/></svg><span className="rail-label">Download app</span></Link>
        <button type="button" className="web-rail-toggle tab-btn" aria-label={navigationExpanded ? "Collapse navigation" : "Expand navigation"} title={navigationExpanded ? "Collapse navigation" : "Expand navigation"} aria-expanded={navigationExpanded} onClick={() => setNavigationExpanded(!navigationExpanded)}><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M3 4h18v16H3z M9 4v16" /></svg><span className="rail-label">{navigationExpanded ? "Collapse navigation" : "Expand navigation"}</span></button>
      </div></nav>
      <div className="sp-console-content">
      <header className="console-context"><h1 aria-live="polite">{VIEW_TITLES[activeView] || "Home"}</h1>{activeView === "sessions" && <button type="button" className="sp-link" onClick={() => changeConsoleView("overview")}>Back to Home</button>}</header>
      {!user && <section className="sp-console-connect" aria-label="Desktop connection">
        <p>Sign in to view activity from your paired desktop.</p>
        <div className="sp-console-connect-actions">
          <Link href="/auth" className="sp-console-link sp-console-link-primary">Sign in</Link>
        </div>
      </section>}

      {copyFeedback && <p role="status" className="sp-console-notice">{copyFeedback}</p>}
      <details className="sp-data-details" hidden={activeView !== "settings"}><summary>Connection details</summary><div className="sp-console-refresh-state" aria-live="polite">
        <span>{refreshMessage || connectedSource}</span>
        <span>{lastSuccessfulUpdate ? `Last successful view refresh: ${lastSuccessfulUpdate.toLocaleTimeString()}` : "Last successful view refresh: not yet"}</span>
      </div></details>

      <main className="gc-web-page">
      {activeView === "settings" && <><section className="web-profile-section"><h2>Appearance</h2><ThemeControl /></section><section className="web-profile-section"><h2>Desktop connection and data</h2><p>Account sync shares session metadata without end-to-end encryption. Linked desktops are available to every device signed into your account.</p><details><summary>What gets shared</summary><p>Connecting publishes project names, session IDs, agent activity, usage and quotas to your account. The snapshot is not end-to-end encrypted and may contain sensitive metadata. New snapshots exclude conversation text and titles; older copies may still contain titles. Sign in on your desktop to make it available here. Linking the native app allows every device signed in to this account to open its available sessions. No phone pairing or per-desktop approval is required.</p></details><div className="web-profile-actions"><button type="button" className="sp-console-action" onClick={refreshCloudSnapshot} disabled={refreshing || !user}>{refreshing ? "Refreshing…" : "Reload activity"}</button></div>{user && <details><summary>Connect an existing browser account</summary><p>Use this option in the native app when you signed in through a provider. Treat the copied connection like a password.</p><button type="button" className="sp-console-action" onClick={copyQuickToken}>Copy account connection</button></details>}</section><section className="web-profile-section"><h2>Automations</h2><p>Schedule messages, reconnect after a limit, or choose automatic model switching from Workspace. Manage usage rules in the desktop app.</p><button type="button" className="sp-console-action" onClick={()=>changeConsoleView('chat')}>Open workspace</button></section><DesktopDevices /><SettingsPanel embedded /></>}
      <div className="sp-web-remote-host" hidden={activeView !== "chat"}><RemoteCompanion embedded visible={activeView === "chat"} /></div>
      {user && desktopChoices.length>0 && (activeView==="overview"||activeView==="usage") && <label className="sp-console-timeline-filter sp-desktop-picker"><span>Desktop</span><select value={desktopId || desktopChoices[0].id} onChange={event=>{lastDesktopSnapshot.current=null;setDesktopId(event.target.value);setPage(1);}}>{desktopChoices.map(desktop=><option key={desktop.id} value={desktop.id}>{desktop.name}</option>)}</select></label>}
      {viewError && <div className="sp-console-notice" role="alert">{viewError}</div>}
      {activeView === "overview" && <div className="sp-home">
        {snapshotGeneratedAt && Date.now() - Date.parse(snapshotGeneratedAt) > 10 * 60_000 && <p className="sp-snapshot-note" role="note">
          Home and limits show the last snapshot your desktop published, {new Date(snapshotGeneratedAt).toLocaleString()}. Desktops publish only while Account sync is on (desktop Settings, Privacy and data). Conversations in Workspace are live over Remote.
        </p>}
        <div className="sp-home-grid">
          <HomeSessionsPanel sessions={homeSessions} onAction={homeAction} />
          <QuotaPanel pools={quota} publishedAt={snapshotGeneratedAt} />
        </div>
        <HeatmapPanel heatmap={activityHeatmap} />
        <section className="sp-section"><TodayPanel report={todayReport} activeCount={activeCount} /></section>
      </div>}
      {activeView === "usage" && <div className="sp-web-usage-overview"><QuotaPanel pools={quota} publishedAt={snapshotGeneratedAt} /><HeatmapPanel heatmap={activityHeatmap} /></div>}

      {activeView === "usage" && <div className="sp-console-timeline-filter">
        <div className="text-xs text-muted">
          <span className="text-foreground">{report?.range_label || "Activity"}</span>
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
          <select aria-label="Reporting period" value={timeline} onChange={event => { setTimeline(event.target.value); setPage(1); }}>{[["today","Today"],["month","This month"],["7d","Last 7 days"],["30d","Last 30 days"],["all","All time"]].map(([value,label])=><option key={value} value={value}>{label}</option>)}</select>
        </div>
      </div>

      }
      {activeView === "usage" && <PeriodTilesPanel report={report} />}

      {activeView === "usage" && <ProvidersPanel report={report} />}

      {activeView === "usage" && <ValuePanel byProvider={roiByProvider} combined={combinedRoi} onManage={() => changeConsoleView("settings")} />}

      {activeView === "usage" && <ActivityPanel report={report} />}
      {activeView === "usage" && <section className="sp-usage-projects" aria-label="Project activity">
        {projectsFromSessions(homeSessions).length > 0 ? <ProjectsPanel sessions={homeSessions} onOpen={project => { setProjectFilter(project); setPage(1); changeConsoleView('sessions'); }} /> : <><header className="sp-section-head"><h2>Project activity</h2></header><div className="sp-empty"><p>{!user ? 'Sign in and connect your desktop to see projects from your session history.' : snapshotLoaded ? 'No projects are present in the latest desktop snapshot. Start an agent in a project on your desktop, then reload activity.' : 'No desktop activity has been received yet. Connect your desktop to publish your project and session metadata.'}</p><div><button type="button" className="sp-console-action" onClick={() => changeConsoleView('settings')}>Connect your desktop</button>{user && <button type="button" className="sp-console-action" disabled={refreshing} onClick={refreshCloudSnapshot}>{refreshing ? 'Reloading...' : 'Reload activity'}</button>}</div></div></>}
      </section>}
      

      {activeView === "sessions" && snapshotTotal !== null && snapshotTotal >= 200 && <p role="status">The account view contains up to 200 recent or active sessions. The desktop reports {snapshotTotal} recorded sessions; older desktop versions may undercount. Filters and counts below cover only the published sessions. Open the local console for full history.</p>}
      {/* Session history */}
      {activeView === "sessions" && <div className="bg-card border border-border rounded-lg p-5 font-mono">
        <div className="flex justify-between items-center mb-4 flex-wrap gap-3">
          <div className="text-xs text-muted uppercase font-semibold">Session history <Link href="/remote" className="ml-3 underline normal-case">Open encrypted Remote</Link></div>
          <div className="sp-session-filters">
            <select aria-label="Project" value={projectFilter} onChange={event => { setProjectFilter(event.target.value); setPage(1); }}><option value="all">All projects</option>{projectNames.map(project => <option key={project} value={project}>{project}</option>)}</select>
            <select aria-label="Agent"
              value={agentFilter}
              onChange={(e) => { setAgentFilter(e.target.value); setPage(1); }}
              className="bg-bg border border-border text-xs rounded px-2.5 py-1 text-foreground focus-visible:outline-2 focus-visible:outline-[var(--sp-accent)] focus-visible:outline-offset-2"
            >
              <option value="all">All Agents</option>
              <option value="claude-code">Claude Code</option>
              <option value="codex">Codex</option>
              <option value="antigravity">Antigravity</option>
            </select>
            <select aria-label="Session status"
              value={statusFilter}
              onChange={(e) => { setStatusFilter(e.target.value); setPage(1); }}
              className="bg-bg border border-border text-xs rounded px-2.5 py-1 text-foreground focus-visible:outline-2 focus-visible:outline-[var(--sp-accent)] focus-visible:outline-offset-2"
            >
              <option value="all">All Statuses</option>
              <option value="ongoing">Ongoing</option><option value="active">Running or waiting</option>
              <option value="idle">Idle</option><option value="ended">Ended</option>
            </select>
          </div>
        </div>

        <SessionsPanel sessions={sessions} />
        {/* Pagination Controls */}
        <div className="sp-session-pagination">
          <div>
            Page {page} of {totalPages} · {totalCount} sessions
          </div>
          <div className="flex gap-2" hidden={totalPages <= 1}>
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
      </main></div></div>
    </div>
    </AppLockGate>
  );
}
