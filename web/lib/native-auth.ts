// OAuth for the Android app (ADR 0066). Providers refuse or break inside an embedded WebView, so the
// app opens the system browser and the provider returns to dev.gnomcode.app://auth/callback; the
// native shell then loads /auth?code=... and this module exchanges the code (PKCE) for a session.
import { createClient } from '@supabase/supabase-js';
import { supabaseUrl, supabasePublishableKey } from './supabase';

export const NATIVE_AUTH_REDIRECT = 'dev.gnomcode.app://auth/callback';

/** True inside the Capacitor Android shell. */
export function isNativeApp(): boolean {
  const cap = (globalThis as { Capacitor?: { isNativePlatform?: () => boolean } }).Capacitor;
  return Boolean(cap?.isNativePlatform?.());
}

// Same storage key as the default client, so the session it saves is the one the app reads.
const pkce = createClient(supabaseUrl, supabasePublishableKey, { auth: { flowType: 'pkce', detectSessionInUrl: false } });

/** Opens the provider in the system browser. */
export async function startNativeOAuth(provider: 'github' | 'google'): Promise<string | null> {
  const { data, error } = await pkce.auth.signInWithOAuth({ provider, options: { redirectTo: NATIVE_AUTH_REDIRECT, skipBrowserRedirect: true } });
  if (error || !data?.url) return error?.message || 'Sign-in could not start.';
  window.location.href = data.url;
  return null;
}

/** Completes a native sign-in when the shell returned with ?code=. Returns true when signed in. */
export async function finishNativeOAuth(code: string): Promise<boolean> {
  const { error } = await pkce.auth.exchangeCodeForSession(code);
  return !error;
}
