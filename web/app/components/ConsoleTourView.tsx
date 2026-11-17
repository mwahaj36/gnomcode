"use client";

import { memo, useEffect, useRef, useState } from "react";
import {
  type ActivityHeatmap,
  type ProviderOverview,
  renderHeatmap,
  renderHomeSessions,
  renderPeriodTiles,
  renderProviderCards,
  renderTodayTiles,
} from "../../lib/console-presentation.js";

// Illustrative data rendered by the real console presenters, so the tour uses the console's own markup and CSS.
// Nothing here is measured. Dates are fixed so server and client render the same grid.
const END = Date.UTC(2026, 10, 13);
const day = (offset: number) => new Date(END - offset * 86400000).toISOString().slice(0, 10);
const recent = (minutes: number) => new Date(Date.now() - minutes * 60000).toISOString();

const heatDays = Array.from({ length: 371 }, (_, index) => {
  const offset = 370 - index;
  const weekday = (new Date(day(offset) + "T00:00:00Z").getUTCDay() + 6) % 7;
  const wave = (index * 37 + offset * 11) % 17;
  const level = offset > 364 ? 0 : weekday > 4 ? (wave > 13 ? 1 : 0) : wave < 3 ? 0 : wave < 8 ? 1 : wave < 12 ? 2 : wave < 15 ? 3 : 4;
  return { date: day(offset), level, total_tokens: level * 42000, turns: level * 6, cost_usd: level * 0.6, cost_basis: "estimated" as const };
});
const sampleHeatmap: ActivityHeatmap = {
  from: day(370),
  to: day(0),
  first_recorded: day(364),
  days: heatDays,
  stats: { total_tokens: 18_400_000, active_days: 214, current_streak: 9, longest_streak: 31, busiest: { date: day(23), total_tokens: 412_000 } },
};

const sampleProviders: ProviderOverview[] = [
  {
    provider: "claude-code", display_name: "Claude Code", plan: "Max", state: "ok", headline_window: 0, source_label: "Status line",
    windows: [
      { window: "five_hour", remaining_pct: 64, basis: "exact", state: "ok", reset_label: "in 2h 14m" },
      { window: "weekly", remaining_pct: 41, basis: "exact", state: "ok", reset_label: "Thu 9:00" },
    ],
    cost: { today_usd: 4.12, last_30_days_usd: 61.4, basis: "estimated" },
  },
  {
    provider: "codex", display_name: "Codex", plan: "Plus", state: "low", headline_window: 0, source_label: "Local logs",
    windows: [
      { window: "five_hour", remaining_pct: 12, basis: "exact", state: "low", reset_label: "in 48m" },
      { window: "weekly", remaining_pct: 58, basis: "exact", state: "ok", reset_label: "Mon 7:30" },
    ],
    cost: { today_usd: 1.86, last_30_days_usd: 23.9, basis: "estimated" },
  },
  {
    provider: "antigravity", display_name: "Antigravity", state: "low", headline_window: 0, source_label: "Native limits",
    windows: [
      { window: "five_hour", label: "Gemini Models", pool: "Gemini Models", remaining_pct: 13, basis: "api", state: "low", reset_label: "in 3h 05m" },
      { window: "five_hour", label: "Claude Models", pool: "Claude Models", remaining_pct: 0, basis: "api", state: "exhausted", reset_label: "in 1h 20m" },
    ],
  },
];

const sampleSessions = () => [
  { id: "s1", title: "Fix flaky checkout test", project_name: "storefront", agent: "claude-code", model: "Opus", home_group: "needs_you", state: "awaiting_approval", total_tokens: 182000, context_fill_pct: 46, last_event_at: recent(2) },
  {
    id: "s2", title: "Migrate auth to sessions", project_name: "api", agent: "codex", model: "GPT-5", home_group: "running", state: "thinking", total_tokens: 96400, context_fill_pct: 31, last_event_at: recent(1),
    subagent_count: 3, subagents_ongoing: 2,
    subagents: [
      { id: "s2a", title: "Update token store", state: "thinking", is_ongoing: true },
      { id: "s2b", title: "Write migration tests", state: "thinking", is_ongoing: true },
      { id: "s2c", title: "Audit call sites", state: "idle" },
    ],
  },
  { id: "s3", title: "Build pipeline cleanup", project_name: "infra", agent: "antigravity", model: "Gemini Pro", home_group: "running", state: "thinking", total_tokens: 54200, context_fill_pct: 18, last_event_at: recent(4), repo_path: "\\\\wsl.localhost\\Ubuntu\\home\\dev\\infra" },
  { id: "s4", title: "Release notes draft", project_name: "docs", agent: "claude-code", model: "Sonnet", home_group: "recent", state: "idle", total_tokens: 21800, context_fill_pct: 9, last_event_at: recent(180) },
];

