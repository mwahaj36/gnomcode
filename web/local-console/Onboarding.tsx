import { useCallback, useEffect, useRef, useState } from 'react';
import { fetchLocalJson } from './local-api';
import { navigateTo, requestProviderSetup } from './shell-bus';
import { ProviderChecklist, SETUP_PROVIDERS, statusUrl, type ProviderSetupStatus } from './ProviderSetup';

export const ONBOARDING_FLAG = 'gnomcode-onboarding-v2';
const LEGACY_FLAG = 'gnomcode-onboarding-v1';

type Stage = 'welcome' | 'account' | 'provider' | 'phone' | 'done';

/** Resume state kept in local storage; onboarding never blocks and can be reopened. */
export interface OnboardingState {
  stage: Stage;
  selected: string[];
  index: number;
  distros: Record<string, string>;
  finished?: boolean;
}

const FRESH: OnboardingState = { stage: 'welcome', selected: [], index: 0, distros: {} };

export function readOnboarding(): OnboardingState | null {
  try {
    const raw = localStorage.getItem(ONBOARDING_FLAG);
    if (!raw) return localStorage.getItem(LEGACY_FLAG) ? { ...FRESH, finished: true } : null;
    const parsed = JSON.parse(raw) as Partial<OnboardingState>;
    return {
      ...FRESH,
      ...parsed,
      selected: Array.isArray(parsed.selected) ? parsed.selected.filter((p) => SETUP_PROVIDERS.some(([id]) => id === p)) : [],
      distros: parsed.distros && typeof parsed.distros === 'object' ? parsed.distros : {},
    };
  } catch {
    return null;
  }
}

function saveOnboarding(state: OnboardingState): void {
  try {
    localStorage.setItem(ONBOARDING_FLAG, JSON.stringify(state));
  } catch {
    // Storage unavailable: onboarding still works for this visit
  }
}

const STAGES: [Stage, string][] = [
  ['welcome', 'Providers'],
  ['account', 'Account'],
  ['provider', 'Set up'],
  ['phone', 'Phone'],
  ['done', 'Done'],
];

function Progress({ stage, detail }: { stage: Stage; detail: string }) {
  const at = STAGES.findIndex(([id]) => id === stage);
  return (
    <div className="sp-onb-progress">
      <ol aria-label="Onboarding progress">
        {STAGES.map(([id, label], index) => (
          <li key={id} className={index < at ? 'is-done' : index === at ? 'is-current' : ''} aria-current={index === at ? 'step' : undefined}>
            {label}
          </li>
        ))}
      </ol>
      <p id="onboarding-progress" role="status" aria-live="polite">
        {detail}
      </p>
    </div>
  );
}

