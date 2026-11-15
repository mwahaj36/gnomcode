import { useCallback, useEffect, useState } from 'react';
import { approvalCopy } from '../lib/console-presentation.js';
import { fetchLocalJson, fetchLocalMutation } from './local-api';

interface Approval {
  id: string;
  agent: string;
  tool: string;
  summary?: string;
}

/** A standing allow the person gave one session. Held in daemon memory only. */
interface Grant {
  agent: string;
  session_id: string;
  tool: string | null;
}

type Scope = 'once' | 'tool' | 'session';

const POLL_MS = 2000;
/** The daemon never lets a standing grant answer a request to widen the agent's own permissions. */
const ESCALATION_TOOL = 'Permissions';

/**
 * Tool approvals held for Claude Code (hook, ADR 0032) and for Codex and Antigravity protocol
 * sessions (ADR 0057). The daemon owns the rules; this lists the pending requests and sends the
 * person's choice for one exact id. Nothing is ever inferred or auto-approved here: a wider
 * allow is an explicit button, lasts for that session only, and stays listed with a Revoke.
 */
export function ApprovalsCard() {
  const [items, setItems] = useState<Approval[]>([]);
  const [grants, setGrants] = useState<Grant[]>([]);
  const [deciding, setDeciding] = useState<string[]>([]);

  const load = useCallback(async () => {
    try {
      const body = await fetchLocalJson<{ items?: Approval[]; grants?: Grant[] }>('/api/v1/approvals');
      setItems(body.items ?? []);
      setGrants(body.grants ?? []);
    } catch {
      // Keep what is on screen; the next poll shows the real state.
    }
  }, []);

  useEffect(() => {
    void load();
    const timer = window.setInterval(() => void load(), POLL_MS);
    return () => window.clearInterval(timer);
  }, [load]);

  const decide = async (id: string, decision: 'allow' | 'deny', scope: Scope = 'once') => {
    setDeciding((current) => [...current, id]);
    try {
      await fetchLocalMutation(`/api/v1/approvals/${encodeURIComponent(id)}/decision`, {
        method: 'POST',
        body: scope === 'once' ? { decision } : { decision, scope },
      });
    } catch {
      // The list shows the real state.
    }
    setDeciding((current) => current.filter((value) => value !== id));
    await load();
  };

  const revoke = async (sessionId: string) => {
    try {
      await fetchLocalMutation(`/api/v1/approvals/grants/${encodeURIComponent(sessionId)}`, { method: 'DELETE' });
    } catch {
      // The list shows the real state.
    }
    await load();
  };

  // One row per session: the daemon keeps either its per-tool grants or one for every tool.
  const sessions = [...new Set(grants.map((grant) => grant.session_id))];

  return (
    <section
      id="approvals"
      className="sp-approvals"
      aria-label="Waiting for your approval"
      hidden={items.length === 0 && grants.length === 0}
    >
      {items.map((item) => {
        const copy = approvalCopy(item.agent);
        const busy = deciding.includes(item.id);
        const canWiden = item.tool !== ESCALATION_TOOL;
        return (
          <article className="sp-approval" data-approval-id={item.id} key={item.id}>
            <div className="sp-approval-body">
              <p className="sp-approval-title">
                <span className="sp-session-dot sp-session-dot--needs_you" aria-hidden="true" />
                {copy.agent} wants to run <strong>{item.tool}</strong>
              </p>
              {item.summary ? <code className="sp-approval-summary">{item.summary}</code> : null}
              <p className="sp-approval-meta">{copy.unanswered}</p>
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
              {canWiden ? (
                <>
                  <button
                    type="button"
                    className="btn-control"
                    disabled={busy}
                    aria-label={`Allow ${item.tool} for this session`}
                    onClick={() => void decide(item.id, 'allow', 'tool')}
                  >
                    Allow {item.tool} this session
                  </button>
                  <button
                    type="button"
                    className="btn-control"
                    disabled={busy}
                    aria-label="Allow all tools for this session"
                    onClick={() => void decide(item.id, 'allow', 'session')}
                  >
                    Allow all this session
                  </button>
                </>
              ) : null}
            </div>
          </article>
        );
      })}
      {sessions.map((sessionId) => {
        const held = grants.filter((grant) => grant.session_id === sessionId);
        const tools = held.some((grant) => grant.tool === null)
          ? 'all tools'
          : held.map((grant) => grant.tool).join(', ');
        const agent = approvalCopy(held[0]?.agent ?? '').agent;
        return (
          <article className="sp-approval sp-approval--grant" data-grant-session={sessionId} key={`grant-${sessionId}`}>
            <div className="sp-approval-body">
              <p className="sp-approval-title">
                {agent}: allowing <strong>{tools}</strong> for this session
              </p>
              <p className="sp-approval-meta">
                Ends when the session stops or Gnomcode restarts. Permission changes still ask every time.
              </p>
            </div>
            <div className="sp-approval-actions">
              <button
                type="button"
                className="btn-control"
                aria-label={`Stop allowing ${tools} for this ${agent} session`}
                onClick={() => void revoke(sessionId)}
              >
                Revoke
              </button>
            </div>
          </article>
        );
      })}
    </section>
  );
}
