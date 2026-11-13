'use client';

import { useCallback, useEffect, useRef } from 'react';
import {
  updateHeatmap,
  updateHomeSessions,
  updateProviderCards,
  updateTodayTiles,
  type ActivityHeatmap,
  type ProviderOverview,
} from '../lib/console-presentation.js';
import {
  fetchLocalJson,
  type LocalReportResponse,
  type LocalSessionsResponse,
} from './local-api';

const POLL_INTERVAL_MS = 15_000;
const TODAY_TTL_MS = 10_000;
const HEATMAP_TTL_MS = 60_000;

export function HomeView() {
  const limitsRef = useRef<HTMLElement | null>(null);
  const sessionsRef = useRef<HTMLElement | null>(null);
  const todayRef = useRef<HTMLElement | null>(null);
  const activityRef = useRef<HTMLDivElement | null>(null);

  const lastQuotaKeyRef = useRef('');
  const lastTodayAtRef = useRef(0);
  const lastTodayReportRef = useRef<unknown>(null);
  const lastHeatmapAtRef = useRef(0);
  const activeCountRef = useRef<number | null>(null);
  const inFlightRef = useRef(false);

  const renderToday = useCallback(() => {
    if (!todayRef.current) return;
    updateTodayTiles(todayRef.current, {
      report: lastTodayReportRef.current,
      activeCount: activeCountRef.current,
    });
  }, []);

  const loadToday = useCallback(
    async (force = false) => {
      if (!force && Date.now() - lastTodayAtRef.current < TODAY_TTL_MS) {
        renderToday();
        return;
      }
      lastTodayAtRef.current = Date.now();
      try {
        const start = new Date();
        start.setHours(0, 0, 0, 0);
        const res = await fetchLocalJson<LocalReportResponse>(
          `/api/v1/report?range=today&from=${encodeURIComponent(start.toISOString())}`,
        );
        lastTodayReportRef.current = res?.timeline ?? null;
      } catch {
        lastTodayReportRef.current = null;
        lastTodayAtRef.current = Date.now() - 5000;
      }
      renderToday();
    },
    [renderToday],
  );

  const loadHeatmap = useCallback(async (force = false) => {
    if (!force && Date.now() - lastHeatmapAtRef.current < HEATMAP_TTL_MS) return;
    lastHeatmapAtRef.current = Date.now();
    try {
      const data = await fetchLocalJson<ActivityHeatmap>('/api/v1/activity/heatmap');
      if (activityRef.current) {
        updateHeatmap(activityRef.current, data, {
          weeks: 12,
          stats: false,
          legend: false,
          months: false,
        });
      }
    } catch {
      lastHeatmapAtRef.current = Date.now() - 50000;
    }
  }, []);

  const loadQuota = useCallback(async () => {
    try {
      const res = await fetchLocalJson<ProviderOverview[]>('/api/v1/providers/overview');
      if (!Array.isArray(res)) throw new Error('Invalid quota response');
      const key = JSON.stringify(res) + Math.floor(Date.now() / 60000);
      if (key === lastQuotaKeyRef.current) return;
      if (limitsRef.current) {
        updateProviderCards(limitsRef.current, res, { cost: false });
      }
      lastQuotaKeyRef.current = key;
    } catch {
      lastQuotaKeyRef.current = '';
      if (limitsRef.current) {
        limitsRef.current.textContent = 'Provider limits unavailable. Retry Reload view.';
      }
    }
  }, []);

  const loadSessions = useCallback(async () => {
    try {
      const res = await fetchLocalJson<LocalSessionsResponse>('/api/v1/sessions?page=1&limit=100');
      const sessions = Array.isArray(res?.sessions) ? res.sessions : [];
      if (sessionsRef.current) {
        updateHomeSessions(sessionsRef.current, sessions);
      }
      activeCountRef.current = sessions.filter(
        (session) => session.home_group === 'needs_you' || session.home_group === 'running',
      ).length;
      await loadToday(false);
    } catch {
      if (sessionsRef.current) {
        sessionsRef.current.textContent = 'Session status unavailable. Retry Reload view.';
      }
    }
  }, [loadToday]);

  const refreshHome = useCallback(
    async (force = false) => {
      if (inFlightRef.current && !force) return;
      inFlightRef.current = true;
      try {
        await Promise.all([loadQuota(), loadSessions(), loadHeatmap(force)]);
      } finally {
        inFlightRef.current = false;
      }
    },
    [loadHeatmap, loadQuota, loadSessions],
  );

  useEffect(() => {
    void refreshHome(true);
    const timer = window.setInterval(() => {
      const activeView = document.body?.dataset?.consoleView || 'overview';
      if (!document.hidden && activeView === 'overview') {
        void refreshHome(false);
      }
    }, POLL_INTERVAL_MS);

    const onReload = () => {
      void refreshHome(true);
    };
    const onViewChange = (event: Event) => {
      const detail = (event as CustomEvent<{ view?: string }>).detail;
      if (!detail || detail.view === 'overview') {
        void refreshHome(false);
      }
    };

    window.addEventListener('gnomcode:reload-view', onReload);
    window.addEventListener('gnomcode:view-change', onViewChange);
    return () => {
      window.clearInterval(timer);
      window.removeEventListener('gnomcode:reload-view', onReload);
      window.removeEventListener('gnomcode:view-change', onViewChange);
    };
  }, [refreshHome]);

  return (
    <div className="sp-home-react">
      <section className="sp-section" id="home-limits" ref={limitsRef} />
      <section className="sp-section" id="home-sessions" ref={sessionsRef} />
      <section className="sp-section" id="home-today" ref={todayRef} />
      <div id="home-activity" ref={activityRef} />
    </div>
  );
}
