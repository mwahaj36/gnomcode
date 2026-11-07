import {supabase} from './supabase';
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
      const reg = await navigator.serviceWorker.getRegistration();
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

function urlBase64ToUint8Array(base64String: string): ArrayBuffer {
  const padding = '='.repeat((4 - (base64String.length % 4)) % 4);
  const base64 = (base64String + padding).replace(/-/g, '+').replace(/_/g, '/');
  const rawData = window.atob(base64);
  const buffer = new ArrayBuffer(rawData.length);
  const outputArray = new Uint8Array(buffer);
  for (let i = 0; i < rawData.length; ++i) {
    outputArray[i] = rawData.charCodeAt(i);
  }
  return buffer;
}

export interface PushSubscriptionResult {
  ok: boolean;
  subscription?: PushSubscription;
  error?: string;
}

/** Subscribe mobile PWA or browser to VAPID Web Push server */
export async function subscribeToPushServer(): Promise<PushSubscriptionResult> {
  if (!isNotificationSupported() || !('serviceWorker' in navigator)) {
    return { ok: false, error: 'Push notifications are not supported on this browser.' };
  }

  const permission = await requestNotificationPermission();
  if (!permission) {
    return { ok: false, error: 'Notification permission was denied.' };
  }

  try {
    const keyRes = await fetch('/api/push/vapid-key',{signal:AbortSignal.timeout(15000)});
    const { configured, publicKey } = await keyRes.json();
    if (!configured || !publicKey) {
      return { ok: false, error: 'Server VAPID keys are not configured yet.' };
    }

    let timer:ReturnType<typeof setTimeout> | undefined;
    const reg=await Promise.race([
      navigator.serviceWorker.register('/sw.js').then(()=>navigator.serviceWorker.ready),
      new Promise<ServiceWorkerRegistration>((_,reject)=>{timer=setTimeout(()=>reject(new Error('Notification setup timed out. Reload this app and try again.')),10000);})
    ]).finally(()=>{if(timer)clearTimeout(timer);});
    let sub = await reg.pushManager.getSubscription();
    if (!sub) {
      sub = await reg.pushManager.subscribe({
        userVisibleOnly: true,
        applicationServerKey: urlBase64ToUint8Array(publicKey)
      });
    }

    const {data:{session}}=await supabase.auth.getSession();
    if(!session)return {ok:false,error:'Sign in before enabling background notifications.'};
    const saved=await fetch('/api/push/subscription',{method:'POST',signal:AbortSignal.timeout(15000),headers:{'Content-Type':'application/json',Authorization:`Bearer ${session.access_token}`},body:JSON.stringify(sub)});
    if(!saved.ok)return {ok:false,error:'Could not save background notification setup. Try again when connected.'};
    return { ok: true, subscription: sub };
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : 'Subscription failed';
    return { ok: false, error: message };
  }
}

/** Send a cold server-side Web Push test via VAPID */
export async function triggerServerPushTest(): Promise<{ ok: boolean; message: string }> {
  try {
    const registered=await subscribeToPushServer();
    if(!registered.ok)return {ok:false,message:registered.error || 'Notification registration failed.'};
    const {data:{session}}=await supabase.auth.getSession();
    if(!session)return {ok:false,message:'Sign in to test background notifications.'};
    const res = await fetch('/api/push/send', {
      method: 'POST',
      signal:AbortSignal.timeout(15000),
      headers: { 'Content-Type': 'application/json',Authorization:`Bearer ${session.access_token}` },
      body: JSON.stringify({event_id:`test-${crypto.randomUUID()}`,kind:'agent_event'})
    });

    const data = await res.json();
    if (res.ok && data.ok && data.devices > 0) {
      return { ok: true, message: 'The push service accepted the test. Check this device’s notification center.' };
    }
    return { ok: false, message: data.error || 'Push send rejected.' };
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : 'Server push failed';
    return { ok: false, message };
  }
}

/** Revokes only this browser's destination; signing out must not affect other devices. */
export async function disableDeviceNotifications():Promise<boolean>{
  try {
    if(!('serviceWorker' in navigator))return true;
    const registration=await navigator.serviceWorker.getRegistration();
    const subscription=await registration?.pushManager.getSubscription();
    if(!subscription)return true;
    const {data:{session}}=await supabase.auth.getSession();
    if(session){
      const response=await fetch('/api/push/subscription',{method:'DELETE',signal:AbortSignal.timeout(10000),headers:{'Content-Type':'application/json',Authorization:`Bearer ${session.access_token}`},body:JSON.stringify(subscription)});
      if(!response.ok)return false;
    }
    return await subscription.unsubscribe();
  }catch{return false;}
}
