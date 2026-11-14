import { fetchLocalJson, fetchLocalMutation, LocalApiError } from './local-api';
import { confirmAction, showNotice } from './shell-bus';

/** Settings sections in display order, with search keywords. Used by Settings search and the command palette. */
export const SETTINGS_SECTIONS: { id: string; title: string; keywords: string }[] = [
  { id: 'settings-account', title: 'Account', keywords: 'account sign out log out logout disconnect email sync manage' },
  { id: 'settings-providers', title: 'Providers', keywords: 'providers claude codex antigravity install sign in integrate hook limits wsl setup add provider' },
  { id: 'settings-notifications', title: 'Notifications', keywords: 'notifications alerts push sound desktop phone waiting' },
  { id: 'settings-remote', title: 'Remote and devices', keywords: 'remote devices phone pair unpair qr access mobile' },
  { id: 'settings-automations', title: 'Automations', keywords: 'automations reconnect resume rules spend limit guardrail usage rules' },
  { id: 'settings-privacy', title: 'Privacy and data', keywords: 'privacy data reload rescan local history delete export' },
  { id: 'settings-appearance', title: 'Appearance', keywords: 'appearance theme dark light mode' },
  { id: 'settings-updates', title: 'Updates', keywords: 'updates desktop restart version' },
  { id: 'settings-about', title: 'About', keywords: 'about guide getting started onboarding documentation pricing download version' },
];

/** Whether a section matches a search query (every word must hit the title or keywords). */
export function sectionMatches(section: { title: string; keywords: string }, query: string): boolean {
  const hay = `${section.title} ${section.keywords}`.toLowerCase();
  return query
    .toLowerCase()
    .split(/\s+/)
    .filter(Boolean)
    .every((word) => hay.includes(word));
}

/** Account connection for this desktop (`GET /api/v1/auth/session`). */
export async function fetchAccount(): Promise<{ paired: boolean; email?: string | null }> {
  const value = await fetchLocalJson<{ paired?: boolean; email?: string | null }>('/api/v1/auth/session');
  return { paired: Boolean(value.paired), email: value.email };
}

/** Confirmed sign out: disconnects this desktop from the account. Local sessions stay. */
export async function signOutDesktop(): Promise<boolean> {
  if (
    !(await confirmAction(
      'Sign out and disconnect this desktop from your account? Remote stops. Recorded local sessions stay on this device.',
      'Sign out',
    ))
  ) {
    return false;
  }
  try {
    await fetchLocalMutation('/api/v1/auth/session', { method: 'DELETE', timeoutMs: 30000 });
    window.dispatchEvent(new CustomEvent('gnomcode:account-state', { detail: { paired: false } }));
    showNotice('Signed out. This desktop is no longer connected to your account.');
    return true;
  } catch (error) {
    showNotice(
      error instanceof LocalApiError
        ? 'Could not sign out. Reload the account status and retry.'
        : 'Connection could not be confirmed. Reload the account status before retrying.',
    );
    return false;
  }
}
