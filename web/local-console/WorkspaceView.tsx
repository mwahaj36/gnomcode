'use client';

import {
  type FormEvent,
  type KeyboardEvent,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import {
  formatPromptWithAttachments,
  isSessionRunningState,
  reconcileChat,
  renderConversationLimits,
  renderSessionList,
  updateChat,
  type ConversationLimit,
  type PendingChat,
  type ProjectActivity,
} from '../lib/console-presentation.js';
import {
  formatWorktreeBadge,
  renderWorktreeDiffMarkup,
  type WorktreeDiffData,
} from '../lib/worktree-presentation.mjs';
import {
  fetchLocalJson,
  fetchLocalMutation,
  getLocalBearerToken,
  type LocalSessionSummary,
  type LocalSessionsResponse,
  type LocalTerminalInstance,
} from './local-api';
import { AttachmentsBar, QuickActionsBar, SessionTools, useSessionAttachments } from './WorkspaceTools';

interface HostedSession {
  session_id: string;
  agent?: string;
  title?: string;
  model?: string;
  model_observed?: boolean;
  repo_path?: string;
  started_at?: string;
  worktree?: boolean;
  worktree_branch?: string;
  worktree_path?: string;
}

interface InstalledAgent {
  id: string;
  name: string;
  installed: boolean;
  install_command?: string;
  login_command?: string;
}

interface CatalogModel {
  agent: string;
  model: string;
  display_name?: string;
}

interface CatalogProvider {
  name: string;
  message?: string;
  models?: CatalogModel[];
}

interface ChatEvent {
  event_id: string;
  turn_index: number;
  role: string;
  content?: string;
  created_at: string;
}

interface WorkspaceBrowseResponse {
  type?: string;
  message?: string;
  path?: string;
  parent?: string | null;
  roots?: string[];
  folders?: Array<{ name: string; path: string }>;
  truncated?: boolean;
}

interface ScheduledMessage {
  id: string;
  due_at: number;
  status: string;
  text?: string;
}

interface AutomationNotice {
  conversation_id?: string | null;
  message: string;
}

interface WorkspaceTerminalEntry {
  sessionId: string;
  repoPath: string;
  element: HTMLDivElement;
  terminal: LocalTerminalInstance;
  socket: WebSocket;
  observer: ResizeObserver;
  resize: () => void;
}

const POLL_INTERVAL_MS = 2_000;
const PAGE_LIMIT = 50;
const LAST_CHAT_STORAGE_KEY = 'gnomcode-last-chat';
const CLI_AGENT_MAP: Record<string, string> = {
  'claude-code': 'claude',
};

function isWorkspaceVisible(): boolean {
  if (typeof document === 'undefined') return false;
  const host = document.getElementById('local-workspace-root');
  return Boolean(host && !host.hidden && document.visibilityState !== 'hidden');
}

function createUuid(): string {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return crypto.randomUUID();
  }
  return `id-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`;
}

function buildPtySocketUrl(sessionId: string): string {
  const url = new URL(`/api/v1/sessions/${encodeURIComponent(sessionId)}/pty`, window.location.href);
  url.protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  const token = getLocalBearerToken();
  if (token) {
    url.searchParams.set('token', token);
  }
  return url.toString();
}

