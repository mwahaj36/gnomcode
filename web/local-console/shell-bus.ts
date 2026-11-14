/** What the mounted ShellDialogs component offers the rest of the console. */
export interface ShellDialogsController {
  confirm(message: string, actionLabel: string): Promise<boolean>;
  notice(message: string): void;
  startTour(): void;
}

let controller: ShellDialogsController | null = null;

export function registerShellDialogs(next: ShellDialogsController): () => void {
  controller = next;
  return () => {
    if (controller === next) controller = null;
  };
}

/** Modal confirmation; resolves false when dismissed or when the dialogs are not mounted. */
export function confirmAction(message: string, actionLabel = 'Continue'): Promise<boolean> {
  return controller ? controller.confirm(message, actionLabel) : Promise.resolve(false);
}

/** Dismissible status notice at the bottom of the console. */
export function showNotice(message: string): void {
  controller?.notice(message);
}

/** Opens the first-run tour from its first step, whatever the session count. */
export function startOnboarding(): void {
  controller?.startTour();
}

let providerSetupWanted = false;

/** Asks Settings to scroll to the provider setup checklist (from onboarding or the doctor link). */
export function requestProviderSetup(): void {
  providerSetupWanted = true;
  window.dispatchEvent(new CustomEvent('gnomcode:provider-setup'));
}

/** Returns and clears a pending provider setup request. */
export function takeProviderSetupRequest(): boolean {
  const wanted = providerSetupWanted;
  providerSetupWanted = false;
  return wanted;
}

/** The five places the shell can show; `auth` is the Account view reached from the rail or Settings. */
export type ConsoleView = 'overview' | 'sessions' | 'usage' | 'settings' | 'auth';

let navigator: ((view: string) => void) | null = null;

export function registerNavigator(next: (view: string) => void): () => void {
  navigator = next;
  return () => {
    if (navigator === next) navigator = null;
  };
}

/** Moves the shell to a place (old names like `limits` are resolved by the shell). */
export function navigateTo(view: ConsoleView | string): void {
  navigator?.(view);
}

export interface WorkspaceActionDetail {
  type: 'new-session' | 'open-session' | 'toggle-chat-list';
  sessionId?: string;
  folder?: string;
  focus?: boolean;
}

let workspaceIsReady = false;
const queuedWorkspaceActions: WorkspaceActionDetail[] = [];

function dispatchWorkspaceAction(detail: WorkspaceActionDetail): void {
  window.dispatchEvent(new CustomEvent('gnomcode:workspace-action', { detail }));
}

/** Asks the Workspace view to act; actions that arrive before it has mounted wait for it. */
export function workspaceAction(detail: WorkspaceActionDetail): void {
  if (workspaceIsReady) dispatchWorkspaceAction(detail);
  else queuedWorkspaceActions.push(detail);
}

/** Called by the Workspace view once it listens for actions. */
export function markWorkspaceReady(): void {
  workspaceIsReady = true;
  queuedWorkspaceActions.splice(0).forEach(dispatchWorkspaceAction);
}
