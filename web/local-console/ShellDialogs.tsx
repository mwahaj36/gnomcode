import { Fragment, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { fetchLocalJson } from './local-api';
import { navigateTo, registerShellDialogs, workspaceAction } from './shell-bus';

const TOUR_FLAG = 'gnomcode-onboarding-v1';

function storedFlag(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function storeFlag(key: string, value: string): void {
  try {
    localStorage.setItem(key, value);
  } catch {
    // Storage unavailable
  }
}

const TOUR_STEPS: { title: string; intro: string; items: [string, string][]; note: string }[] = [
  {
    title: 'Four places, one local console',
    intro: 'Switch views from the left rail anytime.',
    items: [
      ['Home', "Check live limits, active sessions, and today's totals."],
      ['Workspace', 'Browse conversations in the list and chat beside terminals.'],
      ['Usage', 'Inspect activity heatmaps, project spend, and CSV exports.'],
      ['Settings', 'Configure provider limits, Remote pairing, alerts, and preferences.'],
    ],
    note: 'Local monitoring needs no account or telemetry.',
  },
  {
    title: 'Start work in Workspace',
    intro: 'Workspace opens list-first with your recent conversation.',
    items: [
      ['Open the launcher', 'Select New in Conversations to pick an agent and folder.'],
      ['Keep history handy', 'Search or filter sessions, or collapse the sidebar anytime.'],
      ['Stay in control', 'Open the inline terminal whenever a CLI asks for trust.'],
    ],
    note: 'Gnomcode never skips native agent permission prompts.',
  },
  {
    title: 'Track live provider limits',
    intro: 'Provider cards on Home and Usage show remaining quota.',
    items: [
      ['Enable Claude and Antigravity limits', 'Open Settings, choose Providers, and turn on live polling.'],
      ['Check pace and resets', 'Each card lists reset times and exact or estimated tags.'],
      ['Watch spend and exports', 'Open Usage for daily heatmaps and per-project totals.'],
    ],
    note: 'Estimated spend is a local estimate, not your provider bill.',
  },
  {
    title: 'Desktop tray and Remote access',
    intro: 'Stay informed away from the main window.',
    items: [
      ['Right-click the tray icon', 'View tightest limits, waiting sessions, and quick restart actions.'],
      ['Pair your phone in Settings', 'Open Devices and Remote to link encrypted mobile access.'],
      ['Replay this guide anytime', 'Select Getting started under Settings whenever you need it.'],
    ],
    note: 'Nothing leaves this machine until you connect an account.',
  },
];

interface ConfirmRequest {
  message: string;
  label: string;
  resolve: (accepted: boolean) => void;
}

/** App notice, first-run tip, the four-step tour and the confirmation dialog. */
export function ShellDialogs() {
  const [notice, setNotice] = useState<string | null>(null);
  const [tipOpen, setTipOpen] = useState(false);
  const [step, setStep] = useState(0);
  const [confirmRequest, setConfirmRequest] = useState<ConfirmRequest | null>(null);
  const pendingConfirm = useRef<ConfirmRequest | null>(null);
  const tourRef = useRef<HTMLDialogElement | null>(null);
  const confirmRef = useRef<HTMLDialogElement | null>(null);
  const stepRefs = useRef<(HTMLHeadingElement | null)[]>([]);
  const returnFocus = useRef<Element | null>(null);

  const startTour = useCallback(() => {
    const dialog = tourRef.current;
    setTipOpen(false);
    if (!dialog || dialog.open) return;
    returnFocus.current = document.activeElement;
    setStep(0);
    dialog.showModal();
    stepRefs.current[0]?.focus();
  }, []);

  const finishTour = useCallback((launch: boolean) => {
    storeFlag(TOUR_FLAG, 'dismissed');
    setTipOpen(false);
    tourRef.current?.close();
    if (launch) {
      navigateTo('sessions');
      workspaceAction({ type: 'new-session', focus: true });
    } else if (returnFocus.current instanceof HTMLElement) {
      returnFocus.current.focus();
    }
  }, []);

  const moveTour = (direction: number) => {
    if (step + direction >= TOUR_STEPS.length) {
      finishTour(true);
      return;
    }
    const next = Math.max(0, step + direction);
    setStep(next);
    stepRefs.current[next]?.focus();
  };

  const confirm = useCallback((message: string, label: string) => {
    if (confirmRef.current?.open) return Promise.resolve(false);
    return new Promise<boolean>((resolve) => {
      const request = { message, label, resolve };
      pendingConfirm.current = request;
      setConfirmRequest(request);
    });
  }, []);

  useEffect(() => {
    const dialog = confirmRef.current;
    if (!confirmRequest || !dialog || dialog.open) return;
    dialog.returnValue = 'cancel';
    dialog.showModal();
  }, [confirmRequest]);

  const controller = useMemo(() => ({ confirm, notice: setNotice, startTour }), [confirm, startTour]);
  useEffect(() => {
    const unregister = registerShellDialogs(controller);
    return () => {
      unregister();
    };
  }, [confirm, controller]);

  // An empty history opens the tour by itself; otherwise a dismissible tip offers it once.
  useEffect(() => {
    if (storedFlag(TOUR_FLAG)) return;
    void (async () => {
      try {
        const res = await fetchLocalJson<{ total_count?: number; sessions?: unknown[] }>(
          '/api/v1/sessions?page=1&limit=1',
        );
        const count =
          typeof res?.total_count === 'number' ? res.total_count : Array.isArray(res?.sessions) ? res.sessions.length : null;
        if (storedFlag(TOUR_FLAG) || count === null) return;
        if (count === 0) startTour();
        else setTipOpen(true);
      } catch {
        // No offer when the daemon cannot say
      }
    })();
  }, [startTour]);

  return (
    <>
      <div id="app-notice" className="app-notice" role="status" hidden={notice === null}>
        <p id="app-notice-message">{notice}</p>
        <button type="button" className="btn-control" onClick={() => setNotice(null)}>
          Dismiss
        </button>
      </div>
      <div id="tour-tip" className="app-notice" role="region" aria-label="Workspace tour tip" hidden={!tipOpen}>
        <p>See Home, Workspace, Usage, and Settings in four quick steps.</p>
        <div className="dialog-actions">
          <button type="button" className="btn-primary" onClick={startTour}>
            Open guide
          </button>
          <button
            type="button"
            className="btn-control"
            onClick={() => {
              storeFlag(TOUR_FLAG, 'dismissed');
              setTipOpen(false);
            }}
          >
            Dismiss
          </button>
        </div>
      </div>
      <dialog
        id="onboarding-dialog"
        ref={tourRef}
        aria-labelledby="onboarding-title"
        aria-describedby="onboarding-progress"
        onCancel={(event) => {
          event.preventDefault();
          finishTour(false);
        }}
      >
        <div className="onboarding-header">
          <strong id="onboarding-title">Gnomcode tour</strong>
          <button type="button" className="btn-control" onClick={() => finishTour(false)}>
            Skip guide
          </button>
        </div>
        <p id="onboarding-progress" role="status" aria-live="polite">
          {`Step ${step + 1} of ${TOUR_STEPS.length}`}
        </p>
        <div className="onboarding-body">
          {TOUR_STEPS.map((page, index) => (
            <section data-onboarding-step="" hidden={index !== step} key={page.title}>
              <h2
                tabIndex={-1}
                ref={(node) => {
                  stepRefs.current[index] = node;
                }}
              >
                {page.title}
              </h2>
              <p>{page.intro}</p>
              <dl>
                {page.items.map(([term, description]) => (
                  <Fragment key={term}>
                    <dt>{term}</dt>
                    <dd>{description}</dd>
                  </Fragment>
                ))}
              </dl>
              <p className="onboarding-note">{page.note}</p>
            </section>
          ))}
        </div>
        <div className="onboarding-footer">
          <button id="onboarding-back" type="button" className="btn-control" disabled={step === 0} onClick={() => moveTour(-1)}>
            Back
          </button>
          <button id="onboarding-next" type="button" className="btn-primary" onClick={() => moveTour(1)}>
            {step === TOUR_STEPS.length - 1 ? 'Open session launcher' : 'Continue'}
          </button>
        </div>
      </dialog>
      <dialog
        id="confirm-dialog"
        ref={confirmRef}
        aria-labelledby="confirm-heading"
        aria-describedby="confirm-message"
        onClose={(event) => {
          const request = pendingConfirm.current;
          pendingConfirm.current = null;
          setConfirmRequest(null);
          request?.resolve(event.currentTarget.returnValue === 'confirm');
        }}
      >
        <form method="dialog">
          <h2 id="confirm-heading">{confirmRequest?.label ?? 'Confirm action'}</h2>
          <p id="confirm-message">{confirmRequest?.message}</p>
          <div className="dialog-actions">
            <button className="btn-control" value="cancel" autoFocus>
              Cancel
            </button>
            <button className="btn-primary" id="confirm-accept" value="confirm">
              {confirmRequest?.label ?? 'Continue'}
            </button>
          </div>
        </form>
      </dialog>
    </>
  );
}