const today = { total_tokens: 412000, total_cost_usd: 5.98, cost_basis: "estimated", unpriced_turns: 0 };
const week = { total_tokens: 2_960_000, total_cost_usd: 37.8, total_cache_savings_usd: 21.4, cost_basis: "estimated", unpriced_turns: 0, range_label: "Last 7 days" };
const makeStatic = (html: string) =>
  html
    .replaceAll("<details ", "<div ")
    .replaceAll("<details>", "<div>")
    .replaceAll("</details>", "</div>")
    .replaceAll("<summary ", "<div ")
    .replaceAll("<summary>", "<div>")
    .replaceAll("</summary>", "</div>");
const openAll = (html: string) =>
  makeStatic(html.replaceAll("<details ", "<details open "));

function Html({ html, className }: { html: string; className?: string }) {
  return <div className={className} suppressHydrationWarning dangerouslySetInnerHTML={{ __html: html }} />;
}

/** The console Home: full-width Limits, then Activity (with the year highlights) and Sessions (2fr) beside Today (1fr). */
export function TourHome({ compact = false, sessions = true }: { compact?: boolean; sessions?: boolean }) {
  return <div className="sp-home-react sp-tour-home">
    <Html html={makeStatic(renderProviderCards(sampleProviders, { cost: false }))} />
    <div className="sp-home-grid">
      {!compact && <Html className="sp-tour-activity" html={renderHeatmap(sampleHeatmap, { weeks: 53, stats: true, legend: true, months: true })} />}
      <Html className="sp-tour-today" html={renderTodayTiles({ report: today, activeCount: 2 })} />
      {sessions && <Html className="sp-tour-sessions" html={renderHomeSessions(sampleSessions())} />}
    </div>
  </div>;
}

const TourContent = memo(function TourContent({ kind }: { kind: string }) {
  const root = useRef<HTMLDivElement>(null);
  // Like the console, the activity grid opens on the most recent weeks.
  useEffect(() => { root.current?.querySelectorAll<HTMLElement>(".sp-heatmap-scroll").forEach(grid => { grid.scrollLeft = grid.scrollWidth; }); }, [kind]);
  // On wide screens the tabs and the whole panel fit the window height: scale the view down (never up)
  // so no tab needs a vertical scroll.
  useEffect(() => {
    const element = root.current;
    const panel = element?.closest<HTMLElement>(".sp-tour-panel");
    const tabs = panel?.parentElement?.querySelector<HTMLElement>(".sp-tour-tabs");
    if (!element || !panel) return;
    const fit = () => {
      if (window.innerWidth < 900) { element.style.zoom = ""; return; }
      const current = Number(element.style.zoom) || 1;
      const view = element.getBoundingClientRect().height / current;
      const chrome = panel.getBoundingClientRect().height - element.getBoundingClientRect().height + (tabs?.offsetHeight ?? 0);
      const available = window.innerHeight - 96 - chrome;
      const scale = Math.max(0.55, Math.min(1, available / view));
      const next = String(Math.floor(scale * 100) / 100);
      if (element.style.zoom !== next) element.style.zoom = next;
    };
    fit();
    const observer = new ResizeObserver(() => fit());
    observer.observe(panel);
    window.addEventListener("resize", fit);
    return () => { observer.disconnect(); window.removeEventListener("resize", fit); };
  }, [kind]);
  return <div ref={root}><TourView kind={kind} /></div>;
});

function TourView({ kind }: { kind: string }) {
  if (kind === "overview") return <TourHome sessions={false} />;
  if (kind === "sessions") return <Html html={openAll(renderHomeSessions(sampleSessions()))} />;
  if (kind === "usage") return <div className="sp-home-react">
    <Html html={renderPeriodTiles(week)} />
    <Html html={renderHeatmap(sampleHeatmap)} />
  </div>;
  return <Html html={openAll(renderProviderCards(sampleProviders))} />;
}


export function ConsoleTourView({kind}:{kind:string}) {
 const frame=useRef<HTMLDivElement>(null),stage=useRef<HTMLDivElement>(null);
 const [scale,setScale]=useState(1);
 useEffect(()=>{
  const fit=()=>{if(frame.current&&stage.current)setScale(Math.min(1,frame.current.clientHeight/Math.max(1,stage.current.scrollHeight)));};
  const observer=new ResizeObserver(fit);
  if(frame.current)observer.observe(frame.current);if(stage.current)observer.observe(stage.current);
  fit();return()=>observer.disconnect();
 },[kind]);
 return <div ref={frame} className="sp-tour-fit"><div ref={stage} className="sp-tour-stage" style={{transform:`translateX(-50%) scale(${scale})`}}><TourContent kind={kind} /></div></div>;
}
