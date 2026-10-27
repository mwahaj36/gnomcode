export function renderActivity(report: unknown): string;
export function updateActivity(element: HTMLElement, report: unknown): void;
export function updateSessions(element: HTMLElement, sessions: unknown, options?: { focus?: boolean; rotate?: boolean; illustrative?: boolean }): void;
export function renderSessions(sessions: unknown, options?: { focus?: boolean; rotate?: boolean; illustrative?: boolean }): string;
export function renderQuota(pools: unknown, publishedAt?: string | null, options?: { illustrative?: boolean }): string;
export function renderProjects(projects: unknown): string;
export function renderSummary(data: { activeCount?: number | null; report?: unknown; quotaCount?: number | null }): string;
