import { useCallback, useEffect, useRef, useState } from 'react';
import { fetchLocalJson } from './local-api';
import { takeProviderSetupRequest } from './shell-bus';

/** One checklist step as reported by `GET /api/v1/onboarding/status` (detection lives in Rust). */
export interface SetupStep {
  id: string;
  label: string;
  state: 'done' | 'todo' | 'waiting' | 'unknown';
  optional: boolean;
  trust: 'exact' | 'estimated' | 'unknown';
  note: string;
  command?: string;
}

export interface ProviderSetupStatus {
  provider: string;
  name: string;
  distro?: string;
  version?: string;
  account?: string;
  ready: boolean;
  steps: SetupStep[];
}

export const SETUP_PROVIDERS: [string, string][] = [
  ['claude', 'Claude Code'],
  ['codex', 'Codex'],
  ['antigravity', 'Antigravity'],
];

const POLL_MS = 2000;

export function statusUrl(provider?: string, distro?: string): string {
  const params = new URLSearchParams();
  if (provider) params.set('provider', provider);
  if (distro) params.set('distro', distro);
  const query = params.toString();
  return `/api/v1/onboarding/status${query ? `?${query}` : ''}`;
}

/**
 * Polls one provider's status about every 2 s while `active` and the page is visible.
 * Stops when hidden or unmounted.
 */
export function useProviderStatus(provider: string, distro: string | undefined, active: boolean) {
  const [status, setStatus] = useState<ProviderSetupStatus | null>(null);
  const [failed, setFailed] = useState(false);
  const load = useCallback(async () => {
    try {
      const res = await fetchLocalJson<{ providers?: ProviderSetupStatus[] }>(statusUrl(provider, distro), 8000);
      const next = res.providers?.[0] ?? null;
      setStatus(next);
      setFailed(false);
    } catch {
      setFailed(true);
    }
  }, [provider, distro]);
  useEffect(() => {
    if (!active) return;
    let timer: number | undefined;
    let stopped = false;
    const tick = async () => {
      if (stopped) return;
      if (document.visibilityState === 'visible') await load();
      if (!stopped) timer = window.setTimeout(tick, POLL_MS);
    };
    void tick();
    return () => {
      stopped = true;
      window.clearTimeout(timer);
    };
  }, [active, load]);
  return { status, failed };
}

/** Command snippet with a copy button; the user runs it in their own terminal. */
export function CopySnippet({ command, label }: { command: string; label: string }) {
  const [copied, setCopied] = useState(false);
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(command);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1600);
    } catch {
      setCopied(false);
    }
  };
  return (
    <div className="sp-snippet">
      <code aria-label={label}>{command}</code>
      <button type="button" className="btn-control sp-snippet-copy" onClick={copy} aria-label={`Copy: ${command}`}>
        {copied ? 'Copied' : 'Copy'}
      </button>
      <span className="sr-only" role="status" aria-live="polite">
        {copied ? 'Copied to clipboard' : ''}
      </span>
    </div>
  );
}

const STATE_TEXT: Record<SetupStep['state'], string> = {
  done: 'Done',
  todo: 'To do',
  waiting: 'Waiting',
  unknown: 'Unknown',
};

function waitingText(step: SetupStep): string {
  if (step.id === 'signed_in') return 'Waiting for sign-in...';
  if (step.id === 'installed') return 'Waiting for install...';
  if (step.id === 'integrated') return 'Waiting for the hook...';
  if (step.id === 'first_data' || step.id === 'limits') return 'Waiting for first data...';
  return 'Checking...';
}

/**
 * The one per-provider checklist, used by onboarding, Settings > Providers and the doctor link.
 * Every row keeps a fixed structure so polling never shifts layout.
 */
