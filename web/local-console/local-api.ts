export interface LocalSessionSummary {
  id: string;
  home_group?: string;
}

export interface LocalSessionsResponse {
  sessions?: LocalSessionSummary[];
}

export interface LocalReportResponse {
  timeline?: unknown;
}

declare global {
  interface Window {
    __gnomcodeReactViews?: {
      overview?: boolean;
    };
    __gnomcodeLocalActions?: {
      setView?: (view: string) => void;
      openSession?: (sessionId: string) => void;
    };
  }
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

export async function fetchLocalJson<T>(url: string, timeoutMs = 15000): Promise<T> {
  const response = await fetch(url, {
    headers: localAuthHeaders(),
    signal: AbortSignal.timeout(timeoutMs),
  });
  if (!response.ok) {
    throw new Error(`Request failed (${response.status})`);
  }
  return (await response.json()) as T;
}
