import { type KeyboardEvent, type MouseEvent, type ReactElement, useCallback, useEffect, useRef, useState } from 'react';
import { bindPreviewInteractions, copyTextToClipboard } from '../lib/console-presentation';
import { AccountView } from './AccountView';
import { ApprovalsCard } from './ApprovalsCard';
import { CommandPalette } from './CommandPalette';
import { fetchAccount, signOutDesktop } from './settings-index';
import { toggleChatList, useChatListCollapsed } from './chat-list';
import { HomeView } from './HomeView';
import { fetchLocalJson } from './local-api';
import { PaneResizer } from './pane-resizer';
import { SettingsView } from './SettingsView';
import { type ConsoleView, registerNavigator, requestProviderSetup, workspaceAction } from './shell-bus';
import { ShellDialogs } from './ShellDialogs';
import { UsageView } from './UsageView';
import { WorkspaceView } from './WorkspaceView';

// Old links keep working: Limits and Projects moved into Usage, Automations into Settings.
const VIEW_ALIASES: Record<string, string> = {
  limits: 'usage',
  projects: 'usage',
  automations: 'settings',
  'settings/providers': 'settings',
};
const VIEWS: ConsoleView[] = ['overview', 'sessions', 'usage', 'settings', 'auth'];
const CONSOLE_VIEWS: ConsoleView[] = ['overview', 'sessions', 'usage'];
const TITLES: Record<ConsoleView, string> = {
  overview: 'Home',
  sessions: 'Workspace',
  usage: 'Usage',
  settings: 'Settings',
  auth: 'Account',
};
const RAIL: { view: ConsoleView; label: string; controls: string; icon: ReactElement }[] = [
  {
    view: 'overview',
    label: 'Home',
    controls: 'view-console',
    icon: <path d="M3 11l9-8 9 8v10H3z M9 21v-6h6v6" />,
  },
  { view: 'sessions', label: 'Workspace', controls: 'view-console', icon: <path d="M4 4h16v12H9l-5 4V4Z" /> },
  { view: 'usage', label: 'Usage', controls: 'view-console', icon: <path d="M4 20V10m8 10V4m8 16v-7" /> },
  {
    view: 'settings',
    label: 'Settings',
    controls: 'view-settings',
    icon: (
      <>
        <path d="M4 7h10M18 7h2M4 17h2M10 17h10" />
        <circle cx="16" cy="7" r="2" />
        <circle cx="8" cy="17" r="2" />
      </>
    ),
  },
];

const POLL_MS = 2000;
const isPhone = () => window.matchMedia('(max-width: 767px)').matches;
const resolveView = (name: string): string => (name.startsWith('settings-') ? 'settings' : (VIEW_ALIASES[name] ?? name));
const isView = (name: string): name is ConsoleView => (VIEWS as string[]).includes(name);

function viewUrl(view: string): string {
  const url = new URL(window.location.href);
  url.hash = view === 'overview' ? '' : view;
  return url.pathname + url.search + url.hash;
}

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

// Rewrites an old #limits/#projects/#automations hash in place, so Back does not bounce through the old name.
function normalizeHash(): void {
  if (window.location.hash === '#settings/providers') {
    requestProviderSetup();
    history.replaceState(null, '', viewUrl('settings-providers'));
    return;
  }
  const target = VIEW_ALIASES[window.location.hash.slice(1)];
  if (target) history.replaceState(null, '', viewUrl(target));
}

function initialView(): ConsoleView {
  normalizeHash();
  let name = resolveView(window.location.hash.slice(1));
  if (!name) {
    try {
      name = resolveView(sessionStorage.getItem('gnomcode-active-view') || '');
    } catch {
      // Storage unavailable
    }
  }
  return isView(name) ? name : 'overview';
}

// Deep link from the file manager: #sessions?folder=<path> opens the launcher on that folder.
function takeExplorerFolder(): string | null {
  const url = new URL(window.location.href);
  if (!url.hash.startsWith('#sessions?')) return null;
  const folder = new URLSearchParams(url.hash.slice('#sessions?'.length)).get('folder');
  if (!folder) return null;
  url.hash = 'sessions';
  history.replaceState(null, '', url.toString());
  return folder;
}

let explorerFolder: string | null | undefined;
function consumeExplorerFolder(): string | null {
  if (explorerFolder === undefined) explorerFolder = takeExplorerFolder();
  return explorerFolder;
}

/**
 * The console around the four views: rail, title, connection status, view routing and the delegated
 * clicks for presenter markup. Data, reporting and every number stay in the daemon.
 */
