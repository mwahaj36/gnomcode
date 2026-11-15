'use client';

import { useCallback, useEffect, useRef, useState } from 'react';
import {
  mountProjectList,
  updateActivity,
  updateProviderCards,
  updateProviders,
  updateValue,
  type ProjectActivity,
  type ProviderOverview,
} from '../lib/console-presentation.js';
import {
  renderHandoffEvalMarkup,
  type HandoffEvalReport,
} from '../lib/eval-presentation.mjs';
import { ActivityGrid } from './ActivityGrid';
import {
  fetchLocalJson,
  localAuthHeaders,
  type LocalReportResponse,
} from './local-api';

const POLL_INTERVAL_MS = 15_000;

const PERIOD_LABELS: Record<string, string> = {
  today: 'Today',
  month: 'This month',
  '7d': 'Last 7 days',
  '30d': 'Last 30 days',
  all: 'All time',
};

function buildRangeUrl(basePath: string, range: string, extra?: string): string {
  let url = `${basePath}?range=${encodeURIComponent(range)}`;
  if (extra) url += `&${extra}`;
  if (range === 'today' || range === 'month') {
    const start = new Date();
    start.setHours(0, 0, 0, 0);
    if (range === 'month') start.setDate(1);
    url += `&from=${encodeURIComponent(start.toISOString())}`;
  }
  return url;
}

function isUsageVisible(): boolean {
  if (typeof document === 'undefined') return false;
  const host = document.getElementById('local-usage-root');
  return Boolean(host && !host.hidden && document.visibilityState !== 'hidden');
}

