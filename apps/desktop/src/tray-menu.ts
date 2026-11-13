// Pure formatting for the native tray menu (SPEC 3.13, plan 15.2). Every number, state and
// verdict comes from Rust fields; this file only turns them into short ASCII labels.

export interface TrayWindow {
  label?: string;
  pool?: string | null;
  window_label?: string;
  state?: string;
  remaining_pct?: number | null;
  resets_at?: string | null;
  reset_passed?: boolean;
  stale?: boolean;
  source_issue?: string | null;
}

export interface TrayProvider {
  provider?: string;
  display_name?: string;
  windows?: TrayWindow[];
  headline_window?: number | null;
  updated_at?: string | null;
  enable_command?: string | null;
  source_issue?: string | null;
}

export interface TraySession {
  id?: string;
  title?: string | null;
  project_name?: string;
  agent?: string;
  home_group?: string | null;
  last_event_at?: string;
}

export type TrayAction =
  | { kind: 'focus-session'; id: string }
  | { kind: 'open-view'; view: 'usage' | 'sessions' | 'settings' }
  | { kind: 'restart-to-update' }
  | { kind: 'copy'; text: string }
  | { kind: 'open' }
  | { kind: 'quit' };

export interface TrayItem {
  label?: string;
  type?: 'separator';
  enabled?: boolean;
  submenu?: TrayItem[];
  action?: TrayAction;
}

export interface TrayUpdateStatus {
  state: string;
  available_version?: string | null;
}

export interface TrayFormatOptions {
  /** IANA zone for reset times; the local zone when omitted. */
  timeZone?: string;
  /** Desktop update availability reported by the daemon. */
  update?: TrayUpdateStatus | null;
}

const MAX_SESSIONS_PER_SECTION = 3;
const MAX_TITLE = 40;
const MAX_LABEL = 90;
const MAX_TOOLTIP = 127;
const MIDDLE_DOT = '·';
const SEPARATOR: TrayItem = { type: 'separator' };

/** Plain ASCII plus the middle dot; typographic punctuation is folded, anything else dropped. */
export function plain(value: unknown, max = MAX_LABEL): string {
  const text = String(value ?? '')
    .replace(/[‘’′]/g, "'")
    .replace(/[“”]/g, '"')
    .replace(/[–—−]/g, '-')
    .replace(/…/g, '...')
    .replace(/[\r\n\t]+/g, ' ')
    .replace(/[^\x20-\x7e·]/g, '')
    .replace(/\s+/g, ' ')
    .trim();
  return text.length > max ? `${text.slice(0, Math.max(0, max - 3)).trimEnd()}...` : text;
}

const label = (text: string): string => plain(text);
const disabled = (text: string): TrayItem => ({ label: label(text), enabled: false });
const act = (text: string, action: TrayAction): TrayItem => ({ label: label(text), action });

function parts(date: Date, timeZone?: string): Record<string, string> {
  const formatted = new Intl.DateTimeFormat('en-US', {
    timeZone, weekday: 'short', year: 'numeric', month: 'numeric', day: 'numeric',
    hour: 'numeric', minute: '2-digit', hourCycle: 'h23',
  }).formatToParts(date);
  const out: Record<string, string> = {};
  for (const part of formatted) out[part.type] = part.value;
  return out;
}

/** `Thu 8:00 AM`, or `8:00 AM` when the moment falls on the same local day as `now`. */
function clock(date: Date, now: Date, timeZone: string | undefined, withTime: boolean): string {
  const at = parts(date, timeZone);
  const today = parts(now, timeZone);
  const sameDay = at.year === today.year && at.month === today.month && at.day === today.day;
  const hour = Number(at.hour) % 24;
  const time = `${hour % 12 === 0 ? 12 : hour % 12}:${at.minute} ${hour < 12 ? 'AM' : 'PM'}`;
  if (sameDay) return time;
  return withTime ? `${at.weekday} ${time}` : String(at.weekday);
}

function resetDate(window: TrayWindow): Date | null {
  if (!window.resets_at) return null;
  const date = new Date(window.resets_at);
  return Number.isNaN(date.getTime()) ? null : date;
}

function windowName(window: TrayWindow): string {
  return plain(window.window_label || window.label || 'Limit', 30);
}

function pct(window: TrayWindow): string | null {
  return typeof window.remaining_pct === 'number' && Number.isFinite(window.remaining_pct) ? `${Math.round(window.remaining_pct)}%` : null;
}