export function ConsoleShell() {
  const [folder] = useState(consumeExplorerFolder);
  const [view, setView] = useState<ConsoleView>(() => (folder ? 'sessions' : initialView()));
  const [navTick, setNavTick] = useState(0);
  // The rail is a compact icon column unless the person asks for labels.
  const [railExpanded, setRailExpanded] = useState(() => storedFlag('gnomcode-rail-expanded') === '1');
  const [paired, setPaired] = useState(false);
  const [sync, setSync] = useState<{ text: string; failed: boolean }>({ text: 'Connecting', failed: false });
  const chatListCollapsed = useChatListCollapsed();
  const lastConsoleView = useRef<ConsoleView>('overview');
  const tabs = useRef<(HTMLButtonElement | null)[]>([]);
  const viewRef = useRef(view);
  const busy = useRef({ refresh: false, poll: false });
  const lastSuccess = useRef<Date | null>(null);
  viewRef.current = view;
  if (CONSOLE_VIEWS.includes(view)) lastConsoleView.current = view;

  // The views and the CSS read these from the page, not from React.
  useEffect(() => {
    document.body.dataset.consoleView = view;
    document.body.dataset.rail = railExpanded ? 'expanded' : 'collapsed';
    try {
      sessionStorage.setItem('gnomcode-active-view', view);
    } catch {
      // Storage unavailable
    }
  }, [view, railExpanded]);

  // The views load their own data; the shell only needs to know the daemon still answers.
  const probe = useCallback(async () => {
    try {
      await Promise.all([
        fetchLocalJson('/api/v1/sessions?page=1&limit=1'),
        fetchLocalJson('/api/v1/providers/overview'),
      ]);
      return true;
    } catch {
      return false;
    }
  }, []);

  const poll = useCallback(async () => {
    if (busy.current.refresh || busy.current.poll || document.hidden) return;
    if (viewRef.current === 'auth') return;
    busy.current.poll = true;
    try {
      if (await probe()) {
        lastSuccess.current = new Date();
        setSync({ text: 'Local daemon connected', failed: false });
      } else {
        setSync({ text: lastSuccess.current ? 'Local data stale' : 'Local data unavailable', failed: true });
      }
    } finally {
      busy.current.poll = false;
    }
  }, [probe]);

  const refresh = useCallback(async () => {
    if (busy.current.refresh) return;
    busy.current.refresh = true;
    try {
      while (busy.current.poll) await new Promise((resolve) => setTimeout(resolve, 25));
      window.dispatchEvent(new CustomEvent('gnomcode:reload-view', { detail: { view: viewRef.current } }));
      if (!(await probe())) throw new Error('Local data could not be loaded');
      lastSuccess.current = new Date();
      setSync({ text: 'View refreshed', failed: false });
    } catch {
      setSync({ text: 'Refresh failed', failed: true });
    } finally {
      busy.current.refresh = false;
    }
  }, [probe]);

  // Tell the views which place is showing, then ask them to reload when the person navigated.
  useEffect(() => {
    if (view !== 'auth') window.dispatchEvent(new CustomEvent('gnomcode:view-change', { detail: { view } }));
    if (navTick > 0) void refresh();
  }, [view, navTick, refresh]);

  const navigate = useCallback((target: string, push = true) => {
    const next = resolveView(target);
    if (!isView(next)) return;
    if (push && viewUrl(next) !== window.location.pathname + window.location.search + window.location.hash) {
      history.pushState(null, '', viewUrl(next));
    }
    setView(next);
    setNavTick((tick) => tick + 1);
  }, []);

  useEffect(() => registerNavigator(navigate), [navigate]);

  useEffect(() => {
    const onHashChange = () => {
      const nextFolder = takeExplorerFolder();
      if (nextFolder) {
        navigate('sessions', false);
        workspaceAction({ type: 'new-session', folder: nextFolder });
        return;
      }
      normalizeHash();
      const next = resolveView(window.location.hash.slice(1) || 'overview');
      if (isView(next)) navigate(next, false);
    };
    window.addEventListener('hashchange', onHashChange);
    return () => window.removeEventListener('hashchange', onHashChange);
  }, [navigate]);

  useEffect(() => {
    if (folder) workspaceAction({ type: 'new-session', folder });
    void poll();
    const timer = window.setInterval(() => void poll(), POLL_MS);
    return () => window.clearInterval(timer);
  }, [folder, poll]);

  // Presenter markup inside the views carries data-action and data-copy instead of handlers.
  useEffect(() => bindPreviewInteractions(document.body), []);
  useEffect(() => {
    const onClick = (event: Event) => {
      const origin = event.target as Element | null;
      const copy = origin?.closest<HTMLElement>('[data-copy]');
      if (copy) {
        const label = copy.querySelector<HTMLElement>('.sp-copy-label') ?? copy.querySelector('span');
        const original = label?.textContent || 'Copy';
        void copyTextToClipboard(copy.dataset.copy ?? '').then((ok) => {
          if (label) label.textContent = ok ? 'Copied' : 'Copy failed';
          copy.dataset.copyState = ok ? 'copied' : 'failed';
          window.setTimeout(() => {
            if (label) label.textContent = original;
            delete copy.dataset.copyState;
          }, 1600);
        });
        return;
      }
      if (origin?.closest('[data-project-id]')) {
        event.preventDefault();
        navigate('sessions');
        return;
      }
      const target = origin?.closest<HTMLElement>('[data-action]');
      const action = target?.dataset.action;
      if (!target || !action) return;
      if (action.startsWith('view:')) {
        event.preventDefault();
        navigate(action.slice(5));
        if ((event as globalThis.MouseEvent).detail && isPhone()) document.querySelector<HTMLElement>('.view-content.active')?.focus({ preventScroll: true });
      } else if (action === 'home-new-session') {
        navigate('sessions');
        workspaceAction({ type: 'new-session' });
      } else if (action === 'open-home-session') {
        navigate('sessions');
        workspaceAction({ type: 'open-session', sessionId: target.dataset.arg });
      }
    };
    document.addEventListener('click', onClick);
    return () => document.removeEventListener('click', onClick);
  }, [navigate]);

  // The account view reports whether this desktop is connected; the rail offers Sign in or Sign out.
  useEffect(() => {
    void fetchAccount().then(
      (account) => setPaired(account.paired),
      () => undefined,
    );
    const onState = (event: Event) => setPaired(Boolean((event as CustomEvent<{ paired: boolean }>).detail.paired));
    window.addEventListener('gnomcode:account-state', onState);
    return () => window.removeEventListener('gnomcode:account-state', onState);
  }, []);

  const onRailKey = (event: KeyboardEvent<HTMLElement>) => {
    const items = tabs.current.filter((node): node is HTMLButtonElement => node !== null);
    const current = items.indexOf(document.activeElement as HTMLButtonElement);
    if (current < 0) return;
    let next = current;
    if (event.key === 'ArrowRight' || event.key === 'ArrowDown') next = (current + 1) % items.length;
    else if (event.key === 'ArrowLeft' || event.key === 'ArrowUp') next = (current - 1 + items.length) % items.length;
    else if (event.key === 'Home') next = 0;
    else if (event.key === 'End') next = items.length - 1;
    else return;
    event.preventDefault();
    items[next].focus();
    navigate(RAIL[next].view);
  };

  const onRailClick = (event: MouseEvent<HTMLElement>, target: ConsoleView) => {
    event.preventDefault();
    navigate(target);
    if (event.detail && isPhone()) document.querySelector<HTMLElement>('.view-content.active')?.focus({ preventScroll: true });
  };

  const toggleRail = () => {
    const next = !railExpanded;
    setRailExpanded(next);
    storeFlag('gnomcode-rail-expanded', next ? '1' : '0');
  };

  const railLabel = railExpanded ? 'Collapse navigation' : 'Expand navigation';

  return (
    <>
      <div className="sp-console-workspace">
        <nav className="nav-tabs sp-console-nav" aria-label="Console views" onKeyDown={onRailKey}>
          <a className="app-brand" href={viewUrl('overview')} aria-label="Gnomcode home" onClick={(event) => onRailClick(event, 'overview')}>
            <span className="sp-brand-mark" aria-hidden="true" />
            <span className="rail-label">gnomcode</span>
          </a>
          <div role="tablist" aria-label="Console views" aria-orientation="vertical" className="sp-rail-tabs">
            {RAIL.map((item, index) => {
              const selected = item.view === view;
              return (
                <button
                  key={item.view}
                  ref={(node) => {
                    tabs.current[index] = node;
                  }}
                  className={`tab-btn${selected ? ' active' : ''}`}
                  id={`tab-btn-${item.view}`}
                  data-view={item.view}
                  role="tab"
                  aria-selected={selected}
                  aria-controls={item.controls}
                  tabIndex={selected || (view === 'auth' && item.view === 'settings') ? 0 : -1}
                  title={item.label}
                  onClick={(event) => onRailClick(event, item.view)}
                >
                  <svg viewBox="0 0 24 24" aria-hidden="true">
                    {item.icon}
                  </svg>
                  <span className="rail-label">{item.label}</span>
                </button>
              );
            })}
          </div>
          <div className="sp-nav-footer">
            <button
              type="button"
              className="tab-btn"
              id="account-entry"
              aria-label="Sign in to Gnomcode"
              title="Sign in to Gnomcode"
              hidden={paired}
              onClick={(event) => onRailClick(event, 'auth')}
            >
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M10 17l5-5-5-5M15 12H3M14 3h5a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2h-5" />
              </svg>
              <span className="rail-label" id="account-entry-label">
                Sign in
              </span>
            </button>
            <button
              type="button"
              className="tab-btn"
              id="account-sign-out"
              aria-label="Sign out of Gnomcode"
              title="Sign out of Gnomcode"
              hidden={!paired}
              onClick={() => void signOutDesktop()}
            >
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M14 17l5-5-5-5M19 12H7M10 3H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h5" />
              </svg>
              <span className="rail-label">Sign out</span>
            </button>
            <button
              type="button"
              className="tab-btn"
              id="command-palette-entry"
              aria-label="Search commands and settings"
              aria-keyshortcuts="Control+K Meta+K"
              title="Search commands and settings (Ctrl+K)"
              onClick={() => window.dispatchEvent(new CustomEvent('gnomcode:command-palette'))}
            >
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <circle cx="11" cy="11" r="6" />
                <path d="M20 20l-4.5-4.5" />
              </svg>
              <span className="rail-label">Search</span>
            </button>
            <button
              type="button"
              id="navigation-toggle"
              aria-label={railLabel}
              title={railLabel}
              aria-expanded={railExpanded}
              onClick={toggleRail}
            >
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M3 4h18v16H3z M9 4v16" />
              </svg>
              <span className="rail-label">{railLabel}</span>
            </button>
          </div>
        </nav>
        <div className="sp-chat-column" hidden={view !== 'sessions' || chatListCollapsed}>
          <div id="console-nav-sessions" className="sp-nav-sessions" />
          <PaneResizer cssVar="--chat-w" label="Resize conversations" min={220} max={440} initial={280} direction={1} />
        </div>
        <div className="sp-console-content">
          <div className="console-context">
            <h1 id="console-title">{TITLES[view]}</h1>
            <div className="console-status visually-hidden" aria-live="polite">
              <span id="sync-state" className={sync.failed ? 'local-status-error' : undefined}>
                {sync.text}
              </span>
            </div>
            <button
              type="button"
              className="btn-control"
              aria-controls="ide-session-sidebar"
              aria-expanded={!chatListCollapsed}
              id="chat-list-toggle"
              aria-label="Toggle conversations"
              title="Toggle conversations"
              hidden={view !== 'sessions'}
              onClick={toggleChatList}
            >
              <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true">
                <path d="M3 4h18v16H3z M9 4v16" fill="none" stroke="currentColor" strokeWidth="1.6" />
              </svg>
            </button>
          </div>
          <ApprovalsCard showGrants={view === 'sessions'} />
          <main
            id="view-console"
            className={`view-content${CONSOLE_VIEWS.includes(view) ? ' active' : ''}`}
            role="tabpanel"
            tabIndex={0}
            hidden={!CONSOLE_VIEWS.includes(view)}
            aria-labelledby={`tab-btn-${lastConsoleView.current}`}
          >
            <div id="local-usage-root" data-local-view="usage" hidden={view !== 'usage'}>
              <UsageView />
            </div>
            <div id="local-home-root" data-local-view="overview" className="sp-home" hidden={view !== 'overview'}>
              <HomeView />
            </div>
            <div id="local-workspace-root" data-local-view="sessions" hidden={view !== 'sessions'}>
              <WorkspaceView />
            </div>
          </main>
          <main
            id="view-settings"
            className={`view-content${view === 'settings' ? ' active' : ''}`}
            role="tabpanel"
            tabIndex={0}
            hidden={view !== 'settings'}
            aria-labelledby="tab-btn-settings"
          >
            <div id="local-settings-root">
              <SettingsView />
            </div>
          </main>
          <main id="view-auth" className={`view-content${view === 'auth' ? ' active' : ''}`} tabIndex={0} hidden={view !== 'auth'}>
            <div id="account-root">
              <AccountView />
            </div>
          </main>
        </div>
      </div>
      <ShellDialogs />
      <CommandPalette />
    </>
  );
}