export function UsageView() {
  const [range, setRange] = useState('all');
  const [timelineLabel, setTimelineLabel] = useState('All time');
  const [exportStatus, setExportStatus] = useState('');

  const quotaRef = useRef<HTMLDivElement | null>(null);
  const activityRef = useRef<HTMLDivElement | null>(null);
  const providersRef = useRef<HTMLDivElement | null>(null);
  const valueRef = useRef<HTMLElement | null>(null);
  const projectsRef = useRef<HTMLDivElement | null>(null);
  const evalsRef = useRef<HTMLDivElement | null>(null);

  const rangeRef = useRef(range);
  rangeRef.current = range;
  const lastQuotaKeyRef = useRef('');
  const lastReportKeyRef = useRef('');
  const exportDetailsRef = useRef<HTMLDetailsElement | null>(null);

  const loadQuota = useCallback(async () => {
    try {
      const res = await fetchLocalJson<ProviderOverview[]>('/api/v1/providers/overview');
      if (!Array.isArray(res)) throw new Error('Invalid quota response');
      const key = JSON.stringify(res) + Math.floor(Date.now() / 60000);
      if (key === lastQuotaKeyRef.current) return;
      if (quotaRef.current) {
        updateProviderCards(quotaRef.current, res, {
          cost: true,
          onResetBoundary: () => {
            lastQuotaKeyRef.current = '';
            void loadQuota();
          },
        });
      }
      lastQuotaKeyRef.current = key;
    } catch {
      lastQuotaKeyRef.current = '';
      if (quotaRef.current) {
        quotaRef.current.textContent =
          'Quota refresh failed. Last observations are stale; retry Reload view.';
      }
    }
  }, []);

  const loadReport = useCallback(async (targetRange: string) => {
    try {
      const res = await fetchLocalJson<LocalReportResponse>(
        buildRangeUrl('/api/v1/report', targetRange),
      );
      if (!res || !res.timeline) {
        throw new Error('The local report response is incomplete. Retry Reload view.');
      }
      if (targetRange !== rangeRef.current) return;
      const key = `${targetRange}:${JSON.stringify(res)}`;
      if (key === lastReportKeyRef.current) return;

      const tl = res.timeline;
      let label = typeof tl.range_label === 'string' && tl.range_label
        ? tl.range_label
        : PERIOD_LABELS[targetRange] || targetRange;
      if (tl.earliest_at && tl.latest_at) {
        const startText = new Date(tl.earliest_at).toLocaleDateString(undefined, {
          month: 'short',
          day: 'numeric',
          year: 'numeric',
        });
        const endText = new Date(tl.latest_at).toLocaleDateString(undefined, {
          month: 'short',
          day: 'numeric',
          year: 'numeric',
        });
        label =
          startText === endText
            ? `Showing: ${label} (${startText})`
            : `Showing: ${label} (${startText} - ${endText})`;
      }
      setTimelineLabel(label);

      if (activityRef.current) {
        updateActivity(activityRef.current, tl, { totals: false });
      }
      if (valueRef.current) {
        updateValue(valueRef.current, res.roi_by_provider, res.combined_roi);
      }
      if (providersRef.current) {
        updateProviders(providersRef.current, tl);
      }
      lastReportKeyRef.current = key;
    } catch (error) {
      if (targetRange !== rangeRef.current) return;
      lastReportKeyRef.current = '';
      const msg = error instanceof Error ? error.message : 'unavailable';
      if (activityRef.current) {
        activityRef.current.textContent = `Daily activity unavailable. ${msg}`;
      }
      if (providersRef.current) {
        providersRef.current.innerHTML =
          '<p class="local-view-message">Usage data is unavailable. Retry Reload view when the local daemon is reachable.</p>';
      }
    }
  }, []);

  const loadProjects = useCallback(async (targetRange: string) => {
    try {
      const projects = await fetchLocalJson<ProjectActivity[]>(
        buildRangeUrl('/api/v1/projects', targetRange),
      );
      if (targetRange !== rangeRef.current) return;
      if (projectsRef.current) {
        mountProjectList(projectsRef.current, projects, {
          heading: false,
          helper: 'Most recent first. Select a project to filter sessions.',
        });
      }
    } catch {
      if (projectsRef.current) {
        projectsRef.current.textContent = 'Project usage could not be refreshed. Retry Reload view.';
      }
    }
  }, []);

  const loadEvals = useCallback(async () => {
    try {
      const report = await fetchLocalJson<HandoffEvalReport>('/api/v1/evals/handoff');
      if (evalsRef.current) {
        evalsRef.current.innerHTML = renderHandoffEvalMarkup(report);
      }
    } catch {
      if (evalsRef.current) {
        evalsRef.current.innerHTML = renderHandoffEvalMarkup(null);
      }
    }
  }, []);

  const refreshAll = useCallback(
    async () => {
      const currentRange = rangeRef.current;
      await Promise.all([
        loadQuota(),
        loadReport(currentRange),
        loadProjects(currentRange),
        loadEvals(),
      ]);
    },
    [loadEvals, loadProjects, loadQuota, loadReport],
  );

  useEffect(() => {
    lastReportKeyRef.current = '';
    setTimelineLabel(PERIOD_LABELS[range] || range);
    void Promise.all([loadReport(range), loadProjects(range)]);
  }, [range, loadProjects, loadReport]);

  useEffect(() => {
    void refreshAll();
    const timer = window.setInterval(() => {
      if (isUsageVisible()) {
        void refreshAll();
      }
    }, POLL_INTERVAL_MS);

    const onViewChange = (event: Event) => {
      const detail = (event as CustomEvent<{ view?: string }>).detail;
      if (detail?.view === 'usage') {
        void refreshAll();
      }
    };
    const onReload = (event: Event) => {
      const detail = (event as CustomEvent<{ view?: string }>).detail;
      if (detail?.view === 'usage') {
        lastQuotaKeyRef.current = '';
        lastReportKeyRef.current = '';
        void refreshAll();
      }
    };

    window.addEventListener('gnomcode:view-change', onViewChange);
    window.addEventListener('gnomcode:reload-view', onReload);
    return () => {
      window.clearInterval(timer);
      window.removeEventListener('gnomcode:view-change', onViewChange);
      window.removeEventListener('gnomcode:reload-view', onReload);
    };
  }, [refreshAll]);

  const handleDaySelect = (date: string) => {
    if (!activityRef.current) return;
    const day = activityRef.current.querySelector<HTMLDetailsElement>(
      `.sp-activity-breakdown [data-date="${CSS.escape(date)}"]`,
    );
    if (!day) return;
    const group = day.closest<HTMLDetailsElement>('.sp-activity-breakdown');
    if (group) group.open = true;
    day.open = true;
    day.scrollIntoView({
      block: 'center',
      behavior: window.matchMedia('(prefers-reduced-motion: reduce)').matches
        ? 'auto'
        : 'smooth',
    });
  };

  const handleExport = async (format: 'csv' | 'json') => {
    setExportStatus('');
    if (exportDetailsRef.current) exportDetailsRef.current.open = false;
    try {
      const url = buildRangeUrl('/api/v1/export', range, `format=${format}`);
      const res = await fetch(url, {
        headers: localAuthHeaders(),
        signal: AbortSignal.timeout(15000),
      });
      if (res.status === 402) {
        setExportStatus('Export is unavailable. Reopen the local console and try again.');
        return;
      }
      if (!res.ok) {
        setExportStatus('Export failed. Reload the view and try again.');
        return;
      }
      const blob = await res.blob();
      const downloadUrl = window.URL.createObjectURL(blob);
      const anchor = document.createElement('a');
      anchor.href = downloadUrl;
      anchor.download = `gnomcode-report-${range}.${format}`;
      document.body.appendChild(anchor);
      anchor.click();
      document.body.removeChild(anchor);
      window.URL.revokeObjectURL(downloadUrl);
    } catch (error) {
      const msg = error instanceof Error ? error.message : String(error);
      setExportStatus(`Export failed. Reload the view and retry. Details: ${msg}`);
    }
  };

  return (
    <div className="sp-usage-view">
      <div className="timeline-bar">
        <div className="timeline-label">{timelineLabel}</div>
        <div className="sp-period-actions">
          <select
            aria-label="Reporting period"
            value={range}
            onChange={(event) => setRange(event.target.value)}
          >
            <option value="today">Today</option>
            <option value="month">This month</option>
            <option value="7d">Last 7 days</option>
            <option value="30d">Last 30 days</option>
            <option value="all">All time</option>
          </select>
          <details ref={exportDetailsRef} className="sp-export-menu">
            <summary>Export</summary>
            <div>
              <button
                type="button"
                className="btn-control"
                onClick={() => void handleExport('csv')}
              >
                CSV
              </button>
              <button
                type="button"
                className="btn-control"
                onClick={() => void handleExport('json')}
              >
                JSON
              </button>
            </div>
          </details>
        </div>
      </div>

      {exportStatus ? <p role="status">{exportStatus}</p> : null}
      <div ref={quotaRef}>
        <p>Loading provider limits...</p>
      </div>
      <ActivityGrid viewName="usage" onDaySelect={handleDaySelect} />
      <div ref={activityRef} className="sp-usage-daily">
        <p>Loading daily activity...</p>
      </div>
      <div ref={providersRef} className="sp-usage-providers" />
      <section ref={valueRef} className="sp-section" />
      <section className="sp-section">
        <header className="sp-section-head">
          <h2>Projects</h2>
        </header>
        <div ref={projectsRef}>Loading project usage...</div>
      </section>
      <details className="sp-section sp-eval-report" aria-label="Handoff quality evals">
        <summary className="sp-section-head">
          <h2>Handoff quality evals</h2>
        </summary>
        <div ref={evalsRef}>Loading handoff quality evals...</div>
      </details>
    </div>
  );
}