export function WorkspaceView() {
  const [sessions, setSessions] = useState<LocalSessionSummary[]>([]);
  const [selectedCache, setSelectedCache] = useState<Record<string, LocalSessionSummary>>({});
  const [hostedSessions, setHostedSessions] = useState<HostedSession[]>([]);
  const [projects, setProjects] = useState<ProjectActivity[]>([]);
  const [installedAgents, setInstalledAgents] = useState<InstalledAgent[]>([]);
  const [providers, setProviders] = useState<CatalogProvider[]>([]);
  const [catalogError, setCatalogError] = useState(false);

  const [sourceFilter, setSourceFilter] = useState('all');
  const [searchQuery, setSearchQuery] = useState('');
  const [includeSubagents, setIncludeSubagents] = useState(false);
  const [projectFilter, setProjectFilter] = useState('');
  const [agentFilter, setAgentFilter] = useState('all');
  const [statusFilter, setStatusFilter] = useState('all');
  const [page, setPage] = useState(1);
  const [totalPages, setTotalPages] = useState(1);
  const [totalCount, setTotalCount] = useState(0);
  const [sidebarError, setSidebarError] = useState('');

  const [activeSessionId, setActiveSessionId] = useState('');
  const [launcherOpen, setLauncherOpen] = useState(false);
  const launchAgentRef = useRef<HTMLSelectElement | null>(null);
  const focusLauncherRef = useRef(false);

  // Launcher form state
  const [launchAgent, setLaunchAgent] = useState('claude');
  const [launchRepo, setLaunchRepo] = useState('');
  const [launchModel, setLaunchModel] = useState('');
  const [launchWorktree, setLaunchWorktree] = useState(false);
  const [launchWorktreeBranch, setLaunchWorktreeBranch] = useState('');
  const [launchStatus, setLaunchStatus] = useState('');
  const [launchBusy, setLaunchBusy] = useState(false);
  const [agentHintError, setAgentHintError] = useState('');

  // Worktree review state
  const [worktreeDiffOpen, setWorktreeDiffOpen] = useState(false);
  const [worktreeDiff, setWorktreeDiff] = useState<WorktreeDiffData | null>(null);
  const [worktreeConfirmAction, setWorktreeConfirmAction] = useState<
    'merge' | 'discard' | 'open-pr' | null
  >(null);
  const [worktreeBusy, setWorktreeBusy] = useState(false);
  const [worktreeStatus, setWorktreeStatus] = useState('');

  // Workspace folder browser dialog state
  const [browserOpen, setBrowserOpen] = useState(false);
  const [browserPath, setBrowserPath] = useState('');
  const [browserParent, setBrowserParent] = useState<string | null>(null);
  const [browserRoots, setBrowserRoots] = useState<string[]>([]);
  const [browserFolders, setBrowserFolders] = useState<Array<{ name: string; path: string }>>([]);
  const [browserStatus, setBrowserStatus] = useState('');

  // Chat state
  const [chatEvents, setChatEvents] = useState<ChatEvent[]>([]);
  const [pendingBySession, setPendingBySession] = useState<Record<string, PendingChat[]>>({});
  const [draftsBySession, setDraftsBySession] = useState<Record<string, string>>({});
  const [chatStatus, setChatStatus] = useState('');
  const [actionStatus, setActionStatus] = useState('');
  const [copyPath, setCopyPath] = useState('');
  const [deliveryStatus, setDeliveryStatus] = useState('');
  const [hasEarlier, setHasEarlier] = useState(false);
  const [terminalStreamOpen, setTerminalStreamOpen] = useState(false);
  const [workspaceTerminalOpen, setWorkspaceTerminalOpen] = useState(false);
  const [workspaceTerminalPath, setWorkspaceTerminalPath] = useState('');
  const [confirmDeleteOpen, setConfirmDeleteOpen] = useState(false);
  const [selectedModelValue, setSelectedModelValue] = useState('');
  const [modelHandoffBusy, setModelHandoffBusy] = useState(false);
  const [sendBusy, setSendBusy] = useState(false);
  const [resumeBusy, setResumeBusy] = useState(false);
  const [continueHereBusy, setContinueHereBusy] = useState(false);

  // Automation state
  const [scheduledMessages, setScheduledMessages] = useState<ScheduledMessage[]>([]);
  const [resetArmed, setResetArmed] = useState(false);
  const [routingPolicy, setRoutingPolicy] = useState('ask_first');
  const [routingDirty, setRoutingDirty] = useState(false);
  const [automationNotices, setAutomationNotices] = useState<AutomationNotice[]>([]);
  const [scheduleTime, setScheduleTime] = useState('');
  const [scheduleStatus, setScheduleStatus] = useState('');
  const [schedulingBusy, setSchedulingBusy] = useState(false);

  const sessionListRef = useRef<HTMLDivElement | null>(null);
  const chatStreamRef = useRef<HTMLDivElement | null>(null);
  const chatTerminalRef = useRef<HTMLDivElement | null>(null);
  const workspaceTerminalScreenRef = useRef<HTMLDivElement | null>(null);
  const promptInputRef = useRef<HTMLTextAreaElement | null>(null);
  const browserDialogRef = useRef<HTMLDialogElement | null>(null);

  const activeSessionIdRef = useRef('');
  activeSessionIdRef.current = activeSessionId;
  const chatRequestRef = useRef(0);
  const conversationRequestRef = useRef(0);
  const workspaceRequestRef = useRef(0);
  const reopenTriedRef = useRef(false);
  const deletionGenRef = useRef<Record<string, number>>({});
  const chatSocketRef = useRef<WebSocket | null>(null);
  const workspaceTerminalsRef = useRef<Map<string, WorkspaceTerminalEntry>>(new Map());
  const workspaceTerminalBusyRef = useRef(false);
  const pendingSendRef = useRef<Set<string>>(new Set());
  const attachInputRef = useRef<HTMLInputElement | null>(null);
  const attachments = useSessionAttachments(activeSessionId);

  const activeSession = useMemo(
    () =>
      sessions.find((item) => item.id === activeSessionId) ??
      selectedCache[activeSessionId] ??
      null,
    [sessions, selectedCache, activeSessionId],
  );
  const activeHosted = useMemo(
    () => hostedSessions.find((item) => item.session_id === activeSessionId) ?? null,
    [hostedSessions, activeSessionId],
  );

  const toggleChatSidebar = useCallback(() => {
    if (typeof document === 'undefined') return;
    const collapsed = !document.body.classList.contains('chat-list-collapsed');
    document.body.classList.toggle('chat-list-collapsed', collapsed);
    document.getElementById('chat-list-toggle')?.setAttribute('aria-expanded', String(!collapsed));
    if (!window.matchMedia('(max-width: 760px)').matches) {
      try {
        localStorage.setItem('gnomcode-chat-list-collapsed', collapsed ? '1' : '0');
      } catch {
        // Storage unavailable
      }
    }
  }, []);

  const resizePrompt = useCallback(() => {
    const input = promptInputRef.current;
    if (!input) return;
    input.style.height = 'auto';
    input.style.height = `${Math.min(160, Math.max(36, input.scrollHeight))}px`;
  }, []);

  const loadProjects = useCallback(async () => {
    try {
      const list = await fetchLocalJson<ProjectActivity[]>('/api/v1/projects?range=all');
      if (Array.isArray(list)) {
        setProjects(list);
      }
    } catch {
      // Retain previous project list on transient failure
    }
  }, []);

  const loadInstalledAgents = useCallback(async () => {
    try {
      const data = await fetchLocalJson<{ agents?: InstalledAgent[] }>('/api/v1/agents/installed');
      if (Array.isArray(data?.agents) && data.agents.length) {
        setInstalledAgents(data.agents);
        setAgentHintError('');
        setLaunchAgent((current) =>
          data.agents?.some((item) => item.id === current)
            ? current
            : (data.agents?.[0]?.id ?? 'claude'),
        );
      }
    } catch {
      setAgentHintError(
        'Agent detection failed. Reopen New Session to retry, or launch Terminal Shell.',
      );
    }
  }, []);

  const loadChatModels = useCallback(async () => {
    try {
      const data = await fetchLocalJson<{ providers?: CatalogProvider[] }>('/api/v1/model-catalog');
      setProviders(Array.isArray(data?.providers) ? data.providers : []);
      setCatalogError(false);
    } catch {
      setProviders([]);
      setCatalogError(true);
    }
  }, []);

  const closeAgentTerminal = useCallback(() => {
    if (chatSocketRef.current) {
      chatSocketRef.current.close();
      chatSocketRef.current = null;
    }
  }, []);

  const refreshAutomation = useCallback(async (sessionId: string) => {
    if (!sessionId) return;
    try {
      const [schedules, routing, notices] = await Promise.all([
        fetchLocalMutation<{ messages?: ScheduledMessage[]; reset_armed?: boolean }>(
          '/api/v1/automation',
          { method: 'POST', body: { type: 'schedules', session_id: sessionId } },
        ),
        fetchLocalMutation<{ policy?: string }>('/api/v1/automation', {
          method: 'POST',
          body: { type: 'routing' },
        }),
        fetchLocalMutation<{ notices?: AutomationNotice[] }>('/api/v1/automation', {
          method: 'POST',
          body: { type: 'notices' },
        }),
      ]);
      if (activeSessionIdRef.current !== sessionId) return;
      setScheduledMessages(Array.isArray(schedules?.messages) ? schedules.messages : []);
      setResetArmed(Boolean(schedules?.reset_armed));
      setRoutingDirty((dirty) => {
        if (!dirty && routing?.policy) {
          setRoutingPolicy(routing.policy);
        }
        return dirty;
      });
      const relevantNotices = Array.isArray(notices?.notices)
        ? notices.notices
            .filter((item) => item.conversation_id === sessionId || !item.conversation_id)
            .slice(0, 8)
        : [];
      setAutomationNotices(relevantNotices);
      setScheduleStatus('');
    } catch (error) {
      if (activeSessionIdRef.current !== sessionId) return;
      const msg = error instanceof Error ? error.message : 'unavailable';
      setScheduleStatus(`Schedule status unavailable. ${msg}`);
    }
  }, []);

  const loadChat = useCallback(
    async (
      sessionId: string,
      options: { earlier?: boolean; silent?: boolean } = {},
    ) => {
      if (!sessionId) return;
      const { earlier = false, silent = false } = options;
      if (silent && activeSessionIdRef.current !== sessionId) return;

      if (!silent && !earlier) {
        activeSessionIdRef.current = sessionId;
        setActiveSessionId(sessionId);
        setLauncherOpen(false);
        setWorkspaceTerminalOpen(false);
        setConfirmDeleteOpen(false);
        setActionStatus('');
        setCopyPath('');
        setDeliveryStatus('');
        closeAgentTerminal();
        setTerminalStreamOpen(false);
        setChatEvents([]);
        try {
          localStorage.setItem(
            LAST_CHAT_STORAGE_KEY,
            JSON.stringify({ id: sessionId, at: Date.now() }),
          );
        } catch {
          // Storage unavailable
        }
      }

      const request = ++chatRequestRef.current;
      if (!silent) {
        if (!earlier) {
          setWorktreeDiffOpen(false);
          setWorktreeConfirmAction(null);
          setWorktreeStatus('');
          setWorktreeDiff(null);
        }
        void loadChatModels();
        void refreshAutomation(sessionId);
        if (!earlier) setChatStatus('Loading conversation...');
      }

      try {
        const currentEvents = earlier ? chatEvents : [];
        const before =
          earlier && currentEvents.length
            ? `&before=${encodeURIComponent(currentEvents[0].event_id)}`
            : '';
        const pageEvents = await fetchLocalJson<ChatEvent[]>(
          `/api/v1/sessions/${encodeURIComponent(sessionId)}/chat?limit=500${before}`,
        );
        if (request !== chatRequestRef.current || sessionId !== activeSessionIdRef.current) {
          return;
        }
        setChatEvents((prev) => {
          const base = earlier || silent ? prev : [];
          const merged = new Map(base.map((item) => [item.event_id, item]));
          for (const item of Array.isArray(pageEvents) ? pageEvents : []) {
            merged.set(item.event_id, item);
          }
          return [...merged.values()].sort(
            (a, b) =>
              a.turn_index - b.turn_index ||
              a.created_at.localeCompare(b.created_at) ||
              a.event_id.localeCompare(b.event_id),
          );
        });
        if (!silent) {
          setHasEarlier(Array.isArray(pageEvents) && pageEvents.length >= 500);
          window.requestAnimationFrame(resizePrompt);
        }
        const messageCount = (Array.isArray(pageEvents) ? pageEvents : []).filter(
          (event) => (event.role === 'user' || event.role === 'assistant') && event.content?.trim(),
        ).length;
        setChatStatus(
          messageCount ? `${messageCount} message${messageCount === 1 ? '' : 's'}` : '',
        );
      } catch {
        if (request !== chatRequestRef.current || sessionId !== activeSessionIdRef.current) {
          return;
        }
        if (!silent) {
          setChatStatus(
            'Could not load conversation history. Reload the view and check the desktop connection.',
          );
          window.requestAnimationFrame(resizePrompt);
        }
      }
    },
    [chatEvents, closeAgentTerminal, loadChatModels, refreshAutomation, resizePrompt],
  );

  const loadConversations = useCallback(async () => {
    const request = ++conversationRequestRef.current;
    try {
      const [hostedResult, convResult] = await Promise.all([
        fetchLocalJson<{ sessions?: HostedSession[] }>('/api/v1/sessions/hosted').catch(() => ({
          sessions: [],
        })),
        fetchLocalJson<LocalSessionsResponse | LocalSessionSummary[]>(
          `/api/v1/conversations?page=${page}&limit=${PAGE_LIMIT}&agent=${encodeURIComponent(agentFilter)}&status=${encodeURIComponent(statusFilter)}&project=${encodeURIComponent(projectFilter)}&search=${encodeURIComponent(searchQuery)}&source=${encodeURIComponent(sourceFilter)}&include_subagents=${includeSubagents}`,
        ),
      ]);
      if (request !== conversationRequestRef.current) return;

      const hosted = Array.isArray(hostedResult?.sessions) ? hostedResult.sessions : [];
      const rows = Array.isArray(convResult)
        ? convResult
        : Array.isArray(convResult?.sessions)
          ? convResult.sessions
          : [];
      setHostedSessions(hosted);
      setSessions(rows);
      setSidebarError('');

      const selectedRow = rows.find((item) => item.id === activeSessionIdRef.current);
      if (selectedRow) {
        setSelectedCache((prev) => ({ ...prev, [selectedRow.id]: selectedRow }));
      }

      if (!Array.isArray(convResult)) {
        const nextTotalPages = convResult?.total_pages || 1;
        const nextTotalCount =
          convResult?.total_count != null ? convResult.total_count : rows.length;
        setTotalPages(nextTotalPages);
        setTotalCount(nextTotalCount);
      } else {
        setTotalPages(1);
        setTotalCount(rows.length);
      }

      if (!reopenTriedRef.current && isWorkspaceVisible()) {
        reopenTriedRef.current = true;
        if (!activeSessionIdRef.current) {
          try {
            const raw = localStorage.getItem(LAST_CHAT_STORAGE_KEY);
            const parsed = raw ? (JSON.parse(raw) as { id?: string; at?: number }) : null;
            if (
              parsed?.id &&
              parsed.at &&
              Date.now() - parsed.at < 86_400_000 &&
              rows.some((row) => row.id === parsed.id)
            ) {
              void loadChat(parsed.id);
            }
          } catch {
            // No stored chat
          }
        }
      }
    } catch {
      if (request === conversationRequestRef.current) {
        setSidebarError('Could not load conversations.');
      }
    }
  }, [
    agentFilter,
    includeSubagents,
    loadChat,
    page,
    projectFilter,
    searchQuery,
    sourceFilter,
    statusFilter,
  ]);

  const openLauncher = useCallback(
    (open = true, folder?: string) => {
      reopenTriedRef.current = true;
      activeSessionIdRef.current = '';
      chatRequestRef.current += 1;
      closeAgentTerminal();
      setTerminalStreamOpen(false);
      setWorkspaceTerminalOpen(false);
      setActiveSessionId('');
      setChatEvents([]);
      setLauncherOpen(open);
      setLaunchStatus('');
      if (folder) {
        setLaunchRepo(folder);
      }
      if (open) {
        void loadInstalledAgents();
      }
    },
    [closeAgentTerminal, loadInstalledAgents],
  );

  // The first-run tour hands the launcher focus; opening it with New leaves focus where it is.
  useEffect(() => {
    if (launcherOpen && focusLauncherRef.current) {
      focusLauncherRef.current = false;
      launchAgentRef.current?.focus();
    }
  }, [launcherOpen]);

  // Sync rendered conversation list via shared pure helper
  useEffect(() => {
    if (!sessionListRef.current) return;
    if (sidebarError) {
      sessionListRef.current.textContent = sidebarError;
      return;
    }
    sessionListRef.current.innerHTML = renderSessionList(sessions, activeSessionId);
  }, [sessions, activeSessionId, sidebarError]);

  // Sync reconciled chat cards via shared pure helper
  useEffect(() => {
    if (!chatStreamRef.current || !activeSessionId) return;
    const sessionPending = pendingBySession[activeSessionId] || [];
    const result = reconcileChat(chatEvents, sessionPending, activeSession?.state || '');
    updateChat(chatStreamRef.current, result.events);
  }, [activeSessionId, chatEvents, pendingBySession, activeSession?.state]);

  // Keep model switcher value aligned with active session/hosted model
  const allCatalogModels = useMemo(
    () => providers.flatMap((provider) => provider.models || []),
    [providers],
  );
  const currentSessionModel = activeHosted?.model || activeSession?.model || '';
  const currentSessionAgent = activeHosted?.agent || activeSession?.agent || '';
  const currentCliAgent = CLI_AGENT_MAP[currentSessionAgent] || currentSessionAgent;
  const matchedCatalogOption = useMemo(
    () =>
      allCatalogModels.find(
        (item) => item.model === currentSessionModel && item.agent === currentCliAgent,
      ) ?? null,
    [allCatalogModels, currentSessionModel, currentCliAgent],
  );
  const currentModelSelection = matchedCatalogOption
    ? `${matchedCatalogOption.agent}:${matchedCatalogOption.model}`
    : currentSessionModel;

  useEffect(() => {
    if (!modelHandoffBusy) {
      setSelectedModelValue(currentModelSelection);
    }
  }, [currentModelSelection, modelHandoffBusy, activeSessionId]);

  // Initial load + 2s poll while Workspace is visible
  useEffect(() => {
    void loadProjects();
    void loadInstalledAgents();
    void loadConversations();

    const timer = window.setInterval(() => {
      if (!isWorkspaceVisible()) return;
      void loadConversations();
      const id = activeSessionIdRef.current;
      if (id) {
        void refreshAutomation(id);
        if (!terminalStreamOpen) {
          void loadChat(id, { silent: true });
        }
      }
    }, POLL_INTERVAL_MS);

    return () => {
      window.clearInterval(timer);
    };
  }, [
    loadChat,
    loadConversations,
    loadInstalledAgents,
    loadProjects,
    refreshAutomation,
    terminalStreamOpen,
  ]);

  // Listen to shell navigation, reload, and Home/Explorer bridges
  useEffect(() => {
    const onViewChange = (event: Event) => {
      const detail = (event as CustomEvent<{ view?: string }>).detail;
      if (detail?.view === 'sessions') {
        void loadProjects();
        void loadInstalledAgents();
        void loadConversations();
      }
    };
    const onReload = (event: Event) => {
      const detail = (event as CustomEvent<{ view?: string }>).detail;
      if (detail?.view === 'sessions') {
        void loadProjects();
        void loadConversations();
        if (activeSessionIdRef.current) {
          void loadChat(activeSessionIdRef.current);
        }
      }
    };
    const onWorkspaceAction = (event: Event) => {
      const detail = (
        event as CustomEvent<{ type?: string; sessionId?: string; folder?: string; focus?: boolean }>
      ).detail;
      if (!detail?.type) return;
      if (detail.type === 'new-session') {
        focusLauncherRef.current = Boolean(detail.focus);
        openLauncher(true, detail.folder);
      } else if (detail.type === 'open-session' && detail.sessionId) {
        void loadChat(detail.sessionId);
      } else if (detail.type === 'toggle-chat-list') {
        toggleChatSidebar();
      }
    };

    window.addEventListener('gnomcode:view-change', onViewChange);
    window.addEventListener('gnomcode:reload-view', onReload);
    window.addEventListener('gnomcode:workspace-action', onWorkspaceAction);
    window.__gnomcodeLocalActions?.workspaceReady?.();
    return () => {
      window.removeEventListener('gnomcode:view-change', onViewChange);
      window.removeEventListener('gnomcode:reload-view', onReload);
      window.removeEventListener('gnomcode:workspace-action', onWorkspaceAction);
    };
  }, [loadChat, loadConversations, loadInstalledAgents, loadProjects, openLauncher, toggleChatSidebar]);

  // Handle Escape / cleanup of sockets on unmount
  useEffect(() => {
    const terminals = workspaceTerminalsRef.current;
    return () => {
      closeAgentTerminal();
      for (const entry of terminals.values()) {
        entry.observer.disconnect();
        entry.socket.close();
        entry.terminal.dispose();
      }
      terminals.clear();
    };
  }, [closeAgentTerminal]);

  // Sync dialog modal state cleanly
  useEffect(() => {
    const dialog = browserDialogRef.current;
    if (!dialog) return;
    if (browserOpen && !dialog.open) {
      dialog.showModal();
    } else if (!browserOpen && dialog.open) {
      dialog.close();
    }
  }, [browserOpen]);

  const browseWorkspace = useCallback(async (targetPath?: string) => {
    const request = ++workspaceRequestRef.current;
    setBrowserStatus('Loading folders...');
    try {
      const data = await fetchLocalMutation<WorkspaceBrowseResponse>('/api/v1/workspaces', {
        method: 'POST',
        body: targetPath ? { path: targetPath } : {},
      });
      if (request !== workspaceRequestRef.current) return;
      if (data.type === 'error') {
        throw new Error(data.message || 'Cannot open this folder.');
      }
      setBrowserPath(data.path || '');
      setBrowserParent(data.parent ?? null);
      setBrowserRoots(Array.isArray(data.roots) ? data.roots : []);
      const folders = Array.isArray(data.folders) ? data.folders : [];
      setBrowserFolders(folders);
      setBrowserStatus(
        data.truncated
          ? 'More folders exist. Enter a full path if needed.'
          : folders.length
            ? ''
            : 'No subfolders.',
      );
    } catch (error) {
      if (request === workspaceRequestRef.current) {
        setBrowserStatus(
          error instanceof Error ? error.message : 'Cannot open this folder.',
        );
      }
    }
  }, []);

  const handleChooseWorkspace = async () => {
    try {
      if (window.gnomcodeDesktop?.chooseWorkspace) {
        const folder = await window.gnomcodeDesktop.chooseWorkspace();
        if (folder) setLaunchRepo(folder);
      } else {
        setBrowserOpen(true);
        await browseWorkspace();
      }
    } catch {
      setLaunchStatus('Folder picker unavailable. Enter the workspace path and retry.');
    }
  };

  const selectedInstalledAgent = installedAgents.find((item) => item.id === launchAgent);
  const canLaunchAgent = Boolean(launchRepo.trim()) && !launchBusy && !(selectedInstalledAgent && !selectedInstalledAgent.installed);

  const connectAgentTerminal = useCallback((sessionId: string) => {
    if (chatSocketRef.current || !sessionId || !chatTerminalRef.current) return;
    const host = chatTerminalRef.current;
    host.replaceChildren();
    if (!window.Terminal || !window.FitAddon) {
      host.textContent = 'Terminal renderer unavailable.';
      return;
    }
    const terminal = new window.Terminal({
      cols: 80,
      rows: 24,
      scrollback: 3000,
      fontSize: 13,
      fontFamily: 'Consolas, monospace',
      allowProposedApi: false,
    });
    const fit = new window.FitAddon.FitAddon();
    terminal.loadAddon(fit);
    terminal.open(host);

    const socket = new WebSocket(buildPtySocketUrl(sessionId));
    socket.binaryType = 'arraybuffer';
    chatSocketRef.current = socket;

    const resize = () => {
      if (host.hidden) return;
      fit.fit();
      if (socket.readyState === WebSocket.OPEN) {
        socket.send(JSON.stringify({ resize: { cols: terminal.cols, rows: terminal.rows } }));
      }
    };
    const observer = new ResizeObserver(resize);
    observer.observe(host);
    socket.onopen = () => {
      resize();
      terminal.focus();
    };
    socket.onmessage = (event) => {
      terminal.write(
        typeof event.data === 'string' ? event.data : new Uint8Array(event.data as ArrayBuffer),
      );
    };
    const inputSub = terminal.onData((data) => {
      if (socket.readyState === WebSocket.OPEN) {
        socket.send(new TextEncoder().encode(data));
      }
    });
    socket.onerror = () =>
      terminal.writeln('\r\nAgent terminal unavailable. Reopen this view to retry.');
    socket.onclose = () => {
      observer.disconnect();
      inputSub.dispose();
      terminal.dispose();
      if (chatSocketRef.current === socket) {
        chatSocketRef.current = null;
        host.textContent = 'Agent terminal disconnected. Reopen this view to reconnect.';
      }
    };
    window.requestAnimationFrame(resize);
  }, []);

  const handleToggleAgentTerminal = () => {
    if (!activeSessionId) return;
    if (terminalStreamOpen) {
      setTerminalStreamOpen(false);
      closeAgentTerminal();
    } else {
      setTerminalStreamOpen(true);
      window.requestAnimationFrame(() => connectAgentTerminal(activeSessionId));
    }
  };

  const handleContinueHere = async () => {
    if (!activeSessionId || continueHereBusy) return;
    setContinueHereBusy(true);
    setActionStatus('Continuing imported session in Gnomcode...');
    try {
      const result = await fetchLocalMutation<{ session_id?: string; new_session_id?: string }>(
        `/api/v1/sessions/${encodeURIComponent(activeSessionId)}/continue-here`,
        { method: 'POST', body: {} },
      );
      const nextId = result?.new_session_id || result?.session_id;
      await loadConversations();
      if (nextId) {
        await loadChat(nextId);
        setActionStatus('Conversation continued in Gnomcode.');
      } else {
        setActionStatus('Continuation requested.');
      }
    } catch (error) {
      const reason = error instanceof Error && error.message ? error.message : 'Could not continue this session here.';
      const agentId =
        activeSession?.agent === 'claude-code' ? 'claude' : activeSession?.agent || 'claude';
      openLauncher(true, activeSession?.repo_path || '');
      setLaunchAgent(agentId);
      // ADR 0053: refusal messages are user-readable; keep the launcher prefill as the fallback.
      setLaunchStatus(`${reason} Prefilled this workspace so you can start a new Gnomcode session instead.`);
      setActionStatus('');
    } finally {
      setContinueHereBusy(false);
    }
  };

  const handleToggleWorktreeDiff = async () => {
    if (!activeSessionId) return;
    if (worktreeDiffOpen) {
      setWorktreeDiffOpen(false);
      return;
    }
    setWorktreeDiffOpen(true);
    setWorktreeDiff({ loading: true });
    try {
      const result = await fetchLocalJson<WorktreeDiffData>(
        `/api/v1/sessions/${encodeURIComponent(activeSessionId)}/changes`,
      );
      setWorktreeDiff(result);
    } catch (error) {
      const msg = error instanceof Error ? error.message : 'Unable to load changes';
      setWorktreeDiff({ error: msg });
    }
  };

  const handleWorktreeLifecycle = async (action: 'merge' | 'discard' | 'open-pr') => {
    if (!activeSessionId || worktreeBusy) return;
    setWorktreeBusy(true);
    setWorktreeConfirmAction(null);
    const label =
      action === 'merge' ? 'Merging worktree' : action === 'discard' ? 'Discarding worktree' : 'Opening pull request';
    setWorktreeStatus(`${label}...`);
    try {
      const result = await fetchLocalMutation<{ ok?: boolean; message?: string; url?: string }>(
        `/api/v1/sessions/${encodeURIComponent(activeSessionId)}/worktree/${action}`,
        { method: 'POST', body: {} },
      );
      setWorktreeStatus(
        result?.url
          ? `${result.message || 'Pull request ready:'} ${result.url}`
          : result?.message || `${label} completed.`,
      );
      await loadConversations();
      if (action === 'discard') setWorktreeDiffOpen(false);
    } catch (error) {
      const msg = error instanceof Error ? error.message : 'Request failed';
      // The daemon explains refusals itself (agent still inside, dirty base, conflicts, gh missing).
      setWorktreeStatus(`Worktree action failed. ${msg}`);
    } finally {
      setWorktreeBusy(false);
    }
  };

  const handleLaunchSubmit = async (event: FormEvent) => {
    event.preventDefault();
    const repoPath = launchRepo.trim();
    if (!launchAgent || !repoPath) {
      setLaunchStatus('Please provide an agent and a workspace directory.');
      return;
    }
    setLaunchBusy(true);
    setLaunchStatus(`Launching ${launchAgent}...`);
    try {
      const record = await fetchLocalMutation<{ session_id: string }>('/api/v1/sessions/launch', {
        method: 'POST',
        body: {
          agent: launchAgent,
          repo_path: repoPath,
          model: launchAgent === 'terminal' ? undefined : launchModel.trim() || undefined,
          worktree: launchWorktree || undefined,
          worktree_branch:
            launchWorktree && launchWorktreeBranch.trim() ? launchWorktreeBranch.trim() : undefined,
        },
      });
      setLaunchStatus(`Session ${record.session_id} launched! Opening conversation...`);
      await loadConversations();
      await loadChat(record.session_id);
      if (launchAgent === 'terminal') {
        setTerminalStreamOpen(true);
        window.requestAnimationFrame(() => connectAgentTerminal(record.session_id));
      }
    } catch (error) {
      const msg = error instanceof Error ? error.message : 'Launch failed';
      setLaunchStatus(`Could not launch: ${msg}`);
    } finally {
      setLaunchBusy(false);
    }
  };

  const handleOpenWorkspaceTerminal = async () => {
    const source = activeSessionId;
    if (!source || workspaceTerminalBusyRef.current || !workspaceTerminalScreenRef.current) return;
    workspaceTerminalBusyRef.current = true;
    try {
      let entry = workspaceTerminalsRef.current.get(source);
      if (!entry) {
        if (!window.Terminal || !window.FitAddon) {
          setDeliveryStatus('Terminal renderer unavailable.');
          return;
        }
        const record = await fetchLocalMutation<{ session_id: string; repo_path: string }>(
          `/api/v1/sessions/${encodeURIComponent(source)}/terminal`,
          { method: 'POST' },
        );
        const container = workspaceTerminalScreenRef.current;
        container.querySelectorAll('[data-disconnected]').forEach((node) => node.remove());
        const element = document.createElement('div');
        container.append(element);
        const terminal = new window.Terminal({
          cols: 80,
          rows: 12,
          scrollback: 3000,
          fontSize: 13,
          fontFamily: 'Consolas, monospace',
          theme: { background: '#0d0d0f', foreground: '#f6f7f8' },
          allowProposedApi: false,
        });
        const fit = new window.FitAddon.FitAddon();
        terminal.loadAddon(fit);
        terminal.open(element);
        const socket = new WebSocket(buildPtySocketUrl(record.session_id));
        socket.binaryType = 'arraybuffer';
        const resize = () => {
          if (element.hidden || !workspaceTerminalScreenRef.current) return;
          fit.fit();
          if (socket.readyState === WebSocket.OPEN) {
            socket.send(JSON.stringify({ resize: { cols: terminal.cols, rows: terminal.rows } }));
          }
        };
        const observer = new ResizeObserver(resize);
        observer.observe(container);
        socket.onopen = resize;
        socket.onmessage = (evt) => {
          terminal.write(
            typeof evt.data === 'string' ? evt.data : new Uint8Array(evt.data as ArrayBuffer),
          );
        };
        socket.onerror = () =>
          terminal.writeln('\r\nTerminal connection unavailable. Close and reopen this pane.');
        socket.onclose = () => {
          observer.disconnect();
          workspaceTerminalsRef.current.delete(source);
          terminal.dispose();
          element.textContent = 'Terminal disconnected. Reopen the pane to reconnect.';
          element.dataset.disconnected = 'true';
        };
        terminal.onData((data) => {
          if (socket.readyState === WebSocket.OPEN) {
            socket.send(new TextEncoder().encode(data));
          }
        });
        entry = {
          sessionId: record.session_id,
          repoPath: record.repo_path,
          element,
          terminal,
          socket,
          observer,
          resize,
        };
        workspaceTerminalsRef.current.set(source, entry);
      }
      if (source !== activeSessionIdRef.current) return;
      workspaceTerminalScreenRef.current
        .querySelectorAll<HTMLDivElement>('div')
        .forEach((node) => {
          node.hidden = node !== entry?.element;
        });
      setWorkspaceTerminalPath(entry.repoPath);
      setWorkspaceTerminalOpen(true);
      window.requestAnimationFrame(() => {
        entry?.resize();
        entry?.terminal.focus();
      });
    } catch {
      setDeliveryStatus(
        'Could not open a shell in this workspace. Check that the directory still exists.',
      );
    } finally {
      workspaceTerminalBusyRef.current = false;
    }
  };

  const sendPromptText = async (rawText: string, fromComposer: boolean) => {
    const sessionId = activeSessionId;
    const draft = rawText.trim();
    const ready = fromComposer ? attachments.items.filter((item) => item.status === 'saved' && item.workspace_path) : [];
    const text = formatPromptWithAttachments(draft, ready);
    if (!text || !sessionId || pendingSendRef.current.has(sessionId)) return;

    const generation = deletionGenRef.current[sessionId] || 0;
    const optimistic: PendingChat = {
      id: createUuid(),
      text,
      created_at: new Date().toISOString(),
      baseline: chatEvents.map((item) => item.event_id),
      delivered: false,
    };

    setPendingBySession((prev) => ({
      ...prev,
      [sessionId]: [...(prev[sessionId] || []), optimistic],
    }));
    pendingSendRef.current.add(sessionId);
    setSendBusy(true);
    setDeliveryStatus('');

    try {
      await fetchLocalMutation(`/api/v1/sessions/${encodeURIComponent(sessionId)}/prompt`, {
        method: 'POST',
        body: { text },
      });
      if ((deletionGenRef.current[sessionId] || 0) !== generation) return;

      setPendingBySession((prev) => ({
        ...prev,
        [sessionId]: (prev[sessionId] || []).map((item) =>
          item.id === optimistic.id ? { ...item, delivered: true } : item,
        ),
      }));
      if (fromComposer) {
        attachments.clearSaved(sessionId);
        setDraftsBySession((prev) =>
          (prev[sessionId] || '').trim() === draft ? { ...prev, [sessionId]: '' } : prev,
        );
      }
      if (sessionId === activeSessionIdRef.current) {
        if (fromComposer) {
          window.requestAnimationFrame(resizePrompt);
          promptInputRef.current?.focus();
        }
        window.setTimeout(() => void loadChat(sessionId, { silent: true }), 1000);
      }
    } catch (error) {
      setPendingBySession((prev) => ({
        ...prev,
        [sessionId]: (prev[sessionId] || []).filter((item) => item.id !== optimistic.id),
      }));
      if (sessionId === activeSessionIdRef.current) {
        const msg = error instanceof Error ? error.message : 'The terminal did not accept the prompt.';
        setDeliveryStatus(`Could not send. ${msg}`);
      }
    } finally {
      pendingSendRef.current.delete(sessionId);
      setSendBusy(pendingSendRef.current.has(activeSessionIdRef.current));
    }
  };

  const handleSendPrompt = (event: FormEvent) => {
    event.preventDefault();
    void sendPromptText(draftsBySession[activeSessionId] || '', true);
  };

  const handleModelSwitch = async (nextValue: string) => {
    setSelectedModelValue(nextValue);
    if (!activeSessionId || !nextValue || modelHandoffBusy || nextValue === currentModelSelection) {
      return;
    }
    const chosen = allCatalogModels.find((item) => `${item.agent}:${item.model}` === nextValue);
    const targetModel = chosen?.model || nextValue;
    const targetAgent = chosen?.agent;
    const sourceSession = activeSessionId;
    const draft = pendingSendRef.current.has(sourceSession)
      ? ''
      : draftsBySession[sourceSession] || '';

    setModelHandoffBusy(true);
    pendingSendRef.current.add(sourceSession);
    setSendBusy(true);

    try {
      const result = await fetchLocalMutation<{ session_id: string }>(
        `/api/v1/sessions/${encodeURIComponent(sourceSession)}/switch`,
        {
          method: 'POST',
          body: { target_model: targetModel, agent: targetAgent },
        },
      );
      await loadConversations();
      if (activeSessionIdRef.current === sourceSession) {
        await loadChat(result.session_id);
        if (draft) {
          setDraftsBySession((prev) => ({ ...prev, [result.session_id]: draft }));
        }
      }
    } catch (error) {
      setSelectedModelValue(currentModelSelection);
      if (activeSessionIdRef.current === sourceSession) {
        setDeliveryStatus(
          error instanceof Error
            ? error.message
            : 'Model change could not be confirmed. Your draft is saved.',
        );
      }
    } finally {
      setModelHandoffBusy(false);
      pendingSendRef.current.delete(sourceSession);
      setSendBusy(pendingSendRef.current.has(activeSessionIdRef.current));
    }
  };

  const handleFocusChat = async () => {
    if (!activeSessionId) return;
    try {
      const result = await fetchLocalMutation<{
        message?: string;
        focused?: boolean;
        repo_path?: string;
      }>(`/api/v1/sessions/${encodeURIComponent(activeSessionId)}/focus`, { method: 'POST' });
      setActionStatus(
        result.message || (result.focused ? 'Agent window opened.' : 'Session window unavailable.'),
      );
      setCopyPath(!result.focused && result.repo_path ? result.repo_path : '');
    } catch {
      setActionStatus('Window unavailable. Use the terminal stream for a hosted session.');
    }
  };

  const handleResumeChat = async () => {
    if (!activeSessionId) return;
    const source = activeSessionId;
    setResumeBusy(true);
    setDeliveryStatus('Resuming conversation...');
    try {
      await fetchLocalMutation(`/api/v1/sessions/${encodeURIComponent(source)}/continue`, {
        method: 'POST',
      });
      await loadConversations();
      if (source === activeSessionIdRef.current) {
        await loadChat(source);
        setDeliveryStatus('Ready to continue.');
      }
    } catch {
      setDeliveryStatus(
        'Could not resume. Check that the agent is installed and signed in, then try again.',
      );
    } finally {
      setResumeBusy(false);
    }
  };

  const handleDeleteChatConfirmed = async () => {
    const sessionId = activeSessionId;
    if (!sessionId) return;
    setConfirmDeleteOpen(false);
    try {
      await fetchLocalMutation(`/api/v1/sessions/${encodeURIComponent(sessionId)}/chat`, {
        method: 'DELETE',
      });
      deletionGenRef.current[sessionId] = (deletionGenRef.current[sessionId] || 0) + 1;
      setPendingBySession((prev) => ({ ...prev, [sessionId]: [] }));
      if (activeSessionIdRef.current === sessionId) {
        await loadChat(sessionId);
      }
    } catch {
      setChatStatus('Could not delete local chat history.');
    }
  };

  const handleScheduleDraft = async () => {
    if (schedulingBusy || !activeSessionId) return;
    const sessionId = activeSessionId;
    const text = (draftsBySession[sessionId] || '').trim();
    const dueMs = new Date(scheduleTime).getTime();
    if (!text || !Number.isFinite(dueMs) || dueMs <= Date.now()) {
      setScheduleStatus('Enter a message and a future delivery time.');
      return;
    }
    setSchedulingBusy(true);
    try {
      await fetchLocalMutation('/api/v1/automation', {
        method: 'POST',
        body: {
          type: 'schedule',
          session_id: sessionId,
          id: createUuid(),
          text,
          due_at: Math.floor(dueMs / 1000),
        },
      });
      setDraftsBySession((prev) =>
        (prev[sessionId] || '').trim() === text ? { ...prev, [sessionId]: '' } : prev,
      );
      setScheduleStatus('Schedule confirmed on your laptop. You can close your phone.');
      await refreshAutomation(sessionId);
    } catch (error) {
      const msg = error instanceof Error ? error.message : 'Schedule error';
      setScheduleStatus(`Schedule not confirmed. Check saved messages before retrying. ${msg}`);
    } finally {
      setSchedulingBusy(false);
    }
  };

  const handleCancelSchedule = async (id: string) => {
    if (!activeSessionId) return;
    try {
      await fetchLocalMutation('/api/v1/automation', {
        method: 'POST',
        body: { type: 'cancel_schedule', session_id: activeSessionId, id },
      });
      await refreshAutomation(activeSessionId);
    } catch (error) {
      setScheduleStatus(error instanceof Error ? error.message : 'Could not cancel schedule.');
    }
  };

  const handleSaveRouting = async () => {
    try {
      await fetchLocalMutation('/api/v1/automation', {
        method: 'POST',
        body: { type: 'routing', policy: routingPolicy },
      });
      setRoutingDirty(false);
      setScheduleStatus('Routing policy saved on this laptop.');
    } catch (error) {
      setScheduleStatus(error instanceof Error ? error.message : 'Could not save routing policy.');
    }
  };

  const handleResetReconnect = async (enabled: boolean) => {
    if (!activeSessionId) return;
    try {
      await fetchLocalMutation('/api/v1/automation', {
        method: 'POST',
        body: { type: 'resume_on_reset', session_id: activeSessionId, enabled },
      });
      setResetArmed(enabled);
      setScheduleStatus(
        enabled
          ? 'Reconnection armed. It waits for fresh quota confirmation after reset.'
          : 'Reset reconnection cancelled.',
      );
    } catch (error) {
      setScheduleStatus(error instanceof Error ? error.message : 'Could not update reset state.');
    }
  };

  const handleSessionListClick = (event: React.MouseEvent<HTMLDivElement>) => {
    const target = (event.target as HTMLElement).closest<HTMLElement>('[data-action="load-chat"]');
    const sessionId = target?.dataset.arg;
    if (!sessionId) return;
    void loadChat(sessionId);
    if (window.matchMedia('(max-width: 760px)').matches) {
      document.body.classList.add('chat-list-collapsed');
      document.getElementById('chat-list-toggle')?.setAttribute('aria-expanded', 'false');
    }
  };

  const handleChatStreamClick = (event: React.MouseEvent<HTMLDivElement>) => {
    const target = (event.target as HTMLElement).closest<HTMLElement>('[data-permission-action]');
    if (!target || !activeSessionId) return;
    const action = target.dataset.permissionAction;
    const toolCallId = target.dataset.toolCallId;
    if (!action || !toolCallId) return;
    void fetchLocalMutation(`/api/v1/sessions/${encodeURIComponent(activeSessionId)}/permissions`, {
      method: 'POST',
      body: { action, tool_call_id: toolCallId },
    }).then(() => loadChat(activeSessionId));
  };

  const handlePromptKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key === 'Enter' && !event.shiftKey && !event.nativeEvent.isComposing) {
      event.preventDefault();
      event.currentTarget.form?.requestSubmit();
    }
  };

  const imported = activeSession?.source === 'imported';
  const limitsMarkup = useMemo(
    () =>
      renderConversationLimits(
        Array.isArray(activeSession?.limits) ? (activeSession.limits as ConversationLimit[]) : [],
      ),
    [activeSession?.limits],
  );
  const currentDraft = draftsBySession[activeSessionId] || '';
  const hasCatalogModels = allCatalogModels.length > 0;
  const modelStatusText = catalogError
    ? 'Model selection unavailable'
    : activeHosted?.model_observed ?? Boolean(activeSession?.model)
      ? ''
      : currentSessionModel
        ? 'Switching model...'
        : 'Agent default';
  const importedAgentLabel =
    activeSession?.agent === 'antigravity'
      ? 'Google Antigravity'
      : activeSession?.agent || 'your agent application';

  return (
    <section className="ide-layout" aria-label="Agent workspace">
      <aside className="ide-sidebar" aria-label="Session history">
        <div className="ide-sidebar-title">
          <h2>Conversations</h2>
          <button
            type="button"
            className="btn-control"
            onClick={toggleChatSidebar}
            aria-label="Collapse conversations"
            title="Collapse conversations"
          >
            <svg
              viewBox="0 0 24 24"
              width="18"
              height="18"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.6"
              aria-hidden="true"
            >
              <path d="M3 4h18v16H3z M9 4v16 m-2-7-2 2 2 2" />
            </svg>
          </button>
        </div>

        <button className="btn-control" type="button" onClick={() => openLauncher(true)}>
          New
        </button>

        <label>
          History
          <select
            value={sourceFilter}
            onChange={(event) => {
              setPage(1);
              setSourceFilter(event.target.value);
            }}
          >
            <option value="all">All sessions</option>
            <option value="managed">Gnomcode conversations</option>
            <option value="imported">Imported sessions (view-only)</option>
          </select>
        </label>

        <input
          type="search"
          aria-label="Search conversations"
          placeholder="Search conversations"
          value={searchQuery}
          onChange={(event) => {
            setPage(1);
            setSearchQuery(event.target.value);
          }}
        />

        <details className="conversation-filters">
          <summary>Filters</summary>
          <div className="filters">
            <label className="subagent-filter">
              <input
                type="checkbox"
                checked={includeSubagents}
                onChange={(event) => {
                  setPage(1);
                  setIncludeSubagents(event.target.checked);
                }}
              />{' '}
              Include subagents
            </label>
            <label>
              Project{' '}
              <select
                value={projectFilter}
                onChange={(event) => {
                  setPage(1);
                  setProjectFilter(event.target.value);
                }}
              >
                <option value="">All projects</option>
                {projects.map((project) => (
                  <option key={project.id} value={project.id}>
                    {project.name}
                  </option>
                ))}
              </select>
            </label>
            <select
              aria-label="Agent"
              value={agentFilter}
              onChange={(event) => {
                setPage(1);
                setAgentFilter(event.target.value);
              }}
            >
              <option value="all">All agents</option>
              <option value="antigravity">Antigravity</option>
              <option value="codex">OpenAI Codex</option>
              <option value="claude-code">Claude Code</option>
            </select>
            <select
              aria-label="Session status"
              value={statusFilter}
              onChange={(event) => {
                setPage(1);
                setStatusFilter(event.target.value);
              }}
            >
              <option value="all">All statuses</option>
              <option value="ongoing">Ongoing</option>
              <option value="active">Running or waiting</option>
              <option value="idle">Idle</option>
              <option value="ended">Ended</option>
            </select>
          </div>
        </details>

        <div
          ref={sessionListRef}
          className="sp-workspace-list"
          onClick={handleSessionListClick}
        >
          Loading conversations...
        </div>

        <div className="pagination">
          <span>{`${page} / ${totalPages} · ${totalCount} conversations`}</span>
          {totalPages > 1 ? (
            <div className="sp-workspace-pagination-controls">
              <button
                type="button"
                className="btn-control"
                disabled={page <= 1}
                onClick={() => setPage((current) => Math.max(1, current - 1))}
              >
                Previous
              </button>
              <button
                type="button"
                className="btn-control"
                disabled={page >= totalPages}
                onClick={() => setPage((current) => Math.min(totalPages, current + 1))}
              >
                Next
              </button>
            </div>
          ) : null}
        </div>
      </aside>

      <div className="ide-main">
        <button
          type="button"
          className="btn-control ide-expand-sidebar"
          onClick={toggleChatSidebar}
          aria-label="Expand conversations"
          title="Expand conversations"
        >
          <svg
            viewBox="0 0 24 24"
            width="16"
            height="16"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.6"
            aria-hidden="true"
          >
            <path d="M3 4h18v16H3z M9 4v16 m-4-7 2 2-2 2" />
          </svg>
          <span>Conversations</span>
        </button>

        {!launcherOpen && !activeSessionId ? (
          <div className="sp-empty sp-workspace-empty">
            <p>Pick a conversation or start a new one.</p>
            <button type="button" className="btn-primary" onClick={() => openLauncher(true)}>
              New session
            </button>
          </div>
        ) : null}

        {launcherOpen ? (
          <section className="ide-launcher" aria-label="New agent session">
            <h2>New agent session</h2>
            <p>Choose an agent and the folder it should work in.</p>
            <form onSubmit={handleLaunchSubmit}>
              <div>
                <label>
                  Agent
                  <select
                    ref={launchAgentRef}
                    value={launchAgent}
                    onChange={(event) => setLaunchAgent(event.target.value)}
                  >
                    {installedAgents.length ? (
                      installedAgents.map((agent) => (
                        <option key={agent.id} value={agent.id}>
                          {agent.name}
                          {agent.installed ? '' : ' [Not installed]'}
                        </option>
                      ))
                    ) : (
                      <>
                        <option value="claude">Claude Code</option>
                        <option value="codex">OpenAI Codex</option>
                        <option value="antigravity">Google Antigravity</option>
                        <option value="terminal">Terminal Shell</option>
                      </>
                    )}
                  </select>
                </label>
              </div>
              <div>
                <label>
                  Workspace directory
                  <div className="workspace-field">
                    <input
                      type="text"
                      placeholder="Choose a folder or enter its path"
                      value={launchRepo}
                      onChange={(event) => setLaunchRepo(event.target.value)}
                      required
                    />
                    <button
                      type="button"
                      className="btn-control sp-launcher-browse"
                      onClick={() => void handleChooseWorkspace()}
                      aria-label="Choose folder"
                      title="Choose folder"
                    >
                      <svg
                        viewBox="0 0 24 24"
                        width="20"
                        height="20"
                        fill="none"
                        stroke="currentColor"
                        strokeWidth="1.6"
                        aria-hidden="true"
                      >
                        <path d="M3 6h7l2 3h9v11H3V6Z" />
                      </svg>
                    </button>
                  </div>
                </label>
              </div>
              <div>
                <label>
                  Model (optional)
                  <input
                    type="text"
                    placeholder="Use the agent default"
                    maxLength={128}
                    disabled={launchAgent === 'terminal'}
                    value={launchModel}
                    onChange={(event) => setLaunchModel(event.target.value)}
                  />
                </label>
              </div>
              <div className="sp-worktree-launch">
                <label className="sp-worktree-toggle">
                  <input
                    type="checkbox"
                    checked={launchWorktree}
                    onChange={(event) => setLaunchWorktree(event.target.checked)}
                  />
                  <span>Isolate in git worktree</span>
                </label>
                {launchWorktree ? (
                  <label>
                    Worktree branch (optional)
                    <input
                      type="text"
                      placeholder="gnomcode/session-branch"
                      maxLength={120}
                      value={launchWorktreeBranch}
                      onChange={(event) => setLaunchWorktreeBranch(event.target.value)}
                    />
                  </label>
                ) : null}
              </div>
              <p role="status">
                {agentHintError
                  ? agentHintError
                  : selectedInstalledAgent && !selectedInstalledAgent.installed
                    ? `${selectedInstalledAgent.name} is not found on PATH. Run ${selectedInstalledAgent.install_command || ''} then ${selectedInstalledAgent.login_command || ''}, or launch Terminal Shell.`
                    : selectedInstalledAgent
                      ? `${selectedInstalledAgent.name} is available on PATH. Provider login is checked by the CLI when it starts.`
                      : 'Checking installed agents...'}
              </p>
              <button type="submit" className="btn-primary" disabled={!canLaunchAgent}>
                Launch Agent
              </button>
            </form>

            <dialog
              ref={browserDialogRef}
              className="sp-workspace-browse-dialog"
              aria-label="Choose workspace folder"
              onClose={() => setBrowserOpen(false)}
            >
              <h2>Choose workspace folder</h2>
              <p>{browserPath}</p>
              <div className="sp-workspace-browse-roots">
                {browserRoots.map((root) => (
                  <button
                    key={root}
                    type="button"
                    className="btn-control"
                    onClick={() => void browseWorkspace(root)}
                  >
                    {root}
                  </button>
                ))}
              </div>
              <div className="sp-workspace-browse-list">
                {browserParent ? (
                  <button
                    type="button"
                    className="btn-control"
                    onClick={() => void browseWorkspace(browserParent)}
                  >
                    Up one folder
                  </button>
                ) : null}
                {browserFolders.map((folder) => (
                  <button
                    key={folder.path}
                    type="button"
                    className="btn-control"
                    onClick={() => void browseWorkspace(folder.path)}
                  >
                    {folder.name}
                  </button>
                ))}
              </div>
              <p role="status">{browserStatus}</p>
              <div className="dialog-actions">
                <button
                  type="button"
                  className="btn-control"
                  onClick={() => setBrowserOpen(false)}
                >
                  Cancel
                </button>
                <button
                  type="button"
                  className="btn-primary"
                  disabled={!browserPath}
                  onClick={() => {
                    if (browserPath) setLaunchRepo(browserPath);
                    setBrowserOpen(false);
                  }}
                >
                  Use this folder
                </button>
              </div>
            </dialog>

            {launchStatus ? <p role="status">{launchStatus}</p> : null}
          </section>
        ) : null}

        {activeSessionId ? (
          <section className="sp-workspace-chat-panel" aria-label="Conversation">
            <div className="conversation-heading">
              <div>
                <h2>
                  {activeSession?.title || activeHosted?.title || 'New conversation'}
                </h2>
                <p>
                  {activeHosted?.agent || activeSession?.agent || 'Agent'}
                  {imported ? ' · Observed' : ''}
                </p>
              </div>
              <div
                className="sp-conversation-limits"
                role="status"
                aria-label="Remaining limits"
                dangerouslySetInnerHTML={{ __html: limitsMarkup }}
              />
              {activeSession?.can_resume ? (
                <button
                  type="button"
                  className="btn-control"
                  disabled={resumeBusy}
                  onClick={() => void handleResumeChat()}
                >
                  Resume conversation
                </button>
              ) : null}
              <button
                type="button"
                className="btn-control"
                onClick={() => void handleOpenWorkspaceTerminal()}
              >
                Terminal
              </button>
              <details className="conversation-menu">
                <summary aria-label="Conversation actions">Actions</summary>
                <div>
                  <button
                    type="button"
                    className="btn-control"
                    onClick={() => void handleFocusChat()}
                  >
                    Focus Window
                  </button>
                  {!imported ? (
                    <button
                      type="button"
                      className="btn-control"
                      onClick={handleToggleAgentTerminal}
                    >
                      {terminalStreamOpen ? 'Show conversation' : 'Agent terminal'}
                    </button>
                  ) : null}
                  <button
                    type="button"
                    className="btn-control"
                    onClick={() => openLauncher(false)}
                  >
                    Close
                  </button>
                  {chatEvents.length > 0 ? (
                    <button
                      type="button"
                      className="btn-control"
                      onClick={() => setConfirmDeleteOpen(true)}
                    >
                      Delete local chat history
                    </button>
                  ) : null}
                </div>
              </details>
            </div>

            <SessionTools
              sessionId={activeSessionId}
              repoPath={activeHosted?.repo_path || activeSession?.repo_path || copyPath}
              canInterrupt={
                !imported &&
                (activeHosted !== null || (activeSession as { hosted?: boolean } | null)?.hosted !== false) &&
                isSessionRunningState(activeSession?.state || '')
              }
            />
            {activeSession?.worktree ||
            activeSession?.worktree_branch ||
            activeSession?.worktree_path ||
            activeHosted?.worktree ||
            activeHosted?.worktree_branch ? (
              <div className="sp-worktree-bar" role="region" aria-label="Session git worktree">
                <div className="sp-worktree-summary">
                  <span className="sp-chip sp-chip--neutral">
                    {formatWorktreeBadge(activeSession || activeHosted)}
                  </span>
                  {activeSession?.worktree_path || activeHosted?.worktree_path ? (
                    <code className="sp-worktree-path">
                      {activeSession?.worktree_path || activeHosted?.worktree_path}
                    </code>
                  ) : null}
                </div>
                <div className="sp-worktree-actions">
                  <button
                    type="button"
                    className="btn-control"
                    disabled={worktreeBusy}
                    onClick={() => void handleToggleWorktreeDiff()}
                  >
                    {worktreeDiffOpen ? 'Hide diff' : 'Review diff'}
                  </button>
                  <button
                    type="button"
                    className="btn-control"
                    disabled={worktreeBusy}
                    onClick={() => setWorktreeConfirmAction('merge')}
                  >
                    Merge
                  </button>
                  <button
                    type="button"
                    className="btn-control"
                    disabled={worktreeBusy}
                    onClick={() => setWorktreeConfirmAction('open-pr')}
                  >
                    Open PR
                  </button>
                  <button
                    type="button"
                    className="btn-control sp-tone-red"
                    disabled={worktreeBusy}
                    onClick={() => setWorktreeConfirmAction('discard')}
                  >
                    Discard
                  </button>
                </div>
                {worktreeConfirmAction ? (
                  <div className="sp-confirm-inline" role="region" aria-label="Confirm worktree action">
                    <p>
                      {worktreeConfirmAction === 'merge'
                        ? 'Merge changes from this session worktree into the base branch?'
                        : worktreeConfirmAction === 'discard'
                          ? 'Discard this session worktree and delete uncommitted worktree changes?'
                          : 'Push this worktree branch and open a pull request?'}
                    </p>
                    <div className="dialog-actions">
                      <button
                        type="button"
                        className="btn-control"
                        onClick={() => setWorktreeConfirmAction(null)}
                      >
                        Cancel
                      </button>
                      <button
                        type="button"
                        className="btn-primary"
                        disabled={worktreeBusy}
                        onClick={() => void handleWorktreeLifecycle(worktreeConfirmAction)}
                      >
                        Confirm{' '}
                        {worktreeConfirmAction === 'open-pr' ? 'Open PR' : worktreeConfirmAction}
                      </button>
                    </div>
                  </div>
                ) : null}
                {worktreeStatus ? <p role="status">{worktreeStatus}</p> : null}
                {worktreeDiffOpen ? (
                  <div
                    className="sp-worktree-diff-panel"
                    dangerouslySetInnerHTML={{ __html: renderWorktreeDiffMarkup(worktreeDiff) }}
                  />
                ) : null}
              </div>
            ) : null}

            {confirmDeleteOpen ? (
              <div className="sp-confirm-inline" role="region" aria-label="Confirm deletion">
                <p>
                  Delete Gnomcode&apos;s local chat history for this session? Agent-owned logs and
                  backups stay intact.
                </p>
                <div className="dialog-actions">
                  <button
                    type="button"
                    className="btn-control"
                    onClick={() => setConfirmDeleteOpen(false)}
                  >
                    Cancel
                  </button>
                  <button
                    type="button"
                    className="btn-primary"
                    onClick={() => void handleDeleteChatConfirmed()}
                  >
                    Delete history
                  </button>
                </div>
              </div>
            ) : null}

            {chatStatus ? <p role="status">{chatStatus}</p> : null}
            {actionStatus ? <p role="status">{actionStatus}</p> : null}
            {copyPath ? (
              <button
                type="button"
                className="btn-control"
                onClick={() => {
                  void navigator.clipboard
                    .writeText(copyPath)
                    .then(() => setActionStatus('Workspace path copied.'))
                    .catch(() =>
                      setActionStatus(
                        'Could not copy the path. Check clipboard access and retry.',
                      ),
                    );
                }}
              >
                Copy workspace path
              </button>
            ) : null}

            <div
              ref={chatStreamRef}
              className="sp-chat-stream-box"
              role="log"
              aria-label="Conversation"
              aria-live="polite"
              tabIndex={0}
              hidden={terminalStreamOpen}
              onClick={handleChatStreamClick}
            />
            <div
              ref={chatTerminalRef}
              className="sp-chat-terminal-box"
              role="log"
              aria-label="Agent terminal"
              tabIndex={0}
              hidden={!terminalStreamOpen}
            />

            {imported ? (
              <div className="sp-imported-banner">
                <div className="sp-imported-info">
                  <span>
                    {`This conversation is active in ${importedAgentLabel}. Responses, tokens and quota are observed in real time.`}
                  </span>
                </div>
                <div className="sp-imported-actions">
                  <button
                    type="button"
                    className="btn-primary"
                    disabled={continueHereBusy}
                    onClick={() => void handleContinueHere()}
                  >
                    {continueHereBusy ? 'Continuing...' : 'Continue here'}
                  </button>
                  <button
                    type="button"
                    className="btn-control"
                    onClick={() => void handleFocusChat()}
                  >
                    {activeSession?.agent === 'antigravity'
                      ? 'Open in Antigravity'
                      : 'Focus Agent Window'}
                  </button>
                </div>
              </div>
            ) : (
              <>
                <QuickActionsBar disabled={sendBusy} onSend={(text) => void sendPromptText(text, false)} />
                <form className="sp-chat-composer" onSubmit={handleSendPrompt}>
                  <AttachmentsBar items={attachments.items} status={attachments.status} onRemove={attachments.remove} />
                  <input
                    ref={attachInputRef}
                    type="file"
                    hidden
                    accept=".png,.jpg,.jpeg,.webp,.gif,.txt,.md,.log,.json,.csv,.pdf"
                    onChange={(event) => {
                      const file = event.target.files?.[0];
                      event.target.value = '';
                      void attachments.pick(file);
                    }}
                  />
                  <textarea
                    ref={promptInputRef}
                    className="sp-chat-composer-input"
                    aria-label="Message the agent"
                    rows={1}
                    maxLength={4000}
                    placeholder="Message the agent..."
                    value={currentDraft}
                    onChange={(event) => {
                      const next = event.target.value;
                      setDraftsBySession((prev) => ({ ...prev, [activeSessionId]: next }));
                      resizePrompt();
                    }}
                    onKeyDown={handlePromptKeyDown}
                  />
                  <div className="composer-toolbar">
                    <div className="composer-model">
                      <select
                        aria-label="Model"
                        disabled={!hasCatalogModels || modelHandoffBusy}
                        value={selectedModelValue}
                        onChange={(event) => void handleModelSwitch(event.target.value)}
                      >
                        <option value="">
                          {catalogError ? 'Model catalog unavailable' : 'Agent default'}
                        </option>
                        {providers.map((provider) => (
                          <optgroup key={provider.name} label={provider.name}>
                            {provider.models?.length ? (
                              provider.models.map((item) => (
                                <option
                                  key={`${item.agent}:${item.model}`}
                                  value={`${item.agent}:${item.model}`}
                                >
                                  {item.display_name || item.model}
                                </option>
                              ))
                            ) : (
                              <option disabled>{provider.message || 'No models'}</option>
                            )}
                          </optgroup>
                        ))}
                        {currentSessionModel && !matchedCatalogOption ? (
                          <option value={currentSessionModel} disabled>
                            {`${currentSessionModel} (current)`}
                          </option>
                        ) : null}
                      </select>
                      <span role="status">{modelStatusText}</span>
                    </div>
                    <button type="button" className="btn-control" onClick={() => attachInputRef.current?.click()}>
                      Attach
                    </button>
                    <button type="submit" className="btn-primary" disabled={sendBusy}>
                      Send
                    </button>
                  </div>
                  <small className="sp-chat-composer-help">
                    Enter to send · Shift+Enter for a new line
                  </small>
                </form>

                <details className="profile-disclosure sp-chat-automation">
                  <summary>Schedule and automation</summary>
                  <p>
                    Saved on this laptop after confirmation. Keep it awake with Gnomcode running.
                    Message text is redacted before storage.
                  </p>
                  <label>
                    Delivery time (your local time)
                    <input
                      type="datetime-local"
                      value={scheduleTime}
                      onChange={(event) => setScheduleTime(event.target.value)}
                    />
                  </label>
                  <button
                    type="button"
                    className="btn-control"
                    disabled={schedulingBusy}
                    onClick={() => void handleScheduleDraft()}
                  >
                    Schedule current draft
                  </button>
                  <div role="status">
                    {scheduleStatus ? <p>{scheduleStatus}</p> : null}
                    {scheduledMessages.length ? (
                      scheduledMessages.map((msg) => (
                        <p key={msg.id}>
                          {`${new Date(msg.due_at * 1000).toLocaleString()} - ${msg.status}`}
                          {msg.text ? (
                            <>
                              <br />
                              {msg.text.slice(0, 120)}
                            </>
                          ) : null}
                          {msg.status === 'pending' ? (
                            <>
                              {' '}
                              <button
                                type="button"
                                className="btn-control"
                                onClick={() => void handleCancelSchedule(msg.id)}
                              >
                                Cancel
                              </button>
                            </>
                          ) : null}
                        </p>
                      ))
                    ) : (
                      <p>No scheduled messages.</p>
                    )}
                  </div>
                  <label>
                    When an agent reaches a limit
                    <select
                      value={routingPolicy}
                      onChange={(event) => {
                        setRoutingDirty(true);
                        setRoutingPolicy(event.target.value);
                      }}
                    >
                      <option value="ask_first">Let me choose</option>
                      <option value="automatic">Automatically switch model or provider</option>
                      <option value="deny_cross_provider">
                        Automatically switch within provider
                      </option>
                    </select>
                  </label>
                  <button
                    type="button"
                    className="btn-control"
                    onClick={() => void handleSaveRouting()}
                  >
                    Save routing policy
                  </button>
                  <p>
                    Switching needs fresh known quota. Send your next message to continue; previous
                    tasks are not replayed.
                  </p>
                  <p role="status">
                    {resetArmed
                      ? 'Reset reconnection armed. Waiting for fresh quota confirmation.'
                      : 'Reset reconnection is not armed.'}
                  </p>
                  {!activeSession?.can_reset ? (
                    <p>
                      This agent does not support native reconnection after reset. Choose another
                      model or start a new conversation.
                    </p>
                  ) : (
                    <button
                      type="button"
                      className="btn-control"
                      onClick={() => void handleResetReconnect(true)}
                    >
                      Reconnect after reset
                    </button>
                  )}
                  <button
                    type="button"
                    className="btn-control"
                    onClick={() => void handleResetReconnect(false)}
                  >
                    Cancel reset reconnection
                  </button>
                  <div aria-label="Recent events">
                    {automationNotices.map((notice, idx) => (
                      <p key={`${idx}-${notice.message}`}>{notice.message}</p>
                    ))}
                  </div>
                </details>
              </>
            )}

            <section
              className="sp-workspace-terminal"
              hidden={!workspaceTerminalOpen}
              aria-label="Workspace terminal"
            >
              <div className="workspace-terminal-heading">
                <span>{workspaceTerminalPath || 'Terminal'}</span>
                <button
                  type="button"
                  className="btn-control"
                  onClick={() => setWorkspaceTerminalOpen(false)}
                >
                  Hide
                </button>
              </div>
              <div ref={workspaceTerminalScreenRef} className="sp-workspace-terminal-screen" />
            </section>

            {deliveryStatus ? <p role="status">{deliveryStatus}</p> : null}
            {hasEarlier ? (
              <div className="ide-chat-controls">
                <button
                  type="button"
                  className="btn-control"
                  onClick={() => void loadChat(activeSessionId, { earlier: true })}
                >
                  Load earlier messages
                </button>
              </div>
            ) : null}
          </section>
        ) : null}
      </div>
    </section>
  );
}
