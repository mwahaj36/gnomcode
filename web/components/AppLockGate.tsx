'use client';

import { useEffect, useState, type ReactNode } from 'react';
import {
  clearAppLockState,
  emptyAppLockState,
  enrollPasskeyLock,
  enrollPinLock,
  isSessionUnlocked,
  isWebAuthnSupported,
  loadAppLockState,
  lockSessionNow,
  markSessionUnlocked,
  saveAppLockState,
  unlockWithPasskey,
  unlockWithPin,
  type AppLockState,
} from '../lib/app-lock';
import { forgetDevice } from '../lib/remote-device';

const APP_LOCK_EVENT = 'gnomcode-app-lock-change';

function notifyLockChanged() {
  if (typeof window !== 'undefined') window.dispatchEvent(new Event(APP_LOCK_EVENT));
}

function formatCooldown(ms: number) {
  const secs = Math.max(1, Math.ceil(ms / 1000));
  const mins = Math.floor(secs / 60);
  const rem = secs % 60;
  return mins > 0 ? `${mins}m ${rem}s` : `${rem}s`;
}

export default function AppLockGate({ children }: { children: ReactNode }) {
  const [lockState, setLockState] = useState<AppLockState>(emptyAppLockState);
  const [unlocked, setUnlocked] = useState(true);
  const [pinInput, setPinInput] = useState('');
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState('');
  const [confirmReset, setConfirmReset] = useState(false);

  const syncFromStorage = () => {
    const next = loadAppLockState();
    setLockState(next);
    setUnlocked(isSessionUnlocked(next));
  };

  useEffect(() => {
    syncFromStorage();
    const onSync = () => syncFromStorage();
    window.addEventListener(APP_LOCK_EVENT, onSync);
    window.addEventListener('storage', onSync);
    const onVisibility = () => {
      if (document.visibilityState === 'visible') syncFromStorage();
      else if (loadAppLockState().mode !== 'off' && isSessionUnlocked(loadAppLockState())) {
        markSessionUnlocked();
      }
    };
    document.addEventListener('visibilitychange', onVisibility);
    return () => {
      window.removeEventListener(APP_LOCK_EVENT, onSync);
      window.removeEventListener('storage', onSync);
      document.removeEventListener('visibilitychange', onVisibility);
    };
  }, []);

  useEffect(() => {
    if (unlocked || lockState.mode === 'off') return;
    const onActivity = () => {
      if (isSessionUnlocked(loadAppLockState())) markSessionUnlocked();
    };
    void onActivity;
  }, [unlocked, lockState.mode]);

  if (lockState.mode === 'off' || unlocked) {
    return <>{children}</>;
  }

  async function handlePinSubmit(event: React.FormEvent) {
    event.preventDefault();
    if (busy) return;
    setBusy(true);
    setMessage('');
    try {
      const res = await unlockWithPin(lockState, pinInput.trim());
      saveAppLockState(res.nextState);
      setLockState(res.nextState);
      if (res.unlocked) {
        markSessionUnlocked();
        setPinInput('');
        setUnlocked(true);
        notifyLockChanged();
      } else if (res.lockedOut) {
        setMessage(`Too many wrong attempts. Try again in ${formatCooldown(res.remainingMs)}.`);
      } else {
        setMessage('Incorrect PIN. Check digits and try again.');
      }
    } catch (err) {
      setMessage(err instanceof Error ? err.message : 'Could not verify PIN.');
    } finally {
      setBusy(false);
    }
  }

  async function handlePasskeyUnlock() {
    if (busy) return;
    setBusy(true);
    setMessage('');
    try {
      const res = await unlockWithPasskey(lockState);
      saveAppLockState(res.nextState);
      setLockState(res.nextState);
      if (res.unlocked) {
        markSessionUnlocked();
        setUnlocked(true);
        notifyLockChanged();
      } else if (res.lockedOut) {
        setMessage(`Too many wrong attempts. Try again in ${formatCooldown(res.remainingMs)}.`);
      } else {
        setMessage('Passkey could not be verified.');
      }
    } catch (err) {
      setMessage(err instanceof Error ? err.message : 'Passkey unlock cancelled.');
    } finally {
      setBusy(false);
    }
  }

  async function handleResetLockAndDevice() {
    setBusy(true);
    try {
      await forgetDevice('', undefined).catch(() => {});
      clearAppLockState();
      setLockState(emptyAppLockState());
      setUnlocked(true);
      setConfirmReset(false);
      notifyLockChanged();
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="sp-lock-screen" role="dialog" aria-modal="true" aria-labelledby="sp-lock-heading">
      <div className="sp-lock-card">
        <span className="sp-lock-badge">[LOCKED]</span>
        <h1 id="sp-lock-heading">Unlock Gnomcode</h1>
        <p className="sp-lock-sub">
          {lockState.mode === 'passkey'
            ? 'Verify with your device passkey to view paired conversations.'
            : 'Enter your 4 to 8 digit device PIN to view paired conversations.'}
        </p>
        {message && <p className="sp-lock-status" role="alert">{message}</p>}
        {lockState.mode === 'passkey' ? (
          <button type="button" className="sp-settings-action sp-settings-primary sp-lock-submit" disabled={busy} onClick={() => void handlePasskeyUnlock()}>
            {busy ? 'Checking passkey...' : 'Unlock with passkey'}
          </button>
        ) : (
          <form className="sp-lock-form" onSubmit={event => void handlePinSubmit(event)}>
            <label htmlFor="sp-unlock-pin">Device PIN</label>
            <input
              id="sp-unlock-pin"
              type="password"
              inputMode="numeric"
              pattern="[0-9]{4,8}"
              maxLength={8}
              autoComplete="off"
              value={pinInput}
              onChange={event => setPinInput(event.target.value.replace(/\D/g, '').slice(0, 8))}
              required
            />
            <button type="submit" className="sp-settings-action sp-settings-primary sp-lock-submit" disabled={busy || pinInput.length < 4}>
              {busy ? 'Verifying...' : 'Unlock'}
            </button>
          </form>
        )}
        <div className="sp-lock-reset">
          {!confirmReset ? (
            <button type="button" className="sp-link" onClick={() => setConfirmReset(true)}>
              Forgot PIN or passkey?
            </button>
          ) : (
            <div className="sp-lock-reset-confirm">
              <p>Clearing app lock also forgets remembered paired desktops on this browser so you must re-pair.</p>
              <div className="sp-lock-reset-actions">
                <button type="button" className="sp-settings-action" onClick={() => setConfirmReset(false)}>Cancel</button>
                <button type="button" className="sp-settings-action" disabled={busy} onClick={() => void handleResetLockAndDevice()}>Reset &amp; forget device</button>
              </div>
            </div>
          )}
        </div>
      </div>
    </section>
  );
}

export function AppLockSettings() {
  const [lockState, setLockState] = useState<AppLockState>(emptyAppLockState);
  const [pinDraft, setPinDraft] = useState('');
  const [pinConfirm, setPinConfirm] = useState('');
  const [idleMinutes, setIdleMinutes] = useState(5);
  const [webAuthnOk, setWebAuthnOk] = useState(false);
  const [status, setStatus] = useState('');
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    const sync = () => {
      const next = loadAppLockState();
      setLockState(next);
      setIdleMinutes(next.idleMinutes ?? 5);
    };
    sync();
    setWebAuthnOk(isWebAuthnSupported());
    window.addEventListener(APP_LOCK_EVENT, sync);
    return () => window.removeEventListener(APP_LOCK_EVENT, sync);
  }, []);

  async function enablePin(event: React.FormEvent) {
    event.preventDefault();
    if (pinDraft !== pinConfirm) {
      setStatus('PIN entries do not match.');
      return;
    }
    setBusy(true);
    setStatus('');
    try {
      const next = await enrollPinLock(pinDraft, { unlocked: isSessionUnlocked(lockState), idleMinutes });
      saveAppLockState(next);
      markSessionUnlocked();
      setLockState(next);
      setPinDraft('');
      setPinConfirm('');
      setStatus('Device PIN lock enabled on this browser.');
      notifyLockChanged();
    } catch (err) {
      setStatus(err instanceof Error ? err.message : 'Could not enable PIN lock.');
    } finally {
      setBusy(false);
    }
  }

  async function enablePasskey() {
    setBusy(true);
    setStatus('');
    try {
      const next = await enrollPasskeyLock({ unlocked: isSessionUnlocked(lockState), idleMinutes });
      saveAppLockState(next);
      markSessionUnlocked();
      setLockState(next);
      setStatus('Passkey lock enabled on this browser.');
      notifyLockChanged();
    } catch (err) {
      setStatus(err instanceof Error ? err.message : 'Could not register passkey.');
    } finally {
      setBusy(false);
    }
  }

  function disableLock() {
    clearAppLockState();
    setLockState(emptyAppLockState());
    setStatus('App lock disabled on this browser.');
    notifyLockChanged();
  }

  function lockNow() {
    lockSessionNow();
    notifyLockChanged();
  }

  return (
    <div className="sp-settings-applock">
      <h3>PWA App Lock</h3>
      <p className="sp-settings-hint">
        Optional local lock for this browser only. Paired end-to-end encryption keys remain untouched.
        Status: <strong>{lockState.mode === 'passkey' ? 'Passkey' : lockState.mode === 'pin' ? 'Device PIN' : 'Off'}</strong>
      </p>
      {status && <p className="sp-settings-message" role="status">{status}</p>}
      <div className="sp-applock-controls">
        <label htmlFor="sp-lock-idle">Auto-lock timeout</label>
        <select
          id="sp-lock-idle"
          value={idleMinutes}
          onChange={event => {
            const mins = Number(event.target.value);
            setIdleMinutes(mins);
            if (lockState.mode !== 'off') {
              const updated = { ...lockState, idleMinutes: mins };
              saveAppLockState(updated);
              setLockState(updated);
            }
          }}
        >
          <option value={0}>On browser reopen only</option>
          <option value={1}>After 1 minute</option>
          <option value={5}>After 5 minutes</option>
          <option value={15}>After 15 minutes</option>
        </select>
      </div>
      <form className="sp-applock-pin-form" onSubmit={event => void enablePin(event)}>
        <div>
          <label htmlFor="sp-lock-pin-new">New device PIN (4-8 digits)</label>
          <input id="sp-lock-pin-new" type="password" inputMode="numeric" pattern="[0-9]{4,8}" maxLength={8} autoComplete="new-password" value={pinDraft} onChange={event => setPinDraft(event.target.value.replace(/\D/g, '').slice(0, 8))} />
        </div>
        <div>
          <label htmlFor="sp-lock-pin-confirm">Confirm PIN</label>
          <input id="sp-lock-pin-confirm" type="password" inputMode="numeric" pattern="[0-9]{4,8}" maxLength={8} autoComplete="new-password" value={pinConfirm} onChange={event => setPinConfirm(event.target.value.replace(/\D/g, '').slice(0, 8))} />
        </div>
        <button type="submit" className="sp-settings-action sp-settings-primary" disabled={busy || pinDraft.length < 4}>
          {lockState.mode === 'pin' ? 'Update PIN' : 'Enable Device PIN'}
        </button>
      </form>
      <div className="sp-applock-actions">
        {webAuthnOk && (
          <button type="button" className="sp-settings-action" disabled={busy} onClick={() => void enablePasskey()}>
            {lockState.mode === 'passkey' ? 'Re-register Passkey' : 'Enable Passkey Lock'}
          </button>
        )}
        {lockState.mode !== 'off' && (
          <>
            <button type="button" className="sp-settings-action" onClick={lockNow}>Lock now</button>
            <button type="button" className="sp-settings-action" onClick={disableLock}>Turn off app lock</button>
          </>
        )}
      </div>
    </div>
  );
}
