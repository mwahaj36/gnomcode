/**
 * Gnomcode notification service for desktop browsers and mobile Progressive Web Apps.
 * Supports Web Notification API, Service Worker showNotification, and optional Web Audio chime.
 */

export type NotificationStatus = 'granted' | 'denied' | 'default' | 'unsupported';

export interface NotificationPayload {
  title: string;
  body?: string;
  tag?: string;
  data?: Record<string, unknown>;
  icon?: string;
  silent?: boolean;
}

/** Check if Notification API is supported in this browser/device environment */
export function isNotificationSupported(): boolean {
  return typeof window !== 'undefined' && 'Notification' in window;
}

/** Current permission status */
export function getNotificationPermission(): NotificationStatus {
  if (!isNotificationSupported()) return 'unsupported';
  return window.Notification.permission;
}

/** Check if currently running in installed standalone PWA mode */
export function isStandalonePWA(): boolean {
  if (typeof window === 'undefined') return false;
  const isStandaloneMedia = window.matchMedia('(display-mode: standalone)').matches;
  const isIOSStandalone = (window.navigator as unknown as { standalone?: boolean }).standalone === true;
  return isStandaloneMedia || isIOSStandalone;
}

/** Request notification permission from user */
export async function requestNotificationPermission(): Promise<boolean> {
  if (!isNotificationSupported()) return false;
  try {
    const permission = await window.Notification.requestPermission();
    return permission === 'granted';
  } catch (err) {
    console.warn('[Gnomcode] Notification request error:', err);
    return false;
  }
}

/** Play a subtle geometric tone using Web Audio API (no external file dependency) */
function playChime() {
  try {
    const AudioCtx = window.AudioContext || (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
    if (!AudioCtx) return;
    const ctx = new AudioCtx();
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();

    osc.type = 'sine';
    osc.frequency.setValueAtTime(587.33, ctx.currentTime); // D5
    osc.frequency.exponentialRampToValueAtTime(880, ctx.currentTime + 0.12); // A5

    gain.gain.setValueAtTime(0.08, ctx.currentTime);
    gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + 0.25);

    osc.connect(gain);
    gain.connect(ctx.destination);

    osc.start();
    osc.stop(ctx.currentTime + 0.25);
  } catch {
    // Audio context may be restricted before user gesture
  }
}

/** Send notification to desktop or mobile PWA */
export async function sendAppNotification(payload: NotificationPayload): Promise<boolean> {
  if (!isNotificationSupported()) return false;
  if (window.Notification.permission !== 'granted') return false;

  const title = payload.title || 'Gnomcode';
  const icon = payload.icon || '/icons/gnomcode-192.png';
  const options: NotificationOptions & { renotify?: boolean; vibrate?: number[] } = {
    body: payload.body || '',
    icon,
    badge: '/icons/gnomcode-192.png',
    tag: payload.tag || 'gnomcode-alert',
    data: payload.data || { url: '/remote' },
    renotify: true
  };

  if (!payload.silent) {
    playChime();
  }

  // Mobile PWA / Service Worker path (required for mobile notifications)
  if ('serviceWorker' in navigator) {
    try {
      const reg = await navigator.serviceWorker.ready;
      if (reg && 'showNotification' in reg) {
        await reg.showNotification(title, {
          ...options,
          // Vibration pattern: buzz 150ms, pause 50ms, buzz 150ms
          ...({ vibrate: [150, 50, 150] } as unknown as object)
        });
        return true;
      }
    } catch (err) {
      console.warn('[Gnomcode] ServiceWorker notification error, falling back to window.Notification:', err);
    }
  }

  // Desktop browser fallback
  try {
    const notification = new window.Notification(title, options);
    notification.onclick = () => {
      window.focus();
      notification.close();
    };
    return true;
  } catch (err) {
    console.warn('[Gnomcode] Desktop notification error:', err);
    return false;
  }
}

/** Test notification for user verification */
export async function triggerTestNotification(): Promise<boolean> {
  const granted = await requestNotificationPermission();
  if (!granted) return false;

  return sendAppNotification({
    title: 'Gnomcode Notifications Ready',
    body: 'Alerts are configured for agent turns, approvals and completions on this device.',
    tag: 'gnomcode-test',
    data: { url: '/remote' }
  });
}

/** Register PWA service worker */
export function registerPwaServiceWorker(): void {
  if (typeof window === 'undefined' || !('serviceWorker' in navigator)) return;

  window.addEventListener('load', () => {
    navigator.serviceWorker.register('/sw.js').then((reg) => {
      // Check for updates
      reg.addEventListener('updatefound', () => {
        const installingWorker = reg.installing;
        if (!installingWorker) return;
        installingWorker.addEventListener('statechange', () => {
          if (installingWorker.state === 'installed' && navigator.serviceWorker.controller) {
            console.info('[Gnomcode] PWA updated in background.');
          }
        });
      });
    }).catch((err) => {
      console.warn('[Gnomcode] Service worker registration failed:', err);
    });
  });
}
