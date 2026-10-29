export function sessionStateLabel(state: string): string;
export function renderActivity(report: unknown): string;
export function renderProviders(report: unknown): string;
export function updateProviders(element: HTMLElement, report: unknown): void;
export function updateSummary(element: HTMLElement, data: unknown): void;
export function updateActivity(element: HTMLElement, report: unknown): void;
export function updateSessions(element: HTMLElement, sessions: unknown, options?: { focus?: boolean; rotate?: boolean; illustrative?: boolean }): void;
export function renderSessions(sessions: unknown, options?: { focus?: boolean; rotate?: boolean; illustrative?: boolean }): string;
export function renderQuota(pools: unknown, publishedAt?: string | null, options?: { illustrative?: boolean }): string;
export function renderProjects(projects: unknown): string;
export function renderSummary(data: { activeCount?: number | null; report?: unknown; quotaCount?: number | null }): string;

export function updateOngoing(element: HTMLElement, sessions: unknown): void;
export function renderOngoing(sessions: unknown): string;

export function updateQuota(element: HTMLElement, pools: unknown, publishedAt?: string | null): void;

export function renderChatCards(events: unknown): string;
export function renderSessionList(sessions: unknown[], selected?: string, query?: string): string;
export function updateChat(element: HTMLElement, events: unknown): void;

export type PendingChat = {id:string; text:string; created_at:string; baseline:string[]; delivered:boolean};
export function reconcileChat(events:unknown[],pending:PendingChat[],state?:string): {events:unknown[];pending:PendingChat[]};
