import { type FormEvent, useCallback, useEffect, useRef, useState } from 'react';
import { fetchLocalJson, fetchLocalMutation, getConsoleBaseUrl, LocalApiError } from './local-api';
import { advanceOnboardingAfterAccount } from './Onboarding';
import {
  clearOnboardingPausedForAuth,
  confirmAction,
  isOnboardingPausedForAuth,
  startOnboarding,
} from './shell-bus';
import { signOutDesktop } from './settings-index';

interface AccountState {
  paired?: boolean;
  email?: string | null;
}

interface SyncStatus {
  message?: string;
  last_success_at?: string | null;
}

type Tone = 'muted' | 'ok' | 'error';

const ACCOUNT_ACCESS_DISCLOSURE =
  'Linking this native app makes its available conversations and supported session controls accessible from any device signed in to your account, anywhere. This includes devices already signed in and devices you sign in to later. No phone pairing or per-desktop approval will be required. This desktop must stay awake with Gnomcode running. Protect your account: anyone with access to it can use these controls. You can disable account access in Devices or disconnect this desktop.';

const confirmAccountAccess = () => confirmAction(ACCOUNT_ACCESS_DISCLOSURE, 'Link and allow account access');

/** Turns what the person pasted (JSON, base64 JSON or a bare token) into the session body. */
function parseConnection(value: string): Record<string, unknown> {
  try {
    return JSON.parse(value);
  } catch {
    try {
      return JSON.parse(atob(value));
    } catch {
      return { access_token: value, token_type: 'bearer' };
    }
  }
}

