export interface LocalSessionSummary {
  id: string;
  agent?: string;
  title?: string;
  model?: string;
  state?: string;
  state_label?: string;
  source?: string;
  repo_path?: string;
  started_at?: string;
  home_group?: string;
  can_resume?: boolean;
  can_reset?: boolean;
  worktree?: boolean;
  worktree_branch?: string;
  worktree_path?: string;
  limits?: unknown[];
  project_name?: string;
  last_event_at?: string;
  input_tokens?: number;
  output_tokens?: number;
  total_tokens?: number;
  total_cost_usd?: number;
  cost_basis?: string;
}

export interface LocalSessionsResponse {
  sessions?: LocalSessionSummary[];
  page?: number;
  total_pages?: number;
  total_count?: number;
}

export interface LocalTimelineReport {
  range_label?: string;
  earliest_at?: string | null;
  latest_at?: string | null;
  [key: string]: unknown;
}

export interface LocalReportResponse {
  timeline?: LocalTimelineReport | null;
  roi_by_provider?: unknown[];
  combined_roi?: unknown;
}

export interface LocalDesktopBridge {
  chooseWorkspace?: () => Promise<string | null>;
  restartToUpdate?: () => Promise<void> | void;
}

export interface LocalTerminalInstance {
  cols: number;
  rows: number;
  loadAddon: (addon: unknown) => void;
  open: (container: HTMLElement) => void;
  write: (data: string | Uint8Array) => void;
  writeln: (line: string) => void;
  focus: () => void;
  dispose: () => void;
  onData: (cb: (data: string) => void) => { dispose: () => void };
}

export interface LocalTerminalConstructor {
  new (options: Record<string, unknown>): LocalTerminalInstance;
}

export interface LocalFitAddonNamespace {
  FitAddon: new () => { fit: () => void };
}

declare global {
  interface Window {
    Terminal?: LocalTerminalConstructor;
    FitAddon?: LocalFitAddonNamespace;
    gnomcodeDesktop?: LocalDesktopBridge;
  }
}

/** The marketing site address the daemon injects into the page, empty when unset. */
export function getConsoleBaseUrl(): string {
  const raw = document.getElementById('gnomcode-console-root')?.dataset.baseUrl || '';
  return raw && raw !== '__GNOMCODE_BASE_URL__' ? raw : '';
}

export function getLocalBearerToken(): string {
  if (typeof window === 'undefined') return '';
  try {
    return new URLSearchParams(window.location.search).get('token') || '';
  } catch {
    return '';
  }
}

export function localAuthHeaders(): Record<string, string> {
  const token = getLocalBearerToken();
  return token ? { Authorization: `Bearer ${token}` } : {};
}

/** A response the daemon answered with a non-2xx status (as opposed to a network failure or timeout). */
export class LocalApiError extends Error {
  constructor(
    message: string,
    readonly status: number,
  ) {
    super(message);
  }
}

export async function fetchLocalJson<T>(url: string, timeoutMs = 15000): Promise<T> {
  const response = await fetch(url, {
    headers: localAuthHeaders(),
    signal: AbortSignal.timeout(timeoutMs),
  });
  if (!response.ok) {
    throw new LocalApiError(`Request failed (${response.status})`, response.status);
  }
  return (await response.json()) as T;
}

export async function fetchLocalMutation<T>(
  url: string,
  options: { method: 'POST' | 'PUT' | 'DELETE'; body?: unknown; timeoutMs?: number },
): Promise<T> {
  const headers: Record<string, string> = { ...localAuthHeaders() };
  if (options.body !== undefined) {
    headers['Content-Type'] = 'application/json';
  }
  const response = await fetch(url, {
    method: options.method,
    headers,
    body: options.body !== undefined ? JSON.stringify(options.body) : undefined,
    signal: AbortSignal.timeout(options.timeoutMs ?? 15000),
  });
  if (!response.ok) {
    const err = await response.json().catch(() => null);
    const message =
      err && typeof err === 'object' && 'message' in err && typeof err.message === 'string'
        ? err.message
        : `Request failed (${response.status})`;
    throw new LocalApiError(message, response.status);
  }
  const text = await response.text();
  if (!text) return {} as T;
  return JSON.parse(text) as T;
}