/** Headline text for one window, like the card summary: `Weekly 98% left · Thu`. */
export function windowHeadline(window: TrayWindow, now: Date, options: TrayFormatOptions = {}): string {
  const name = windowName(window);
  const reset = resetDate(window);
  const percent = pct(window);
  if (window.state === 'exhausted') {
    return reset ? `${name} out ${MIDDLE_DOT} back ${clock(reset, now, options.timeZone, true)}` : `${name} out`;
  }
  if (window.state === 'reset_unconfirmed' || window.state === 'reset_awaiting') return `${name} reset, not confirmed (EST)`;
  if (window.state === 'unknown' || percent === null) return `${name} unknown`;
  const head = window.state === 'low' ? `${name} low, ${percent} left` : `${name} ${percent} left`;
  const seen = window.stale ? ' (last seen)' : '';
  return reset ? `${head}${seen} ${MIDDLE_DOT} ${clock(reset, now, options.timeZone, false)}` : `${head}${seen}`;
}

/** Detail line inside a provider submenu: `Weekly: 98% left, resets Thu 8:00 AM`. */
export function windowDetail(window: TrayWindow, now: Date, options: TrayFormatOptions = {}): string {
  const name = windowName(window);
  const reset = resetDate(window);
  const percent = pct(window);
  if (window.state === 'reset_unconfirmed' || window.state === 'reset_awaiting') return `${name}: Reset, not confirmed (EST)`;
  if (window.state === 'exhausted') {
    return reset ? `${name}: exhausted, back ${clock(reset, now, options.timeZone, true)}` : `${name}: exhausted`;
  }
  if (window.state === 'unknown' || percent === null) return `${name}: unknown`;
  const bits = [window.state === 'low' ? `${percent} left (low)` : `${percent} left`];
  if (window.stale) bits.push('stale');
  if (reset) bits.push(`resets ${clock(reset, now, options.timeZone, true)}`);
  return `${name}: ${bits.join(', ')}`;
}

function windowsOf(provider: TrayProvider): TrayWindow[] {
  return Array.isArray(provider.windows) ? provider.windows : [];
}

function poolsOf(windows: TrayWindow[]): string[] {
  const seen: string[] = [];
  for (const window of windows) {
    const pool = window.pool ? plain(window.pool, 40) : '';
    if (pool && !seen.includes(pool)) seen.push(pool);
  }
  return seen;
}

function providerName(provider: TrayProvider): string {
  return plain(provider.display_name || provider.provider || 'Provider', 30);
}

/** Text after the provider name on the top-level item. */
export function providerHeadline(provider: TrayProvider, now: Date, options: TrayFormatOptions = {}): string {
  const windows = windowsOf(provider);
  if (windows.length === 0) return 'Limits off';
  const pools = poolsOf(windows);
  if (pools.length > 1) {
    const out = pools.filter(pool => windows.some(window => window.pool && plain(window.pool, 40) === pool && window.state === 'exhausted')).length;
    if (out > 0) return `${out} ${out === 1 ? 'pool' : 'pools'} out`;
  }
  const index = provider.headline_window;
  const headline = typeof index === 'number' && windows[index] ? windows[index] : windows[0];
  return windowHeadline(headline, now, options);
}

function providerSubmenu(provider: TrayProvider, now: Date, options: TrayFormatOptions): TrayItem[] {
  const windows = windowsOf(provider);
  if (windows.length === 0) {
    return provider.enable_command ? [act('Copy enable command', { kind: 'copy', text: String(provider.enable_command) })] : [disabled('Limits off')];
  }
  const items: TrayItem[] = [];
  const pools = poolsOf(windows);
  if (pools.length > 1) {
    pools.forEach((pool, index) => {
      if (index > 0) items.push(SEPARATOR);
      items.push(disabled(pool));
      for (const window of windows) if (window.pool && plain(window.pool, 40) === pool) items.push(disabled(windowDetail(window, now, options)));
    });
  } else {
    for (const window of windows) items.push(disabled(windowDetail(window, now, options)));
  }
  const issues = [provider.source_issue, ...windows.map(window => window.source_issue)]
    .filter((value, index, all): value is string => typeof value === 'string' && value.length > 0 && all.indexOf(value) === index);
  if (issues.length) items.push(SEPARATOR, ...issues.map(issue => disabled(issue)));
  items.push(SEPARATOR, act('Open limits', { kind: 'open-view', view: 'usage' }));
  return items;
}

function ago(then: Date, now: Date): string {
  const minutes = Math.max(0, Math.floor((now.getTime() - then.getTime()) / 60000));
  if (minutes < 1) return 'just now';
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  return hours < 48 ? `${hours}h ago` : `${Math.floor(hours / 24)}d ago`;
}

