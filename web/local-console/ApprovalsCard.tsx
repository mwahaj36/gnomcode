import { useCallback, useEffect, useState } from 'react';
import { fetchLocalJson, fetchLocalMutation } from './local-api';

interface Approval {
  id: string;
  agent: string;
  tool: string;
  summary?: string;
}

const POLL_MS = 2000;

/**
 * Tool approvals held by a Claude Code hook (ADR 0032). The daemon owns the rules; this lists the
 * pending requests and sends the person's choice for one exact id. Nothing is ever inferred or
 * auto-approved, and every field is rendered as text.
 */
export function ApprovalsCard() {
  const [items, setItems] = useState<Approval[]>([]);
  const [deciding, setDeciding] = useState<string[]>([]);

  const load = useCallback(async () => {
    try {
      const body = await fetchLocalJson<{ items?: Approval[] }>('/api/v1/approvals');
      setItems(body.items ?? []);
    } catch {
      // Keep what is on screen; the next poll shows the real state.
    }
  }, []);

  useEffect(() => {
    void load();
    const timer = window.setInterval(() => void load(), POLL_MS);
    return () => window.clearInterval(timer);
  }, [load]);

  const decide = async (id: string, decision: 'allow' | 'deny') => {
    setDeciding((current) => [...current, id]);
    try {
      await fetchLocalMutation(`/api/v1/approvals/${encodeURIComponent(id)}/decision`, {
        method: 'POST',
        body: { decision },
      });
    } catch {
      // The list shows the real state.
    }
    setDeciding((current) => current.filter((value) => value !== id));
    await load();
  };

  return (
    <section id="approvals" className="sp-approvals" aria-label="Waiting for your approval" hidden={items.length === 0}>
      {items.map((item) => {
        const agent = item.agent === 'claude-code' ? 'Claude Code' : item.agent;
        const busy = deciding.includes(item.id);
        return (
          <article className="sp-approval" data-approval-id={item.id} key={item.id}>
            <div className="sp-approval-body">
              <p className="sp-approval-title">
                <span className="sp-session-dot sp-session-dot--needs_you" aria-hidden="true" />
                {agent} wants to run <strong>{item.tool}</strong>
              </p>
              {item.summary ? <code className="sp-approval-summary">{item.summary}</code> : null}
              <p className="sp-approval-meta">If you do not answer, Claude asks in its own window.</p>
            </div>
            <div className="sp-approval-actions">
              <button
                type="button"
                className="btn-control"
                disabled={busy}
                aria-label={`Deny ${item.tool}`}
                onClick={() => void decide(item.id, 'deny')}
              >
                Deny
              </button>
              <button
                type="button"
                className="btn-control"
                disabled={busy}
                aria-label={`Approve ${item.tool} once`}
                onClick={() => void decide(item.id, 'allow')}
              >
                Approve once
              </button>
            </div>
          </article>
        );
      })}
    </section>
  );
}
