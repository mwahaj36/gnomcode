import { useCallback, useEffect, useState } from 'react';
import { fetchLocalJson, fetchLocalMutation } from './local-api';

interface SetupStatus {
  limit_message?: string | null;
  blocker?: 'trust_folder' | 'setup' | null;
  agent?: string;
  folder?: string;
  /** True while a protocol-mode agent works on a turn; null when only its transcript can say. */
  busy?: boolean | null;
}

const POLL_MS = 2000;

/**
 * Tells the person when the launched agent is stopped on a setup screen instead of leaving
 * prompts silently waiting. The daemon reads the agent's screen and owns the rules: folder
 * trust can be confirmed here with one explicit click or a remembered project choice, and any other
 * screen (sign-in, key, update) is only pointed to in the agent terminal.
 */
export function SetupPrompt({
  sessionId,
  onOpenTerminal,
  onStatus,
}: {
  sessionId: string;
  onOpenTerminal: () => void;
  /** Reports what the host said, so the workspace can show a running turn it owns. */
  onStatus?: (status: SetupStatus) => void;
}) {
  const [status, setStatus] = useState<SetupStatus>({});
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState(false);

  const load = useCallback(async () => {
    try {
      const next = await fetchLocalJson<SetupStatus>(`/api/v1/sessions/${encodeURIComponent(sessionId)}/setup`);
      setStatus(next);
      onStatus?.(next);
    } catch {
      // Unknown is never shown as a question; the next poll shows the real state.
      setStatus({});
      onStatus?.({});
    }
  }, [sessionId, onStatus]);

  useEffect(() => {
    setStatus({});
    setFailed(false);
    void load();
    const timer = window.setInterval(() => {
      if (document.visibilityState === 'visible') void load();
    }, POLL_MS);
    return () => window.clearInterval(timer);
  }, [load]);

  const trust = async (remember = false) => {
    setBusy(true);
    setFailed(false);
    try {
      if (remember) {
        await fetchLocalMutation(`/api/v1/sessions/${encodeURIComponent(sessionId)}/trust-project`, { method: 'POST' });
      } else {
        await fetchLocalMutation(`/api/v1/sessions/${encodeURIComponent(sessionId)}/trust-folder`, { method: 'POST' });
      }
    } catch {
      setFailed(true);
    }
    setBusy(false);
    await load();
  };

  if (status.limit_message) return <div className="sp-limit-reached" role="alert"><strong>Limit reached</strong><p>{status.limit_message.replace(/^Limit reached\.\s*/, '')}</p><button type="button" onClick={onOpenTerminal}>Open agent terminal</button></div>;
  if (!status.blocker) return null;
  const agent = status.agent || 'The agent';
  const isTrust = status.blocker === 'trust_folder';
  return (
    <section className="sp-approvals" aria-label="Agent setup needed">
      <article className="sp-approval" data-setup-blocker={status.blocker}>
        <div className="sp-approval-body">
          <p className="sp-approval-title">
            <span className="sp-session-dot sp-session-dot--needs_you" aria-hidden="true" />
            {isTrust ? `${agent} asks whether to trust this folder` : `${agent} is waiting on a setup step`}
          </p>
          {isTrust && status.folder ? <code className="sp-approval-summary">{status.folder}</code> : null}
          <p className="sp-approval-meta" role="status">
            {failed
              ? 'Could not confirm from here. Open the agent terminal to answer.'
              : isTrust
                ? 'Your messages wait until you answer. Trust only folders whose contents you know.'
                : 'Sign-in, update and similar screens are answered in the agent terminal. Your messages wait until then.'}
          </p>
        </div>
        <div className="sp-approval-actions">
          <button type="button" className="btn-control" onClick={onOpenTerminal}>
            Open agent terminal
          </button>
          {isTrust ? (
            <>
            <button type="button" className="btn-control" disabled={busy} onClick={() => void trust()}>
              Trust folder
            </button>
            <button type="button" className="btn-control" disabled={busy} title="Remember folder and tool approvals for future sessions in this exact project" onClick={() => void trust(true)}>
              Trust this project
            </button>
            </>
          ) : null}
        </div>
      </article>
    </section>
  );
}
