// Hands a content-free widget snapshot to the Android app's home-screen widgets (ADR 0066).
// Runs only inside the Capacitor shell; a no-op in browsers. Never passes titles or conversation text.

type Day = { date: string; level: number };
type Heatmap = { days?: Day[]; stats?: { total_tokens?: number; current_streak?: number } } | null | undefined;
type Quota = { provider: string; used_pct: number; resets_at: string | null; basis: string };
type Session = { agent?: string; project_name?: string; home_group?: string | null };

type Bridge = { setSnapshot(options: { json: string }): Promise<void> };

function bridge(): Bridge | undefined {
  const cap = (globalThis as { Capacitor?: { isNativePlatform?: () => boolean; Plugins?: { WidgetBridge?: Bridge } } }).Capacitor;
  return cap?.isNativePlatform?.() ? cap.Plugins?.WidgetBridge : undefined;
}

/** Builds the widget payload; exported for tests. */
export function widgetSnapshot(generatedAt: string, heatmap: Heatmap, quota: Quota[], sessions: Session[]) {
  const headline = new Map<string, Quota>();
  for (const q of quota) {
    const seen = headline.get(q.provider);
    if (!seen || q.used_pct > seen.used_pct) headline.set(q.provider, q);
  }
  return {
    generated_at: generatedAt,
    heatmap: heatmap
      ? {
          days: (heatmap.days || []).slice(-17 * 7).map((d) => ({ date: d.date, level: d.level })),
          stats: { total_tokens: heatmap.stats?.total_tokens ?? 0, current_streak: heatmap.stats?.current_streak ?? 0 },
        }
      : null,
    limits: [...headline.values()].map(({ provider, used_pct, resets_at, basis }) => ({ provider, used_pct, resets_at, basis })),
    waiting: sessions
      .filter((s) => s.home_group === 'needs_you')
      .slice(0, 5)
      .map((s) => ({ agent: s.agent || '', project: s.project_name || '' })),
  };
}

/** Sends the snapshot to the widgets when running in the Android app. Fails silently. */
export function publishWidgetSnapshot(generatedAt: string | null | undefined, heatmap: Heatmap, quota: Quota[], sessions: Session[]): void {
  const target = bridge();
  if (!target || !generatedAt) return;
  target.setSnapshot({ json: JSON.stringify(widgetSnapshot(generatedAt, heatmap, quota, sessions)) }).catch(() => undefined);
}