export function ProviderChecklist({
  provider,
  distro,
  active = true,
  onStatus,
}: {
  provider: string;
  distro?: string;
  active?: boolean;
  onStatus?: (status: ProviderSetupStatus) => void;
}) {
  const { status, failed } = useProviderStatus(provider, distro, active);
  const notify = useRef(onStatus);
  notify.current = onStatus;
  useEffect(() => {
    if (status) notify.current?.(status);
  }, [status]);
  const name = status?.name ?? SETUP_PROVIDERS.find(([id]) => id === provider)?.[1] ?? provider;
  const where = distro ? `WSL: ${distro}` : 'this computer';
  const current = status?.steps.find((s) => s.state !== 'done' && !s.optional)?.id;
  return (
    <section className="sp-setup" aria-label={`${name} setup`} data-provider={provider} aria-busy={!status}>
      <header className="sp-setup-head">
        <h3>{name}</h3>
        <span className="sp-setup-meta">
          {status?.account ? `${status.account} · ` : ''}
          {where}
        </span>
        <span className={`sp-badge sp-setup-ready ${status?.ready ? 'is-ready' : ''}`}>
          {status ? (status.ready ? 'Ready' : 'Setup needed') : 'Checking'}
        </span>
      </header>
      {failed && !status ? <p role="status">Status unavailable. Is the daemon running?</p> : null}
      <ol className="sp-setup-steps">
        {(status?.steps ?? []).map((step) => {
          const live = step.id === current && step.state !== 'unknown';
          return (
            <li
              key={step.id}
              className={`sp-setup-step is-${step.state}${live ? ' is-current' : ''}`}
              data-step={step.id}
              data-state={step.state}
              aria-current={live ? 'step' : undefined}
            >
              <span className="sp-setup-mark" aria-hidden="true">
                {step.state === 'done' ? (
                  <svg viewBox="0 0 16 16" width="16" height="16">
                    <path className="sp-tick" d="M3.5 8.5l3 3 6-7" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
                  </svg>
                ) : null}
              </span>
              <div className="sp-setup-body">
                <p className="sp-setup-label">
                  {step.label}
                  {step.optional ? <span className="sp-setup-optional"> Optional</span> : null}
                  <span className="sr-only">{`: ${STATE_TEXT[step.state]}`}</span>
                </p>
                <p className="sp-setup-note">
                  {step.note}
                  {step.trust === 'estimated' ? <span className="sp-badge sp-trust"> EST</span> : null}
                </p>
                {step.state !== 'done' && step.command ? (
                  <>
                    <CopySnippet command={step.command} label={`${step.label} command`} />
                    {live ? (
                      <p className="sp-setup-wait" role="status" aria-live="polite">
                        <span className="sp-pulse" aria-hidden="true" />
                        {waitingText(step)}
                      </p>
                    ) : null}
                  </>
                ) : null}
              </div>
            </li>
          );
        })}
      </ol>
    </section>
  );
}

/** Settings > Providers: live status and fix commands for every provider, polled while on screen. */
export function ProviderSetupPanel() {
  const ref = useRef<HTMLDivElement | null>(null);
  const [visible, setVisible] = useState(false);
  const [distro, setDistro] = useState('');
  useEffect(() => {
    const node = ref.current;
    if (!node || typeof IntersectionObserver === 'undefined') {
      setVisible(true);
      return;
    }
    const observer = new IntersectionObserver((entries) => setVisible(entries.some((e) => e.isIntersecting)));
    observer.observe(node);
    return () => observer.disconnect();
  }, []);
  useEffect(() => {
    const reveal = () => {
      if (!takeProviderSetupRequest()) return;
      window.setTimeout(() => {
        ref.current?.scrollIntoView({ block: 'start' });
        ref.current?.focus({ preventScroll: true });
      }, 0);
    };
    reveal();
    window.addEventListener('gnomcode:provider-setup', reveal);
    return () => window.removeEventListener('gnomcode:provider-setup', reveal);
  }, []);
  const target = distro.trim() || undefined;
  return (
    <div id="provider-setup" ref={ref} className="sp-setup-panel" tabIndex={-1} aria-label="Provider setup">
      <label className="sp-onb-distro">
        <span>Run in</span>
        <input
          type="text"
          placeholder="This computer"
          aria-describedby="setup-distro-help"
          value={distro}
          onChange={(event) => setDistro(event.target.value)}
        />
      </label>
      <p id="setup-distro-help" className="sp-setup-note">
        Windows only: type a WSL distro name to check inside it.
      </p>
      {SETUP_PROVIDERS.map(([id]) => (
        <ProviderChecklist key={`${id}@${target ?? ''}`} provider={id} distro={target} active={visible} />
      ))}
    </div>
  );
}