function PhoneStep({ active }: { active: boolean }) {
  const [paired, setPaired] = useState<number | null>(null);
  const [permission, setPermission] = useState<string>(() =>
    typeof Notification === 'undefined' ? 'unsupported' : Notification.permission,
  );
  useEffect(() => {
    if (!active) return;
    let stopped = false;
    let timer: number | undefined;
    const tick = async () => {
      if (document.visibilityState === 'visible') {
        try {
          const res = await fetchLocalJson<{ devices?: unknown[] }>('/api/v1/remote/devices', 8000);
          if (!stopped) setPaired(Array.isArray(res.devices) ? res.devices.length : 0);
        } catch {
          if (!stopped) setPaired(null);
        }
      }
      if (!stopped) timer = window.setTimeout(tick, 2000);
    };
    void tick();
    return () => {
      stopped = true;
      window.clearTimeout(timer);
    };
  }, [active]);
  return (
    <ol className="sp-setup-steps">
      <li className={`sp-setup-step is-${paired ? 'done' : 'todo'}`} data-step="phone" data-state={paired ? 'done' : 'todo'}>
        <span className="sp-setup-mark" aria-hidden="true">
          {paired ? (
            <svg viewBox="0 0 16 16" width="16" height="16">
              <path className="sp-tick" d="M3.5 8.5l3 3 6-7" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
          ) : null}
        </span>
        <div className="sp-setup-body">
          <p className="sp-setup-label">Pair your phone</p>
          <p className="sp-setup-note">
            {paired ? `${paired} device${paired === 1 ? '' : 's'} paired` : 'Scan the Remote QR code in Settings > Devices.'}
          </p>
          {!paired ? (
            <>
              <button type="button" className="btn-control" onClick={() => navigateTo('settings')}>
                Open Devices and Remote
              </button>
              <p className="sp-setup-wait" role="status" aria-live="polite">
                <span className="sp-pulse" aria-hidden="true" />
                Waiting for a paired device...
              </p>
            </>
          ) : null}
        </div>
      </li>
      <li className={`sp-setup-step is-${permission === 'granted' ? 'done' : 'todo'}`} data-step="notifications">
        <span className="sp-setup-mark" aria-hidden="true">
          {permission === 'granted' ? (
            <svg viewBox="0 0 16 16" width="16" height="16">
              <path className="sp-tick" d="M3.5 8.5l3 3 6-7" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
          ) : null}
        </span>
        <div className="sp-setup-body">
          <p className="sp-setup-label">Notifications</p>
          <p className="sp-setup-note">
            {permission === 'granted'
              ? 'Allowed'
              : permission === 'unsupported'
                ? 'Not supported in this browser'
                : permission === 'denied'
                  ? 'Blocked; change it in browser settings'
                  : 'Get alerts when an agent waits or a limit runs low.'}
          </p>
          {permission === 'default' ? (
            <button
              type="button"
              className="btn-control"
              onClick={async () => setPermission(await Notification.requestPermission())}
            >
              Allow notifications
            </button>
          ) : null}
        </div>
      </li>
    </ol>
  );
}

/**
 * Verified first-run onboarding: pick providers, run each provider's live checklist,
 * optionally pair a phone, then a summary. Every stage can be skipped; progress resumes.
 */