function sessionLabel(session: TraySession, providers: TrayProvider[]): string {
  const title = plain(session.title || session.project_name || 'Untitled session', MAX_TITLE);
  const agent = String(session.agent ?? '');
  const known = providers.find(provider => provider.provider === agent);
  const name = known?.display_name || AGENT_NAMES[agent] || agent;
  return name ? `${title} ${MIDDLE_DOT} ${plain(name, 24)}` : title;
}

const AGENT_NAMES: Record<string, string> = {
  'claude-code': 'Claude Code', claude: 'Claude Code', codex: 'Codex', antigravity: 'Antigravity',
};

function sessionSection(title: string, sessions: TraySession[], providers: TrayProvider[], oldestFirst: boolean): TrayItem[] {
  if (sessions.length === 0) return [];
  const time = (session: TraySession): number => Date.parse(session.last_event_at ?? '') || 0;
  const sorted = [...sessions].sort((a, b) => (oldestFirst ? time(a) - time(b) : time(b) - time(a)));
  const items: TrayItem[] = [disabled(`${title} (${sessions.length})`)];
  for (const session of sorted.slice(0, MAX_SESSIONS_PER_SECTION)) {
    items.push(session.id ? act(sessionLabel(session, providers), { kind: 'focus-session', id: String(session.id) }) : disabled(sessionLabel(session, providers)));
  }
  if (sessions.length > MAX_SESSIONS_PER_SECTION) items.push(disabled(`and ${sessions.length - MAX_SESSIONS_PER_SECTION} more`));
  return items;
}

/**
 * Builds the tray menu template. `overview` or `sessions` may be null when that request failed;
 * with both null the menu fails open to `Daemon unavailable` plus Open and Quit.
 */
export function formatTrayMenu(
  overview: TrayProvider[] | null,
  sessions: TraySession[] | null,
  now: Date,
  options: TrayFormatOptions = {},
): TrayItem[] {
  const open = act('Open Gnomcode', { kind: 'open' });
  const quit = act('Quit Gnomcode', { kind: 'quit' });
  if (overview === null && sessions === null) return [disabled('Daemon unavailable'), SEPARATOR, open, SEPARATOR, quit];

  const providers = overview ?? [];
  const list = sessions ?? [];
  const items: TrayItem[] = [
    ...sessionSection('Needs you', list.filter(session => session.home_group === 'needs_you'), providers, true),
    ...sessionSection('Running', list.filter(session => session.home_group === 'running'), providers, false),
  ];
  if (items.length) items.push(SEPARATOR);

  if (overview === null) items.push(disabled('Limits unavailable'));
  for (const provider of providers) {
    items.push({
      // Parts are already plain; the four spaces must survive whitespace folding.
      label: `${providerName(provider)}    ${plain(providerHeadline(provider, now, options), 60)}`,
      submenu: providerSubmenu(provider, now, options),
    });
  }
  items.push(SEPARATOR);

  const stamps = providers.map(provider => Date.parse(provider.updated_at ?? '')).filter(Number.isFinite);
  if (stamps.length) items.push(disabled(`Updated ${ago(new Date(Math.max(...stamps)), now)}`));
  if (options.update?.available_version && ['available', 'downloading', 'ready'].includes(options.update.state)) {
    items.push(act(`Update available: v${options.update.available_version}`, { kind: 'open-view', view: 'settings' }));
    if (options.update.state === 'ready') {
      items.push(act('Restart to update', { kind: 'restart-to-update' }));
    }
  }
  items.push(act('New session', { kind: 'open-view', view: 'sessions' }), open, SEPARATOR, quit);
  return items;
}

function tooltipPart(provider: TrayProvider): string | null {
  const windows = windowsOf(provider);
  if (windows.length === 0) return null;
  const index = provider.headline_window;
  const window = typeof index === 'number' && windows[index] ? windows[index] : windows[0];
  const name = providerName(provider).replace(/ Code$/, '');
  if (window.state === 'exhausted') return `${name} out`;
  if (window.state === 'reset_unconfirmed' || window.state === 'reset_awaiting') return `${name} reset, not confirmed (EST)`;
  const percent = pct(window);
  if (window.state === 'unknown' || percent === null) return null;
  return `${name} ${windowName(window).toLowerCase()} ${percent}`;
}

/** Tooltip with the tightest limit per provider; Windows allows at most 127 characters. */
export function formatTrayTooltip(overview: TrayProvider[] | null): string {
  if (overview === null) return 'Gnomcode: daemon unavailable';
  const text = ['Gnomcode', ...overview.map(tooltipPart).filter((part): part is string => part !== null)].join(` ${MIDDLE_DOT} `);
  return plain(text, MAX_TOOLTIP);
}
