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