/** Account view: connection status, sign-in on this desktop, token pairing and disconnect. */
export function AccountView() {
  const [account, setAccount] = useState<AccountState | null>(null);
  const [cloudStatus, setCloudStatus] = useState('Checking account connection...');
  const [publication, setPublication] = useState('Checking the last desktop update...');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [token, setToken] = useState('');
  const [showToken, setShowToken] = useState(false);
  const [busy, setBusy] = useState<'signin' | 'oauth' | 'pair' | 'unpair' | null>(null);
  const [status, setStatus] = useState<{ text: string; tone: Tone } | null>(null);
  const [onboardingPaused, setOnboardingPaused] = useState(() => isOnboardingPausedForAuth());
  const [localUsageCount, setLocalUsageCount] = useState<number | null>(null);
  const [restoreBusy, setRestoreBusy] = useState(false);
  const [restoreMsg, setRestoreMsg] = useState('');
  const busyRef = useRef(false);
  const cloudStatusRef = useRef<HTMLParagraphElement | null>(null);

  const check = useCallback(async (): Promise<boolean> => {
    const [session, sync, usageSync] = await Promise.allSettled([
      fetchLocalJson<AccountState>('/api/v1/auth/session'),
      fetchLocalJson<SyncStatus>('/api/v1/sync/status'),
      fetchLocalJson<{ local_usage_count?: number }>('/api/v1/sync/usage/status'),
    ]);
    if (session.status !== 'fulfilled') {
      setCloudStatus('Account connection unavailable. Retry Reload view.');
      setPublication('Local monitoring does not require an account connection.');
      setAccount({ paired: false });
      return false;
    }
    const value = session.value;
    const info = sync.status === 'fulfilled' ? sync.value : null;
    setPublication(
      info
        ? `${info.message} Last published: ${info.last_success_at ? new Date(info.last_success_at).toLocaleString() : 'not yet'}.`
        : 'Publication status unavailable. Retry Reload view.',
    );
    setCloudStatus(value.paired ? 'Connected to your account' : 'Not connected. Local monitoring is ready.');
    setAccount({ paired: Boolean(value.paired), email: value.email });
    if (usageSync.status === 'fulfilled' && typeof usageSync.value.local_usage_count === 'number') {
      setLocalUsageCount(usageSync.value.local_usage_count);
    }
    if (value.paired && isOnboardingPausedForAuth()) {
      clearOnboardingPausedForAuth();
      setOnboardingPaused(false);
      advanceOnboardingAfterAccount();
      startOnboarding();
    }
    return true;
  }, []);

  useEffect(() => {
    setOnboardingPaused(isOnboardingPausedForAuth());
    void check();
    const onReload = (event: Event) => {
      if ((event as CustomEvent<{ view?: string }>).detail?.view === 'auth') {
        setOnboardingPaused(isOnboardingPausedForAuth());
        void check();
      }
    };
    window.addEventListener('gnomcode:reload-view', onReload);
    return () => window.removeEventListener('gnomcode:reload-view', onReload);
  }, [check]);

  // The rail offers "Sign in" only while this desktop is not connected.
  useEffect(() => {
    if (account) window.dispatchEvent(new CustomEvent('gnomcode:account-state', { detail: { paired: Boolean(account.paired) } }));
  }, [account]);

  const signIn = async (event: FormEvent) => {
    event.preventDefault();
    if (busyRef.current) return;
    busyRef.current = true;
    setBusy('signin');
    setStatus({ text: '', tone: 'muted' });
    try {
      if (!(await confirmAccountAccess())) {
        setStatus(null);
        return;
      }
      await fetchLocalMutation('/api/v1/auth/signin', {
        method: 'POST',
        body: { email: email.trim(), password, remote_access_confirmed: true },
      });
      setPassword('');
      setStatus({ text: 'Signed in. Your desktop will appear in Devices.', tone: 'muted' });
      await check();
    } catch {
      setStatus({
        text: 'Sign-in failed. Check your email, password and connection. Browser accounts can use the connection option below.',
        tone: 'muted',
      });
    } finally {
      busyRef.current = false;
      setBusy(null);
    }
  };

  // GitHub sign-in opens the system browser; the daemon finishes it on its loopback callback.
  const signInWithGitHub = async () => {
    if (busyRef.current) return;
    busyRef.current = true;
    setBusy('oauth');
    setStatus(null);
    try {
      if (!(await confirmAccountAccess())) return;
      const { url } = await fetchLocalMutation<{ url: string }>('/api/v1/auth/oauth/start', {
        method: 'POST',
        body: { provider: 'github', remote_access_confirmed: true },
      });
      window.open(url, '_blank', 'noopener');
      setStatus({ text: 'Finish signing in with GitHub in your browser, then come back here.', tone: 'muted' });
      const deadline = Date.now() + 10 * 60_000;
      while (Date.now() < deadline) {
        await new Promise((resolve) => setTimeout(resolve, 3000));
        const session = await fetchLocalJson<AccountState>('/api/v1/auth/session').catch(() => null);
        if (session?.paired) {
          setStatus({ text: 'Signed in. Your desktop will appear in Devices.', tone: 'muted' });
          await check();
          return;
        }
      }
      setStatus({ text: 'GitHub sign-in timed out. Try again.', tone: 'muted' });
    } catch {
      setStatus({ text: 'GitHub sign-in could not start. Check your connection and try again.', tone: 'muted' });
    } finally {
      busyRef.current = false;
      setBusy(null);
    }
  };

  const pair = async (event: FormEvent) => {
    event.preventDefault();
    const value = token.trim();
    if (busyRef.current || !value) return;
    busyRef.current = true;
    setBusy('pair');
    setStatus({ text: 'Connecting this desktop...', tone: 'muted' });
    try {
      const session = parseConnection(value);
      if (!(await confirmAccountAccess())) {
        setStatus(null);
        return;
      }
      session.remote_access_confirmed = true;
      await fetchLocalMutation('/api/v1/auth/session', { method: 'POST', body: session, timeoutMs: 30000 });
      setStatus({ text: 'Desktop connected. Waiting for its next metadata update.', tone: 'ok' });
      setToken('');
      setShowToken(false);
      if (await check()) cloudStatusRef.current?.focus({ preventScroll: true });
    } catch (error) {
      const refused = error instanceof LocalApiError;
      setStatus({
        text: refused
          ? 'Failed to save session. Check token and try again.'
          : 'Connection could not be confirmed. Reload account status before retrying.',
        tone: 'error',
      });
    } finally {
      busyRef.current = false;
      setBusy(null);
    }
  };

  const unpair = async () => {
    if (busyRef.current) return;
    busyRef.current = true;
    setBusy('unpair');
    try {
      if (await signOutDesktop()) await check();
    } finally {
      busyRef.current = false;
      setBusy(null);
    }
  };

  const paired = account?.paired === true;
  const unpaired = account !== null && !paired;
  const statusClass = status?.tone === 'ok' ? 'sp-tone-green' : status?.tone === 'error' ? 'sp-tone-red' : undefined;

  return (
    <section className="sp-account-panel" aria-labelledby="account-heading">
      {onboardingPaused ? (
        <div className="sp-account-state" role="region" aria-label="Setup in progress">
          <p>Sign in below, or return to the setup checklist to finish configuring your providers.</p>
          <button
            type="button"
            className="btn-control"
            data-onboarding-action="return-to-setup"
            onClick={() => {
              clearOnboardingPausedForAuth();
              setOnboardingPaused(false);
              startOnboarding();
            }}
          >
            Return to setup
          </button>
        </div>
      ) : null}
      <div className="sp-account-intro">
        <div>
          <span className="sp-account-label">THIS DESKTOP</span>
          <h2 id="account-heading">Your account, on this device.</h2>
          <p>Keep monitoring locally, or connect your account to view this desktop from the web console.</p>
        </div>
        <a href={`${getConsoleBaseUrl()}/console`} target="_blank" rel="noopener noreferrer" className="btn-control">
          Open web console
        </a>
      </div>
      <div className="sp-account-state">
        <span className="sp-account-label">CONNECTION</span>
        <p id="auth-cloud-status" role="status" tabIndex={-1} ref={cloudStatusRef}>
          {cloudStatus}
        </p>
        <p id="auth-publication-status">{publication}</p>
      </div>
      <div className="sp-account-columns">
        <div className="sp-account-guide">
          <h3>Local first. Connected when you choose.</h3>
          <p>
            Connecting publishes project names, session IDs, agent activity, usage and quotas to your account. This snapshot is not
            end-to-end encrypted. New snapshots exclude conversation text and titles; older copies may still contain titles. Project
            names and other metadata can still be sensitive. Remote conversations use a separate encrypted connection.
          </p>
          <p>Snapshots update about once a minute while gnomcode is running. Local monitoring works without an account.</p>
        </div>
        <div>
          <div id="auth-paired-card" hidden={!paired}>
            <h3>Desktop connected</h3>
            <p id="auth-paired-email">{account?.email || 'Connected account'}</p>
            <p>
              This desktop is linked to your account. Account access lets any signed-in device open its available sessions.
              Signing out disconnects this desktop, stops Remote and keeps recorded local sessions.
            </p>
            <button id="auth-disconnect-button" type="button" className="btn-control" disabled={busy === 'unpair'} onClick={() => void unpair()}>
              {busy === 'unpair' ? 'Signing out...' : 'Sign out'}
            </button>
            {localUsageCount === 0 ? (
              <div style={{ marginTop: '16px' }}>
                <p>No local usage history yet. Pull backed-up usage records from your account if you have any.</p>
                <button
                  id="auth-restore-usage-button"
                  type="button"
                  className="btn-control"
                  disabled={restoreBusy}
                  onClick={async () => {
                    setRestoreBusy(true);
                    setRestoreMsg('Restoring usage history...');
                    try {
                      const res = await fetchLocalMutation<{ restored_records: number; cloud_records: number }>(
                        '/api/v1/sync/usage/restore',
                        { method: 'POST', timeoutMs: 60000 },
                      );
                      setRestoreMsg(
                        `Restored ${res.restored_records} record${res.restored_records === 1 ? '' : 's'} (${res.cloud_records} in cloud).`,
                      );
                      if (res.restored_records > 0) setLocalUsageCount(res.restored_records);
                    } catch (error) {
                      setRestoreMsg(error instanceof Error ? error.message : 'Restore failed.');
                    } finally {
                      setRestoreBusy(false);
                    }
                  }}
                >
                  {restoreBusy ? 'Restoring...' : 'Restore usage history'}
                </button>
                {restoreMsg ? <p role="status">{restoreMsg}</p> : null}
              </div>
            ) : null}
          </div>
          <div id="auth-unpaired-card" hidden={!unpaired}>
            <h3>Sign in on this desktop</h3>
            <button id="auth-github-button" type="button" className="btn-control" disabled={busy === 'oauth'} onClick={() => void signInWithGitHub()}>
              {busy === 'oauth' ? 'Waiting for GitHub...' : 'Continue with GitHub'}
            </button>
            <p className="sp-auth-divider">or use email</p>
            <form onSubmit={(event) => void signIn(event)}>
              <label htmlFor="auth-email">Email</label>
              <input id="auth-email" type="email" autoComplete="username" required value={email} onChange={(event) => setEmail(event.target.value)} />
              <label htmlFor="auth-password">Password</label>
              <input
                id="auth-password"
                type="password"
                autoComplete="current-password"
                required
                value={password}
                onChange={(event) => setPassword(event.target.value)}
              />
              <button id="auth-signin-button" type="submit" className="btn-primary" disabled={busy === 'signin'}>
                {busy === 'signin' ? 'Signing in...' : 'Sign in'}
              </button>
            </form>
            <p>Use the same account on your phone. Your desktop appears automatically.</p>
            <details>
              <summary>Connect an existing browser account</summary>
              <ol className="sp-pairing-steps">
                <li>Sign in to the web console.</li>
                <li>
                  Select <strong>Copy account connection</strong>.
                </li>
                <li>Paste the token below and connect.</li>
              </ol>
              <form onSubmit={(event) => void pair(event)}>
                <label htmlFor="auth-token-input">Account connection</label>
                <div className="sp-pairing-input">
                  <input
                    type={showToken ? 'text' : 'password'}
                    id="auth-token-input"
                    autoComplete="off"
                    spellCheck={false}
                    placeholder="Paste your account connection"
                    required
                    disabled={busy === 'pair'}
                    value={token}
                    onChange={(event) => setToken(event.target.value)}
                  />
                  <button
                    type="button"
                    className="btn-control"
                    aria-controls="auth-token-input"
                    aria-pressed={showToken}
                    onClick={() => setShowToken((visible) => !visible)}
                  >
                    {showToken ? 'Hide token' : 'Show token'}
                  </button>
                  <button id="auth-pair-button" type="submit" className="btn-primary" disabled={busy === 'pair'}>
                    {busy === 'pair' ? 'Connecting...' : 'Connect desktop'}
                  </button>
                </div>
                <p className="sp-pairing-hint">Treat this token like a password. Only paste it into your local gnomcode app.</p>
              </form>
            </details>
          </div>
          <p id="auth-pair-status" role="status" className={statusClass} hidden={status === null}>
            {status?.text}
          </p>
        </div>
      </div>
    </section>
  );
}
