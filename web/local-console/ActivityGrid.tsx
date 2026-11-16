'use client';

import { useCallback, useEffect, useRef } from 'react';
import { updateHeatmap, type ActivityHeatmap } from '../lib/console-presentation.js';
import { fetchLocalJson } from './local-api';

const HEATMAP_TTL_MS = 60_000;
const POLL_INTERVAL_MS = 15_000;

interface ActivityGridProps {
  id?: string;
  viewName: 'overview' | 'usage';
  /** Weeks of columns to show; year-only stats and month labels appear only at 53. */
  weeks?: number;
  onDaySelect?: (date: string) => void;
}

/** Shared trailing-year UTC activity grid used by Overview and Usage views. */
export function ActivityGrid({ id, viewName, weeks = 53, onDaySelect }: ActivityGridProps) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const lastFetchedAtRef = useRef(0);

  const scrollToToday = useCallback(() => {
    const scroller = containerRef.current?.querySelector<HTMLElement>('.sp-heatmap-scroll');
    if (scroller) {
      scroller.scrollLeft = scroller.scrollWidth;
    }
  }, []);

  const loadHeatmap = useCallback(async (force = false) => {
    if (!force && Date.now() - lastFetchedAtRef.current < HEATMAP_TTL_MS) return;
    lastFetchedAtRef.current = Date.now();
    try {
      const data = await fetchLocalJson<ActivityHeatmap>('/api/v1/activity/heatmap');
      if (containerRef.current) {
        updateHeatmap(containerRef.current, data, {
          weeks,
          stats: weeks >= 53,
          legend: true,
          months: weeks >= 53,
        });
      }
    } catch {
      lastFetchedAtRef.current = Date.now() - 50_000;
      if (containerRef.current && !containerRef.current.firstChild) {
        containerRef.current.innerHTML =
          '<div class="sp-empty"><p>Activity could not be loaded.</p></div>';
      }
    }
  }, [weeks]);

  useEffect(() => {
    void loadHeatmap(true).then(() => scrollToToday());
    const timer = window.setInterval(() => {
      const activeView = document.body?.dataset?.consoleView || 'overview';
      if (!document.hidden && activeView === viewName) {
        void loadHeatmap(false);
      }
    }, POLL_INTERVAL_MS);

    const onReload = () => {
      void loadHeatmap(true).then(() => scrollToToday());
    };
    const onViewChange = (event: Event) => {
      const detail = (event as CustomEvent<{ view?: string }>).detail;
      if (!detail || detail.view === viewName) {
        void loadHeatmap(false).then(() => scrollToToday());
      }
    };

    window.addEventListener('gnomcode:reload-view', onReload);
    window.addEventListener('gnomcode:view-change', onViewChange);
    return () => {
      window.clearInterval(timer);
      window.removeEventListener('gnomcode:reload-view', onReload);
      window.removeEventListener('gnomcode:view-change', onViewChange);
    };
  }, [loadHeatmap, scrollToToday, viewName]);

  const handleClick = (event: React.MouseEvent<HTMLDivElement>) => {
    if (!onDaySelect) return;
    const cell = (event.target as HTMLElement).closest<HTMLElement>(
      '.sp-heatmap-grid .sp-heat[data-date]',
    );
    const date = cell?.dataset.date;
    if (date) onDaySelect(date);
  };

  return <div id={id} ref={containerRef} onClick={onDaySelect ? handleClick : undefined} />;
}
