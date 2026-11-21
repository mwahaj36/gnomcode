import { useSyncExternalStore } from 'react';

const STORAGE_KEY = 'gnomcode-chat-list-collapsed';
const isPhone = () => window.matchMedia('(max-width: 899px)').matches;

function stored(): string | null {
  try {
    return localStorage.getItem(STORAGE_KEY);
  } catch {
    return null;
  }
}

// Open on desktop unless the person collapsed it; phones start collapsed.
let collapsed = isPhone() || stored() === '1';
const listeners = new Set<() => void>();

function apply(next: boolean, persist: boolean): void {
  collapsed = next;
  document.body.classList.toggle('chat-list-collapsed', next);
  if (persist && !isPhone()) {
    try {
      localStorage.setItem(STORAGE_KEY, next ? '1' : '0');
    } catch {
      // Storage unavailable
    }
  }
  listeners.forEach((listener) => listener());
}

apply(collapsed, false);

window.matchMedia('(max-width: 899px)').addEventListener('change', (event) => {
  apply(event.matches || stored() === '1', false);
});

/** Flips the conversation list and remembers the choice on desktop. */
export function toggleChatList(): void {
  apply(!collapsed, true);
}

/** Collapses the list after opening a conversation on a phone; never remembered. */
export function collapseChatList(): void {
  apply(true, false);
}

export function useChatListCollapsed(): boolean {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => collapsed,
    () => collapsed,
  );
}
