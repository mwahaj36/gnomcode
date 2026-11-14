import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { fetchLocalJson } from './local-api';
import { registerShellDialogs } from './shell-bus';
import { Onboarding, readOnboarding, type OnboardingState } from './Onboarding';


interface ConfirmRequest {
  message: string;
  label: string;
  resolve: (accepted: boolean) => void;
}

/** App notice, first-run tip, verified onboarding (ADR 0061) and the confirmation dialog. */
export function ShellDialogs() {
  const [notice, setNotice] = useState<string | null>(null);
  const [tipOpen, setTipOpen] = useState(false);
  const [tourOpen, setTourOpen] = useState(false);
  const [resume, setResume] = useState<OnboardingState | null>(null);
  const [confirmRequest, setConfirmRequest] = useState<ConfirmRequest | null>(null);
  const pendingConfirm = useRef<ConfirmRequest | null>(null);
  const confirmRef = useRef<HTMLDialogElement | null>(null);

  const startTour = useCallback(() => {
    setTipOpen(false);
    setResume(readOnboarding());
    setTourOpen(true);
  }, []);

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

  // Unfinished onboarding resumes; an empty history opens it; otherwise a tip offers it once.
  useEffect(() => {
    const saved = readOnboarding();
    if (saved?.finished) return;
    if (saved) {
      setTipOpen(true);
      return;
    }
    void (async () => {
      try {
        const res = await fetchLocalJson<{ total_count?: number; sessions?: unknown[] }>(
          '/api/v1/sessions?page=1&limit=1',
        );
        const count =
          typeof res?.total_count === 'number' ? res.total_count : Array.isArray(res?.sessions) ? res.sessions.length : null;
        if (readOnboarding() || count === null) return;
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
      <div id="tour-tip" className="app-notice" role="region" aria-label="Setup tip" hidden={!tipOpen}>
        <p>Set up your agents with live checks.</p>
        <div className="dialog-actions">
          <button type="button" className="btn-primary" onClick={startTour}>
            Set up
          </button>
          <button
            type="button"
            className="btn-control"
            onClick={() => {
              try {
                const saved = readOnboarding();
                localStorage.setItem(
                  'gnomcode-onboarding-v2',
                  JSON.stringify({ ...(saved ?? { stage: 'welcome', selected: [], index: 0, distros: {} }), finished: true }),
                );
              } catch {
                // Storage unavailable
              }
              setTipOpen(false);
            }}
          >
            Dismiss
          </button>
        </div>
      </div>
      <Onboarding open={tourOpen} initial={resume} onClose={() => setTourOpen(false)} />
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