export function Onboarding({
  open,
  initial,
  onClose,
}: {
  open: boolean;
  initial: OnboardingState | null;
  onClose: () => void;
}) {
  const dialogRef = useRef<HTMLDialogElement | null>(null);
  const headingRef = useRef<HTMLHeadingElement | null>(null);
  const returnFocus = useRef<Element | null>(null);
  const [state, setState] = useState<OnboardingState>(initial && !initial.finished ? initial : FRESH);
  const [detected, setDetected] = useState<Record<string, ProviderSetupStatus>>({});
  const [ready, setReady] = useState<Record<string, boolean>>({});

  const update = useCallback((next: OnboardingState) => {
    setState(next);
    saveOnboarding(next);
  }, []);

  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;
    if (open && !dialog.open) {
      returnFocus.current = document.activeElement;
      const start = initial && !initial.finished ? initial : FRESH;
      setState(start);
      dialog.showModal();
      headingRef.current?.focus();
    } else if (!open && dialog.open) {
      dialog.close();
    }
  }, [open, initial]);

  // Detect installed providers once to pre-check them on the welcome step.
  useEffect(() => {
    if (!open) return;
    void (async () => {
      try {
        const res = await fetchLocalJson<{ providers?: ProviderSetupStatus[] }>(statusUrl(), 10000);
        const map: Record<string, ProviderSetupStatus> = {};
        for (const p of res.providers ?? []) map[p.provider] = p;
        setDetected(map);
        setState((prev) => {
          if (prev.stage !== 'welcome' || prev.selected.length) return prev;
          const picked = SETUP_PROVIDERS.map(([id]) => id).filter(
            (id) => map[id]?.steps.find((s) => s.id === 'installed')?.state === 'done',
          );
          return { ...prev, selected: picked };
        });
      } catch {
        // Fail open: nothing pre-checked
      }
    })();
  }, [open]);

  useEffect(() => {
    headingRef.current?.focus();
  }, [state.stage, state.index]);

  const close = (finished: boolean) => {
    const next = finished ? { ...state, stage: 'done' as Stage, finished: true } : state;
    saveOnboarding(finished ? next : { ...state, finished: state.finished ?? false });
    try {
      localStorage.setItem(LEGACY_FLAG, 'dismissed');
    } catch {
      // Storage unavailable
    }
    onClose();
    if (returnFocus.current instanceof HTMLElement) returnFocus.current.focus();
  };

  const provider = state.selected[state.index];
  const providerName = SETUP_PROVIDERS.find(([id]) => id === provider)?.[1] ?? '';
  const distro = provider ? state.distros[provider] || undefined : undefined;

  const advanceFromAccount = () =>
    update(state.selected.length ? { ...state, stage: 'provider', index: 0 } : { ...state, stage: 'phone' });
  const nextProvider = () => {
    if (state.index + 1 < state.selected.length) update({ ...state, index: state.index + 1 });
    else update({ ...state, stage: 'phone' });
  };
  const back = () => {
    if (state.stage === 'account') update({ ...state, stage: 'welcome' });
    else if (state.stage === 'provider' && state.index > 0) update({ ...state, index: state.index - 1 });
    else if (state.stage === 'provider') update({ ...state, stage: 'account' });
    else if (state.stage === 'phone') {
      update(state.selected.length ? { ...state, stage: 'provider', index: state.selected.length - 1 } : { ...state, stage: 'account' });
    } else if (state.stage === 'done') update({ ...state, stage: 'phone' });
  };

  const detail =
    state.stage === 'provider'
      ? `${providerName}: provider ${state.index + 1} of ${state.selected.length}`
      : state.stage === 'welcome'
        ? 'Choose providers to set up'
        : state.stage === 'account'
          ? 'Optional: sign in or skip for local-only use'
          : state.stage === 'phone'
            ? 'Optional: phone and notifications'
            : 'Setup summary';

  return (
    <dialog
      id="onboarding-dialog"
      className="sp-onb"
      ref={dialogRef}
      aria-labelledby="onboarding-heading"
      aria-describedby="onboarding-progress"
      onCancel={(event) => {
        event.preventDefault();
        close(false);
      }}
    >
      <div className="onboarding-header">
        <strong id="onboarding-title">Set up Gnomcode</strong>
        <button type="button" className="btn-control" onClick={() => close(false)}>
          Close
        </button>
      </div>
      <Progress stage={state.stage} detail={detail} />
      <div className="onboarding-body">
        {state.stage === 'welcome' ? (
          <section data-onboarding-step="welcome">
            <h2 id="onboarding-heading" tabIndex={-1} ref={headingRef}>
              Which agents do you use?
            </h2>
            <p>Installed agents are already checked. Gnomcode verifies each step live.</p>
            <fieldset className="sp-onb-pick">
              <legend className="sr-only">Providers</legend>
              {SETUP_PROVIDERS.map(([id, name]) => {
                const installed = detected[id]?.steps.find((s) => s.id === 'installed')?.state === 'done';
                return (
                  <label key={id} className="sp-onb-choice">
                    <input
                      type="checkbox"
                      checked={state.selected.includes(id)}
                      onChange={(event) =>
                        update({
                          ...state,
                          selected: event.target.checked
                            ? SETUP_PROVIDERS.map(([p]) => p).filter((p) => p === id || state.selected.includes(p))
                            : state.selected.filter((p) => p !== id),
                        })
                      }
                    />
                    <span className="sp-onb-choice-name">{name}</span>
                    <span className="sp-onb-choice-meta">{installed ? 'Detected' : 'Not detected'}</span>
                  </label>
                );
              })}
            </fieldset>
          </section>
        ) : null}
        {state.stage === 'account' ? (
          <section data-onboarding-step="account">
            <h2 id="onboarding-heading" tabIndex={-1} ref={headingRef}>
              Sign in (optional)
            </h2>
            <p>
              Local sessions, usage tracking, and workspace handoff work offline without an account. Sign in only if you want encrypted Remote pairing or cloud sync.
            </p>
            <div className="sp-onb-actions" style={{ display: 'flex', gap: '8px', flexWrap: 'wrap', marginTop: '12px' }}>
              <button
                type="button"
                className="btn-primary"
                data-onboarding-action="signin"
                onClick={() => {
                  close(false);
                  navigateTo('auth');
                }}
              >
                Sign in
              </button>
              <button
                type="button"
                className="btn-control"
                data-onboarding-action="skip-account"
                onClick={advanceFromAccount}
              >
                Skip for now
              </button>
            </div>
          </section>
        ) : null}
        {state.stage === 'provider' && provider ? (
          <section data-onboarding-step="provider">
            <h2 id="onboarding-heading" tabIndex={-1} ref={headingRef}>
              {`Set up ${providerName}`}
            </h2>
            <p>Run each command in your own terminal. Steps tick as Gnomcode sees them.</p>
            <label className="sp-onb-distro">
              <span>Run in</span>
              <input
                type="text"
                inputMode="text"
                placeholder="This computer"
                aria-describedby="onb-distro-help"
                value={state.distros[provider] ?? ''}
                onChange={(event) =>
                  update({ ...state, distros: { ...state.distros, [provider]: event.target.value.trim() } })
                }
              />
            </label>
            <p id="onb-distro-help" className="sp-setup-note">
              Windows only: type a WSL distro name (for example Ubuntu) to check inside it.
            </p>
            <ProviderChecklist
              key={`${provider}@${distro ?? ''}`}
              provider={provider}
              distro={distro}
              active={open}
              onStatus={(status) => setReady((prev) => (prev[provider] === status.ready ? prev : { ...prev, [provider]: status.ready }))}
            />
          </section>
        ) : null}
        {state.stage === 'phone' ? (
          <section data-onboarding-step="phone">
            <h2 id="onboarding-heading" tabIndex={-1} ref={headingRef}>
              Take it with you
            </h2>
            <p>Optional. Pair a phone for Remote and approvals.</p>
            <PhoneStep active={open} />
          </section>
        ) : null}
        {state.stage === 'done' ? (
          <section data-onboarding-step="done">
            <h2 id="onboarding-heading" tabIndex={-1} ref={headingRef}>
              You are set up
            </h2>
            <ul className="sp-onb-summary">
              {state.selected.length ? (
                state.selected.map((id) => (
                  <li key={id}>
                    <span>{SETUP_PROVIDERS.find(([p]) => p === id)?.[1]}</span>
                    <span className={`sp-badge ${ready[id] ? 'is-ready' : ''}`}>{ready[id] ? 'Ready' : 'Unfinished'}</span>
                  </li>
                ))
              ) : (
                <li>No providers chosen</li>
              )}
            </ul>
            <p>Re-run any step later from Settings, Providers. The doctor links there too.</p>
            <button
              type="button"
              className="btn-control"
              onClick={() => {
                close(true);
                requestProviderSetup();
                navigateTo('settings');
              }}
            >
              Open Settings, Providers
            </button>
          </section>
        ) : null}
      </div>
      <div className="onboarding-footer">
        <button id="onboarding-back" type="button" className="btn-control" disabled={state.stage === 'welcome'} onClick={back}>
          Back
        </button>
        {state.stage === 'provider' ? (
          <button type="button" className="btn-control" onClick={nextProvider}>
            Skip
          </button>
        ) : null}
        {state.stage === 'welcome' ? (
          <button
            id="onboarding-next"
            type="button"
            className="btn-primary"
            onClick={() => update({ ...state, stage: 'account' })}
          >
            Continue
          </button>
        ) : null}
        {state.stage === 'account' ? (
          <button
            id="onboarding-next"
            type="button"
            className="btn-control"
            onClick={advanceFromAccount}
          >
            Skip for now
          </button>
        ) : null}
        {state.stage === 'provider' ? (
          <button id="onboarding-next" type="button" className="btn-primary" onClick={nextProvider} disabled={!ready[provider ?? '']}>
            Next
          </button>
        ) : null}
        {state.stage === 'phone' ? (
          <button id="onboarding-next" type="button" className="btn-primary" onClick={() => update({ ...state, stage: 'done' })}>
            Continue
          </button>
        ) : null}
        {state.stage === 'done' ? (
          <button
            id="onboarding-next"
            type="button"
            className="btn-primary"
            onClick={() => close(true)}
          >
            Finish
          </button>
        ) : null}
      </div>
    </dialog>
  );
}
