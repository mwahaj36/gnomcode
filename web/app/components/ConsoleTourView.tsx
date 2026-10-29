"use client";

import { useEffect, useRef, type CSSProperties } from "react";
import { renderQuota, renderSessions, updateActivity } from "../../lib/console-presentation.js";

// Illustrative measurements, precomputed for the tour; no live provider data or pricing runs here.
const sessions = [
  { id: "sample-codex", project_name: "web", agent: "Codex", model: "GPT-6.1", surface: "desktop_app", state: "thinking", total_tokens: 18400, input_tokens: 16000, output_tokens: 2400, cache_read_tokens: 9000, context_tokens: 12000, context_window: 258400, context_fill_pct: 4.6, total_cost_usd: .42, cost_basis: "estimated", usage_available: true },
  { id: "sample-claude", project_name: "api", agent: "Claude Code", model: "Sonnet", surface: "cli", state: "awaiting_approval", total_tokens: 12800, input_tokens: 2000, output_tokens: 2800, cache_read_tokens: 8000, context_tokens: 10000, context_window: 200000, context_fill_pct: 5, total_cost_usd: .31, cost_basis: "estimated", usage_available: true },
  { id: "sample-gemini", project_name: "docs", agent: "Gemini CLI", model: "Flash", surface: "cli", state: "idle", total_tokens: 8100, input_tokens: 3100, output_tokens: 1000, cache_read_tokens: 4000, context_tokens: 7100, context_window: 1000000, context_fill_pct: .7, total_cost_usd: .13, cost_basis: "estimated", usage_available: true },
].map(session => ({ ...session, started_at: "2026-10-26T09:00:00Z", last_event_at: "2026-10-26T12:00:00Z" }));
const report = {
  total_tokens: 296000,
  daily: [
    { date: "2026-10-20", total_tokens: 36000, chart_pct: 55.8, cost_usd: .34, turns: 6 },
    { date: "2026-10-21", total_tokens: 24000, chart_pct: 37.2, cost_usd: .29, turns: 4 },
    { date: "2026-10-22", total_tokens: 49500, chart_pct: 76.7, cost_usd: .55, turns: 8 },
    { date: "2026-10-23", total_tokens: 33000, chart_pct: 51.2, cost_usd: .43, turns: 5 },
    { date: "2026-10-24", total_tokens: 64500, chart_pct: 100, cost_usd: .68, turns: 11 },
    { date: "2026-10-25", total_tokens: 49700, chart_pct: 77.1, cost_usd: .63, turns: 8 },
    { date: "2026-10-26", total_tokens: 39300, chart_pct: 60.9, cost_usd: .86, turns: 7 },
  ].map(day => ({ ...day, cost_basis: "estimated" })),
};
const pools = [
  { provider: "Codex", limit_id: "standard", window: "five_hour", used_pct: 38, remaining_pct: 62, basis: "api", reset_label: "in 2h 14m" },
  { provider: "Claude", limit_id: "weekly", window: "weekly", used_pct: 72, remaining_pct: 28, basis: "estimated", reset_label: "in 3d 6h" },
  { provider: "Gemini", limit_id: "daily", window: "daily", used_pct: 56, remaining_pct: 44, basis: "api", reset_label: "in 8h 20m" },
];

export function ConsoleTourView({ kind, visible = true, compact = false }: { kind: string; visible?: boolean; compact?: boolean }) {
  const activity = useRef<HTMLDivElement>(null);
  useEffect(() => { if (visible && activity.current) updateActivity(activity.current, report); }, [kind, visible]);
  return <div className={`sp-tour-real${compact ? " is-compact" : ""}`}>
    {kind === "overview" && <div className="sp-tour-kpis"><div><small>ACTIVE SESSIONS</small><div className="sp-tour-kpi-line"><strong>2</strong><em>EXACT</em></div></div><div><small>TOKENS / SAMPLE DAY</small><div className="sp-tour-kpi-line"><strong>39.3k</strong><em>EXACT</em></div></div><div><small>COST / SAMPLE DAY</small><div className="sp-tour-kpi-line"><strong>$0.86</strong><em>EST</em></div></div></div>}
    {kind === "sessions" && <div className="sp-tour-summary"><span>3 RUNS · EXACT</span><span>1 WAITING · EXACT</span><span>3 AGENTS · EXACT</span></div>}
    {kind === "usage" && <div className="sp-tour-kpis sp-tour-kpis-usage"><div><small>TOKENS / SAMPLE WEEK</small><div className="sp-tour-kpi-line"><strong>296.0k</strong><em>EXACT</em></div></div><div><small>API-EQUIVALENT COST</small><div className="sp-tour-kpi-line"><strong>$3.78</strong><em>EST</em></div></div></div>}
    {kind === "overview" && <>
      {!compact && <figure className="sp-tour-overview-chart">
        <figcaption>Daily tokens · illustrative week · EXACT</figcaption>
        <ol aria-label="Seven recorded sample days">{report.daily.map((day, index) => <li key={day.date} data-sample-day={day.date} aria-label={`${day.date}: ${day.total_tokens.toLocaleString()} tokens, exact`} style={{ "--day-height": `${day.chart_pct}%`, "--day-delay": `${index * 35}ms` } as CSSProperties}><i aria-hidden="true" /></li>)}</ol>
      </figure>}
      <div className="sp-tour-overview-columns"><span>Agent</span><span>State / EST</span><span>Tokens / EXACT</span></div>
      <ul className="sp-tour-overview-list">{sessions.map(session => <li className="sp-tour-overview-row" key={session.id}><strong>{session.agent}</strong><span>{session.state === "thinking" ? "Running" : session.state === "awaiting_approval" ? "Waiting" : "Idle"}</span><span>{new Intl.NumberFormat("en", { notation: "compact", maximumFractionDigits: 1 }).format(session.total_tokens)}</span></li>)}</ul>
    </>}
    {!compact && kind === "usage" && <div ref={activity} />}
    {kind === "sessions" && <div className="sp-tour-real-table" role="region" aria-label="Illustrative sessions; expand for details" tabIndex={0} dangerouslySetInnerHTML={{ __html: renderSessions(sessions, { illustrative: true }) }} />}
    {kind === "usage" && <div className="sp-tour-provider-breakdown">{[["Codex", "124.0k", "$1.84"], ["Claude", "98.0k", "$1.42"], ["Gemini", "74.0k", "$0.52"]].map(([provider, tokens, cost]) => <div key={provider}><strong>{provider}</strong><span>{tokens} tokens · EXACT</span><span>{cost} · EST</span></div>)}</div>}
    {kind === "limits" && <div dangerouslySetInnerHTML={{ __html: renderQuota(pools, null, { illustrative: true }) }} />}
  </div>;
}
