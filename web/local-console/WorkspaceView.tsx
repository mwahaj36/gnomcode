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
import { createPortal } from 'react-dom';
import {
  copyTextToClipboard,
  createInterruptRequestId,
  formatInterruptAckMessage,
  formatPromptWithAttachments,
  isSessionRunningState,
  parseGitStatusLines,
  renderLiveBadge,
  renderActivityIndicator,
  middleTruncate,
  reconcileChat,
  wslDistro,
  wslFolder,
  renderConversationLimits,
  renderProviderCards,
  renderSessionList,
  SUBAGENT_PREF_KEY,
  subagentsIncluded,
  updateChat,
  hydrateAttachmentThumbs,
  type ConversationLimit,
  type PendingChat,
  type ProjectActivity,
  type ProviderOverview,
} from '../lib/console-presentation.js';
import {
  formatWorktreeBadge,
  renderWorktreeDiffMarkup,
  type WorktreeDiffData,
} from '../lib/worktree-presentation.mjs';
import { collapseChatList, toggleChatList } from './chat-list';
import { agentLabel, toggleChatMeta } from '../lib/console-presentation.js';
import { PaneResizer } from './pane-resizer';
import { markWorkspaceReady, navigateTo } from './shell-bus';
import { SetupPrompt } from './SetupPrompt';
import {
  fetchLocalJson,
  fetchLocalMutation,
  getLocalBearerToken,
  type LocalSessionSummary,
  type LocalSessionsResponse,
  type LocalTerminalInstance,
} from './local-api';
import { AttachmentsBar, SessionTools, useSessionAttachments, type WorkspaceSidePanel } from './WorkspaceTools';

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
const PINNED_SESSIONS_KEY = 'gnomcode.pinnedSessions';
type SidebarQuickFilter = 'all' | 'active' | 'pinned';

function readPinnedSessions(): string[] {
  try {
    const raw = JSON.parse(window.localStorage.getItem(PINNED_SESSIONS_KEY) || '[]');
    return Array.isArray(raw) ? raw.filter((item): item is string => typeof item === 'string') : [];
  } catch {
    return [];
  }
}

function isQuickActiveSession(session: LocalSessionSummary): boolean {
  if (session.home_group === 'needs_you' || session.home_group === 'running') return true;
  if (session.live?.state === 'working' || session.live?.state === 'waiting_for_you') return true;
  return isSessionRunningState(session.state || '');
}

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

/**
 * The automation endpoint answers HTTP 200 even when it refuses a request, with
 * `{ type: 'error', message }`. Treat that as the failure it is, so a refused schedule is
 * never reported as saved.
 */
async function automationCall<T = unknown>(options: { method: 'POST'; body: unknown }): Promise<T> {
  const reply = await fetchLocalMutation<T & { type?: string; message?: string }>('/api/v1/automation', options);
  if (reply && reply.type === 'error') {
    throw new Error(reply.message || 'The request was refused.');
  }
  return reply;
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
  const [providerOverviews, setProviderOverviews] = useState<ProviderOverview[]>([]);
  const [usagePopoverOpen, setUsagePopoverOpen] = useState(false);
  const [allChatsOpen, setAllChatsOpen] = useState(false);
  // All chats "Runs in" filter (ADR 0060): 'all', 'windows' or a distro name. Filters the loaded page.
  const [placeFilter, setPlaceFilter] = useState('all');
  useEffect(() => {
    if (!usagePopoverOpen) return;
    const onKey = (event: globalThis.KeyboardEvent) => {
      if (event.key === 'Escape') setUsagePopoverOpen(false);
    };
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, [usagePopoverOpen]);
  const [catalogError, setCatalogError] = useState(false);

  const [sourceFilter, setSourceFilter] = useState('all');
  const [sidebarQuickFilter, setSidebarQuickFilter] = useState<SidebarQuickFilter>('all');
  const [pinnedSessionIds, setPinnedSessionIds] = useState<string[]>(() => readPinnedSessions());
  const [searchQuery, setSearchQuery] = useState('');
  const [includeSubagents, setIncludeSubagentsState] = useState(() => subagentsIncluded());
  const setIncludeSubagents = useCallback((value: boolean) => {
    setIncludeSubagentsState(value);
    try {
      localStorage.setItem(SUBAGENT_PREF_KEY, String(value));
    } catch {
      // Preference stays in memory when storage is blocked
    }
  }, []);
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
  // "Run in" picker (ADR 0060): '' is Windows, otherwise a WSL distro name.
  const [runIn, setRunIn] = useState('');
  const [wslDistros, setWslDistros] = useState<{ name: string; label: string; running: boolean; claude_hook?: string }[]>([]);
  const [pathCopied, setPathCopied] = useState(false);
  const [integrateStatus, setIntegrateStatus] = useState('');
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
  const chatEventsRef = useRef(chatEvents);
  useEffect(() => { chatEventsRef.current = chatEvents; }, [chatEvents]);
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
  const [workspaceTerminalError, setWorkspaceTerminalError] = useState<string | null>(null);
  const [confirmDeleteOpen, setConfirmDeleteOpen] = useState(false);
  const [selectedModelValue, setSelectedModelValue] = useState('');
  const [modelHandoffBusy, setModelHandoffBusy] = useState(false);
  const [sendBusy, setSendBusy] = useState(false);
  const [interruptBusy, setInterruptBusy] = useState(false);
  const [activeChangesCount, setActiveChangesCount] = useState(0);
  const [resumeBusy, setResumeBusy] = useState(false);
  const [continueHereBusy, setContinueHereBusy] = useState(false);
  // 'automation' is Workspace-only; the other panels belong to SessionTools.
  const [sidePanel, setSidePanel] =
    useState<WorkspaceSidePanel | 'automation' | 'agent-terminal' | 'shell' | null>(null);
  const [artifactLink, setArtifactLink] = useState('');
  const sidePanelRef = useRef(sidePanel);
  sidePanelRef.current = sidePanel;
  const [overviewOpen, setOverviewOpenState] = useState(false);
  const overviewOpenRef = useRef(overviewOpen);
  overviewOpenRef.current = overviewOpen;
  const setOverviewOpen = useCallback((open: boolean) => {
    setOverviewOpenState(open);
  }, []);
  const openTool = (panel: WorkspaceSidePanel | 'automation') => {
    setOverviewOpenState(false);
    setSidePanel(panel);
  };
  const [queuedPrompts, setQueuedPrompts] = useState<
    Array<{ id: string; text: string; status: string; created_at?: string; error?: string | null }>
  >([]);
  const [editingQueueId, setEditingQueueId] = useState<string | null>(null);
  const [editingQueueText, setEditingQueueText] = useState('');
  const [sessionContextMenu, setSessionContextMenu] = useState<{
    session: LocalSessionSummary;
    x: number;
    y: number;
  } | null>(null);
  const longPressTimerRef = useRef<number | null>(null);

  // Automation state
  const [scheduledMessages, setScheduledMessages] = useState<ScheduledMessage[]>([]);
  const [resetArmed, setResetArmed] = useState(false);
  const [routingPolicy, setRoutingPolicy] = useState('ask_first');
  const [routingDirty, setRoutingDirty] = useState(false);
  const [automationNotices, setAutomationNotices] = useState<AutomationNotice[]>([]);
  const [scheduleTime, setScheduleTime] = useState('');
  const [scheduleStatus, setScheduleStatus] = useState('');
  // Scheduled messages are an opt-in feature (off until the person turns them on).
  const [schedulingEnabled, setSchedulingEnabled] = useState<boolean | null>(null);
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
  const interruptAttemptRef = useRef<{ sessionId: string; requestId: string } | null>(null);
  const toggleAgentTerminalRef = useRef<() => void>(() => {});
  const activeCopyPathRef = useRef('');
  const attachInputRef = useRef<HTMLInputElement | null>(null);
  const attachments = useSessionAttachments(activeSessionId);

  const activeSession = useMemo(
    () =>
      sessions.find((item) => item.id === activeSessionId) ??
      selectedCache[activeSessionId] ??
      null,
    [sessions, selectedCache, activeSessionId],
  );
  const activeLive = activeSession?.live || null;
  // A protocol host reports its own turn; its transcript may not be ingested yet.
  const [limitReached, setLimitReached] = useState(false);
  const [hostBusy, setHostBusy] = useState<boolean | null>(null);
  const [liveTick, setLiveTick] = useState(() => Date.now());
  useEffect(() => {
    if (activeLive?.state !== 'working') return;
    const timer = window.setInterval(() => setLiveTick(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [activeLive?.state]);
  const activityMarkup = renderActivityIndicator(activeLive, activeSession?.turn_started_at, liveTick);
  const activeHosted = useMemo(
    () =>
      hostedSessions.find((item) => item.session_id === activeSessionId) ??
      // A conversation is usually selected by its agent-native id, while the host keeps the
      // id Gnomcode launched it under; the row carries the link between the two.
      hostedSessions.find((item) => item.session_id === activeSession?.host_id) ??
      null,
    [hostedSessions, activeSessionId, activeSession?.host_id],
  );

  const toggleChatSidebar = toggleChatList;

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
      const [catalogResult, overviewResult] = await Promise.allSettled([
        fetchLocalJson<{ providers?: CatalogProvider[] }>('/api/v1/model-catalog'),
        fetchLocalJson<ProviderOverview[]>('/api/v1/providers/overview'),
      ]);
      if (catalogResult.status === 'fulfilled') {
        setProviders(Array.isArray(catalogResult.value?.providers) ? catalogResult.value.providers : []);
        setCatalogError(false);
      } else {
        setProviders([]);
        setCatalogError(true);
      }
      if (overviewResult.status === 'fulfilled') {
        setProviderOverviews(Array.isArray(overviewResult.value) ? overviewResult.value : []);
      }
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
        automationCall<{ messages?: ScheduledMessage[]; reset_armed?: boolean }>({
          method: 'POST',
          body: { type: 'schedules', session_id: sessionId },
        }),
        automationCall<{ policy?: string }>({
          method: 'POST',
          body: { type: 'routing' },
        }),
        automationCall<{ notices?: AutomationNotice[] }>({
          method: 'POST',
          body: { type: 'notices' },
        }),
      ]);
      if (activeSessionIdRef.current !== sessionId) return;
      void fetchLocalJson<{ scheduled_messages_enabled?: boolean }>('/api/v1/settings/features')
        .then((features) => setSchedulingEnabled(Boolean(features?.scheduled_messages_enabled)))
        .catch(() => undefined);
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
      // This runs on every poll: only clear its own failure note, never a schedule result.
      setScheduleStatus((current) => (current.startsWith('Schedule status unavailable') ? '' : current));
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
        const currentEvents = earlier ? chatEventsRef.current : [];
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
    [closeAgentTerminal, loadChatModels, refreshAutomation, resizePrompt],
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
      if (window.matchMedia('(max-width: 899px)').matches) collapseChatList();
      setAllChatsOpen(false);
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
        // A WSL folder opens with its distro selected and its Linux path.
        const distro = wslDistro(folder);
        const linux = distro
          ? '/' + folder.replace(/\\/g, '/').split('/').filter(Boolean).slice(2).join('/')
          : '';
        setRunIn(distro || rememberedRunIn(folder));
        setLaunchRepo(distro ? linux : folder);
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

  const togglePinSession = useCallback((sessionId: string) => {
    setPinnedSessionIds((prev) => {
      const next = prev.includes(sessionId)
        ? prev.filter((id) => id !== sessionId)
        : [sessionId, ...prev];
      try {
        localStorage.setItem(PINNED_SESSIONS_KEY, JSON.stringify(next));
      } catch {
        // Storage unavailable
      }
      return next;
    });
  }, []);

  const visibleSidebarSessions = useMemo(() => {
    if (sidebarQuickFilter === 'active') {
      return sessions.filter(isQuickActiveSession);
    }
    if (sidebarQuickFilter === 'pinned') {
      return sessions.filter((item) => pinnedSessionIds.includes(item.id));
    }
    if (!pinnedSessionIds.length) return sessions;
    const pinnedSet = new Set(pinnedSessionIds);
    return [
      ...sessions.filter((item) => pinnedSet.has(item.id)),
      ...sessions.filter((item) => !pinnedSet.has(item.id)),
    ];
  }, [sessions, sidebarQuickFilter, pinnedSessionIds]);

  // Sync rendered conversation list via shared pure helper
  useEffect(() => {
    if (!sessionListRef.current) return;
    if (sidebarError) {
      sessionListRef.current.textContent = sidebarError;
      return;
    }
    if (visibleSidebarSessions.length === 0 && sidebarQuickFilter !== 'all') {
      const empty = document.createElement('p');
      empty.className = 'local-view-message';
      empty.textContent =
        sidebarQuickFilter === 'pinned'
          ? 'No pinned conversations yet. Use More actions on a conversation to pin it.'
          : 'No active conversations right now.';
      sessionListRef.current.replaceChildren(empty);
      return;
    }
    const host = sessionListRef.current;
    const projects = new Map<string, { label: string; sessions: typeof visibleSidebarSessions }>();
    for (const session of visibleSidebarSessions) {
      const key = session.repo_path || session.project_name || '';
      const label = session.project_name || session.repo_path?.split(/[\\/]/).filter(Boolean).at(-1) || 'Other conversations';
      const project = projects.get(key) ?? { label, sessions: [] };
      project.sessions.push(session);
      projects.set(key, project);
    }
    const previous = new Map(Array.from(host.querySelectorAll<HTMLDetailsElement>('details[data-project]'), (group) => [group.dataset.project, group.open]));
    const fragment = document.createDocumentFragment();
    for (const [key, project] of projects) {
      const group = document.createElement('details');
      group.className = 'sp-project-conversations';
      group.dataset.project = key;
      group.open = project.sessions.some((session) => session.id === activeSessionId) || (previous.get(key) ?? previous.size === 0);
      const heading = document.createElement('summary');
      heading.textContent = project.label;
      heading.title = key || project.label;
      const rows = document.createElement('div');
      rows.innerHTML = renderSessionList(project.sessions, activeSessionId);
      group.append(heading, rows);
      fragment.append(group);
    }
    if (!projects.size) {
      host.innerHTML = renderSessionList([], activeSessionId);
      return;
    }
    const focus = document.activeElement;
    const action = focus instanceof HTMLElement && host.contains(focus) ? focus.dataset : null;
    const top = host.scrollTop;
    host.replaceChildren(fragment);
    if (action?.arg && action.action) {
      Array.from(host.querySelectorAll<HTMLElement>('[data-action][data-arg]'))
        .find((element) => element.dataset.arg === action.arg && element.dataset.action === action.action)
        ?.focus({ preventScroll: true });
    }
    host.scrollTop = top;
  }, [visibleSidebarSessions, sidebarQuickFilter, activeSessionId, sidebarError]);

  // Sync reconciled chat cards via shared pure helper
  useEffect(() => {
    if (!chatStreamRef.current || !activeSessionId) return;
    const sessionPending = pendingBySession[activeSessionId] || [];
    const result = reconcileChat(chatEvents, sessionPending, limitReached ? 'error' : activeSession?.state || '');
    if (limitReached) result.events = result.events.filter(event=>!(event as {waiting?:boolean}).waiting);
    updateChat(chatStreamRef.current, result.events);
    const repo = activeSession?.repo_path;
    hydrateAttachmentThumbs(chatStreamRef.current, (path) => {
      if (path.startsWith('/api/v1/')) {
        return fetchLocalJson(path);
      }
      if (!repo) {
        return Promise.reject(new Error('Workspace repository path unavailable'));
      }
      return fetchLocalJson(`/api/v1/repos/file?repo=${encodeURIComponent(repo)}&path=${encodeURIComponent(path)}`);
    });
  }, [activeSessionId, chatEvents, pendingBySession, activeSession?.state, activeSession?.repo_path, limitReached]);

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
      // After a cross-provider switch the host reports no model (CLI default) or a display name.
      allCatalogModels.find(
        (item) =>
          item.agent === currentCliAgent &&
          (currentSessionModel
            ? item.model === currentSessionModel || item.display_name === currentSessionModel
            : item.model === '@default'),
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

    // A poll never starts while its previous one is still running. Each load keeps only the
    // newest response, so with a large history (a request slower than the interval) every
    // answer used to arrive already superseded and the list never rendered at all.
    const running = new Set<string>();
    const poll = (key: string, load: () => Promise<unknown>) => {
      if (running.has(key)) return;
      running.add(key);
      void load().finally(() => running.delete(key));
    };
    // The list is rebuilt from the whole history on each request, so it refreshes every third
    // tick; the open conversation stays on every tick.
    let tick = 0;
    const timer = window.setInterval(() => {
      if (!isWorkspaceVisible()) return;
      if (tick++ % 3 === 0) poll('conversations', loadConversations);
      const id = activeSessionIdRef.current;
      if (id) {
        poll('automation', () => refreshAutomation(id));
        poll('chat', () => loadChat(id, { silent: true }));
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
      } else if (detail.type === 'toggle-agent-terminal') {
        toggleAgentTerminalRef.current();
      } else if (detail.type === 'toggle-overview') {
        if (sidePanelRef.current) {
          setSidePanel(null);
          setOverviewOpen(false);
        } else {
          setOverviewOpen(!overviewOpenRef.current);
        }
      } else if (detail.type === 'copy-workspace-path') {
        const targetPath = activeCopyPathRef.current;
        if (targetPath) {
          void copyTextToClipboard(targetPath).then((ok) =>
            setActionStatus(
              ok
                ? 'Workspace path copied.'
                : 'Could not copy the path. Check clipboard access and retry.',
            ),
          );
        }
      }
    };

    window.addEventListener('gnomcode:view-change', onViewChange);
    window.addEventListener('gnomcode:reload-view', onReload);
    window.addEventListener('gnomcode:workspace-action', onWorkspaceAction);
    markWorkspaceReady();
    return () => {
      window.removeEventListener('gnomcode:view-change', onViewChange);
      window.removeEventListener('gnomcode:reload-view', onReload);
      window.removeEventListener('gnomcode:workspace-action', onWorkspaceAction);
    };
  }, [
    loadChat,
    loadConversations,
    loadInstalledAgents,
    loadProjects,
    openLauncher,
    setOverviewOpen,
    toggleChatSidebar,
  ]);

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
    if (!sessionId || !chatTerminalRef.current) return;
    // Reopening the pane builds a fresh terminal, so any earlier stream is replaced rather
    // than left to close later and strand the new one without a socket.
    if (chatSocketRef.current) {
      const previous = chatSocketRef.current;
      chatSocketRef.current = null;
      previous.close();
    }
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

  const closeRightPane = () => {
    if (terminalStreamOpen) {
      setTerminalStreamOpen(false);
      closeAgentTerminal();
    }
    setWorkspaceTerminalOpen(false);
    setSidePanel(null);
  };

  const handleToggleAgentTerminal = () => {
    if (!activeSessionId) return;
    if (sidePanel === 'agent-terminal') {
      setTerminalStreamOpen(false);
      closeAgentTerminal();
      setSidePanel(null);
      return;
    }
    // Opening the pane mounts the terminal host; the effect below attaches to it once it is
    // on screen, which a callback here cannot rely on.
    setSidePanel('agent-terminal');
    setTerminalStreamOpen(true);
  };
  toggleAgentTerminalRef.current = handleToggleAgentTerminal;

  // The stream attaches only once the pane holding the terminal is actually mounted.
  useEffect(() => {
    if (sidePanel !== 'agent-terminal' || !activeSessionId) return;
    connectAgentTerminal(activeSessionId);
  }, [sidePanel, activeSessionId, connectAgentTerminal]);

  const interruptActiveSession = useCallback(async () => {
    const sessionId = activeSessionIdRef.current;
    if (!sessionId || interruptBusy) return;
    if (!interruptAttemptRef.current || interruptAttemptRef.current.sessionId !== sessionId) {
      interruptAttemptRef.current = { sessionId, requestId: createInterruptRequestId() };
    }
    setInterruptBusy(true);
    setActionStatus('Sending interrupt signal (Esc)...');
    try {
      const ack = await fetchLocalMutation<{ type?: string; message?: string; sent?: boolean }>(
        `/api/v1/sessions/${encodeURIComponent(sessionId)}/interrupt`,
        { method: 'POST', body: { request_id: interruptAttemptRef.current.requestId } },
      );
      if (ack?.type === 'error') {
        throw new Error(ack.message || 'Could not deliver interrupt. Retry or check Agent terminal.');
      }
      interruptAttemptRef.current = null;
      if (activeSessionIdRef.current === sessionId) {
        setActionStatus(formatInterruptAckMessage(ack));
      }
    } catch (err) {
      if (activeSessionIdRef.current === sessionId) {
        setActionStatus(
          err instanceof Error ? err.message : 'Interrupt failed. Retry or check Agent terminal.',
        );
      }
    } finally {
      setInterruptBusy(false);
    }
  }, [interruptBusy]);

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

  useEffect(() => {
    if (!launcherOpen) return;
    let live = true;
    fetchLocalJson<{ distros?: { name: string; label: string; running: boolean; claude_hook?: string }[] }>('/api/v1/wsl/distros')
      .then((data) => {
        if (live) setWslDistros(Array.isArray(data?.distros) ? data.distros : []);
      })
      .catch(() => {
        if (live) setWslDistros([]);
      });
    return () => {
      live = false;
    };
  }, [launcherOpen]);

  const resolvedLaunchPath = runIn ? wslFolder(runIn, launchRepo) : launchRepo.trim();

  const copyLaunchPath = async () => {
    const ok = await copyTextToClipboard(resolvedLaunchPath);
    setPathCopied(ok);
    if (ok) window.setTimeout(() => setPathCopied(false), 1500);
  };

  const integrateInWsl = async (distro: string) => {
    setIntegrateStatus(`Installing the Claude hook in ${distro}...`);
    try {
      const result = await fetchLocalMutation<{ ok?: boolean; message?: string }>('/api/v1/wsl/integrate', {
        method: 'POST',
        body: { distro },
      });
      setIntegrateStatus(result?.message || 'Done.');
      if (result?.ok) {
        setWslDistros((list) => list.map((d) => (d.name === distro ? { ...d, claude_hook: 'integrated' } : d)));
      }
    } catch (error) {
      setIntegrateStatus(`Could not integrate: ${error instanceof Error ? error.message : 'request failed'}`);
    }
  };

  const handleLaunchSubmit = async (event: FormEvent) => {
    event.preventDefault();
    if (runIn && !resolvedLaunchPath) {
      setLaunchStatus(`Enter a Linux folder in ${runIn}, for example /home/you/project.`);
      return;
    }
    const repoPath = resolvedLaunchPath;
    if (!launchAgent || !repoPath) {
      setLaunchStatus('Please provide an agent and a workspace directory.');
      return;
    }
    setLaunchBusy(true);
    setLaunchStatus(runIn ? `Launching ${launchAgent} in WSL: ${runIn}...` : `Launching ${launchAgent}...`);
    try {
      const record = await fetchLocalMutation<{ session_id: string; starting?: string }>('/api/v1/sessions/launch', {
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
      rememberRunIn(launchRepo.trim(), runIn);
      setLaunchStatus(
        record.starting
          ? `${record.starting} Session ${record.session_id} is opening.`
          : `Session ${record.session_id} launched! Opening conversation...`,
      );
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
    if (!source || workspaceTerminalBusyRef.current) return;
    workspaceTerminalBusyRef.current = true;
    setWorkspaceTerminalError(null);
    setSidePanel('shell');
    try {
      // The screen element lives in the pane that just opened; wait for React to mount it.
      for (let frame = 0; frame < 60 && !workspaceTerminalScreenRef.current; frame += 1) {
        await new Promise((done) => window.requestAnimationFrame(() => done(null)));
      }
      if (!workspaceTerminalScreenRef.current || source !== activeSessionIdRef.current) return;
      let entry = workspaceTerminalsRef.current.get(source);
      if (entry && (entry.socket.readyState === WebSocket.CLOSING || entry.socket.readyState === WebSocket.CLOSED)) {
        entry.observer.disconnect();
        entry.terminal.dispose();
        entry.element.remove();
        workspaceTerminalsRef.current.delete(source);
        entry = undefined;
      }
      if (!entry) {
        if (!window.Terminal || !window.FitAddon) {
          setWorkspaceTerminalError('Terminal renderer unavailable.');
          return;
        }
        const record = await fetchLocalMutation<{ session_id: string; repo_path: string }>(
          `/api/v1/sessions/${encodeURIComponent(source)}/terminal`,
          { method: 'POST' },
        );
        const container = source === activeSessionIdRef.current ? workspaceTerminalScreenRef.current : null;
        container?.querySelectorAll('[data-disconnected]').forEach((node) => node.remove());
        const element = document.createElement('div');
        container?.append(element);
        const colors = getComputedStyle(document.body);
        const terminal = new window.Terminal({
          cols: 80,
          rows: 12,
          scrollback: 3000,
          fontSize: 13,
          fontFamily: 'Consolas, monospace',
          theme: { background: colors.getPropertyValue('--sp-surface').trim(), foreground: colors.getPropertyValue('--sp-text').trim() },
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
        if (container) observer.observe(container);
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
      const currentHost = workspaceTerminalScreenRef.current;
      if (source !== activeSessionIdRef.current || !currentHost) return;
      if (entry.element.parentElement !== currentHost) {
        currentHost.append(entry.element);
      }
      entry.observer.disconnect();
      entry.observer.observe(currentHost);
      Array.from(currentHost.children).forEach((node) => {
        if (node instanceof HTMLElement) {
          node.hidden = node !== entry?.element;
        }
      });
      setWorkspaceTerminalPath(entry.repoPath);
      setWorkspaceTerminalOpen(true);
      window.requestAnimationFrame(() => {
        entry?.resize();
        entry?.terminal.focus();
      });
    } catch (error) {
      if (source !== activeSessionIdRef.current) return;
      const message =
        error instanceof Error && error.message
          ? error.message
          : 'Could not open a shell in this workspace. Check that the directory still exists.';
      setWorkspaceTerminalPath('');
      setWorkspaceTerminalError(message);
      setWorkspaceTerminalOpen(false);
    } finally {
      workspaceTerminalBusyRef.current = false;
    }
  };

  useEffect(() => {
    if (sidePanel !== 'shell' || !workspaceTerminalOpen || !activeSessionId) return;
    const entry = workspaceTerminalsRef.current.get(activeSessionId);
    if (!entry) return;
    const frame = window.requestAnimationFrame(() => {
      entry.resize();
      entry.terminal.focus();
    });
    return () => window.cancelAnimationFrame(frame);
  }, [sidePanel, workspaceTerminalOpen, activeSessionId]);

  const loadQueue = useCallback(async (sessionId: string) => {
    if (!sessionId) {
      setQueuedPrompts([]);
      return;
    }
    try {
      const data = await fetchLocalJson<{
        items?: Array<{ id: string; text: string; status: string; created_at?: string; error?: string | null }>;
      }>(`/api/v1/sessions/${encodeURIComponent(sessionId)}/queue`);
      if (activeSessionIdRef.current === sessionId) {
        setQueuedPrompts(Array.isArray(data?.items) ? data.items : []);
      }
    } catch {
      // Queue endpoint unavailable for non-managed or closed sessions
    }
  }, []);

  useEffect(() => {
    setSidePanel(null);
    // The pane that held the terminal is gone with the conversation it belonged to; leaving
    // the flag set would make the next Agent terminal click open an empty box.
    setTerminalStreamOpen(false);
    closeAgentTerminal();
    setWorkspaceTerminalOpen(false);
    setWorkspaceTerminalError(null);
    setHostBusy(null);
    setLimitReached(false);
    setEditingQueueId(null);
    setEditingQueueText('');
    void loadQueue(activeSessionId);
  }, [activeSessionId, closeAgentTerminal, loadQueue]);

  const sendPromptText = async (rawText: string, fromComposer: boolean) => {
    if (fromComposer && attachments.blocked()) return;
    const sessionId = activeSessionId;
    const draft = rawText.trim();
    const ready = fromComposer ? attachments.items.filter((item) => item.status === 'saved' && item.workspace_path) : [];
    const text = formatPromptWithAttachments(draft, ready);
    if (!text || !sessionId || pendingSendRef.current.has(sessionId)) return;

    const generation = deletionGenRef.current[sessionId] || 0;
    const requestId = createUuid();
    const optimistic: PendingChat = {
      id: requestId,
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
    setDeliveryStatus('Sending...');

    try {
      const res = await fetchLocalMutation<{
        queued?: boolean;
        status?: string;
        delivery_state?: string;
      }>(`/api/v1/sessions/${encodeURIComponent(sessionId)}/prompt`, {
        method: 'POST',
        body: { text, id: requestId, queue_if_busy: true },
      });
      if ((deletionGenRef.current[sessionId] || 0) !== generation) return;

      if (res?.queued) {
        setPendingBySession((prev) => ({
          ...prev,
          [sessionId]: (prev[sessionId] || []).filter((item) => item.id !== optimistic.id),
        }));
        if (sessionId === activeSessionIdRef.current) {
          setDeliveryStatus('Queued for next safe turn boundary.');
          void loadQueue(sessionId);
        }
      } else {
        setPendingBySession((prev) => ({
          ...prev,
          [sessionId]: (prev[sessionId] || []).map((item) =>
            item.id === optimistic.id ? { ...item, delivered: true } : item,
          ),
        }));
        if (sessionId === activeSessionIdRef.current) {
          setDeliveryStatus('Sent.');
        }
      }

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
        const uncertain = /timeout|abort|network|502|503|504|uncertain/i.test(msg);
        setDeliveryStatus(
          uncertain
            ? `Delivery uncertain. Check conversation before retrying. (${msg})`
            : `Failed: ${msg}`,
        );
      }
    } finally {
      pendingSendRef.current.delete(sessionId);
      setSendBusy(pendingSendRef.current.has(activeSessionIdRef.current));
    }
  };

  const handleSaveQueuedPrompt = async (msgId: string) => {
    const clean = editingQueueText.trim();
    if (!activeSessionId || !clean) return;
    try {
      await fetchLocalMutation(
        `/api/v1/sessions/${encodeURIComponent(activeSessionId)}/queue/${encodeURIComponent(msgId)}`,
        { method: 'PATCH', body: { text: clean } },
      );
      setEditingQueueId(null);
      setEditingQueueText('');
      await loadQueue(activeSessionId);
    } catch (error) {
      setDeliveryStatus(error instanceof Error ? error.message : 'Could not update queued message.');
    }
  };

  const handleCancelQueuedPrompt = async (msgId: string) => {
    if (!activeSessionId) return;
    try {
      await fetchLocalMutation(
        `/api/v1/sessions/${encodeURIComponent(activeSessionId)}/queue/${encodeURIComponent(msgId)}`,
        { method: 'DELETE' },
      );
      if (editingQueueId === msgId) {
        setEditingQueueId(null);
        setEditingQueueText('');
      }
      await loadQueue(activeSessionId);
    } catch (error) {
      setDeliveryStatus(error instanceof Error ? error.message : 'Could not cancel queued message.');
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

  const [scheduleText, setScheduleText] = useState('');
  const handleScheduleDraft = async () => {
    if (schedulingBusy || !activeSessionId) return;
    const sessionId = activeSessionId;
    const text = scheduleText.trim();
    const dueMs = new Date(scheduleTime).getTime();
    if (!text || !Number.isFinite(dueMs) || dueMs <= Date.now()) {
      setScheduleStatus(!text ? 'Write the message to send.' : 'Pick a time in the future.');
      return;
    }
    setSchedulingBusy(true);
    try {
      await automationCall({
        method: 'POST',
        body: {
          type: 'schedule',
          session_id: sessionId,
          id: createUuid(),
          text,
          due_at: Math.floor(dueMs / 1000),
        },
      });
      setScheduleText('');
      setScheduleStatus('Scheduled. It sends at that time while Gnomcode is running.');
      await refreshAutomation(sessionId);
    } catch (error) {
      const msg = error instanceof Error ? error.message : 'Schedule error';
      setScheduleStatus(`Not scheduled. ${msg}`);
    } finally {
      setSchedulingBusy(false);
    }
  };

  const handleEnableScheduling = async () => {
    try {
      const current = await fetchLocalJson<Record<string, boolean>>('/api/v1/settings/features');
      await fetchLocalMutation('/api/v1/settings/features', {
        method: 'POST',
        body: { ...current, scheduled_messages_enabled: true },
      });
      setSchedulingEnabled(true);
      setScheduleStatus('');
    } catch (error) {
      setScheduleStatus(error instanceof Error ? error.message : 'Could not turn on scheduled messages.');
    }
  };

  const handleCancelSchedule = async (id: string) => {
    if (!activeSessionId) return;
    try {
      await automationCall({
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
      await automationCall({
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
      await automationCall({
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

  const openSessionMenuForId = useCallback(
    (sessionId: string, x: number, y: number) => {
      const match = sessions.find((item) => item.id === sessionId);
      if (!match) return;
      const clampedX = Math.max(8, Math.min(x, (typeof window !== 'undefined' ? window.innerWidth : 800) - 210));
      const clampedY = Math.max(8, Math.min(y, (typeof window !== 'undefined' ? window.innerHeight : 600) - 220));
      setSessionContextMenu({ session: match, x: clampedX, y: clampedY });
    },
    [sessions],
  );

  useEffect(() => {
    if (!sessionContextMenu) return;
    const onPointerDown = (event: MouseEvent) => {
      const target = event.target as HTMLElement | null;
      if (!target?.closest('.sp-session-context-menu')) setSessionContextMenu(null);
    };
    const onKeyDown = (event: globalThis.KeyboardEvent) => {
      if (event.key === 'Escape') setSessionContextMenu(null);
    };
    window.addEventListener('mousedown', onPointerDown);
    window.addEventListener('keydown', onKeyDown);
    return () => {
      window.removeEventListener('mousedown', onPointerDown);
      window.removeEventListener('keydown', onKeyDown);
    };
  }, [sessionContextMenu]);

  const handleSessionListClick = (event: React.MouseEvent<HTMLDivElement>) => {
    const allChatsBtn = (event.target as HTMLElement).closest<HTMLElement>(
      '[data-action="open-all-chats"]',
    );
    if (allChatsBtn) {
      setAllChatsOpen(true);
      return;
    }
    const menuBtn = (event.target as HTMLElement).closest<HTMLElement>('[data-action="session-menu"]');
    if (menuBtn?.dataset.arg) {
      event.stopPropagation();
      const rect = menuBtn.getBoundingClientRect();
      openSessionMenuForId(menuBtn.dataset.arg, rect.right - 180, rect.bottom + 4);
      return;
    }
    const target = (event.target as HTMLElement).closest<HTMLElement>('[data-action="load-chat"]');
    const sessionId = target?.dataset.arg;
    if (!sessionId) return;
    setSessionContextMenu(null);
    setAllChatsOpen(false);
    void loadChat(sessionId);
    if (window.matchMedia('(max-width: 899px)').matches) {
      collapseChatList();
    }
  };

  const handleSessionListContextMenu = (event: React.MouseEvent<HTMLDivElement>) => {
    const row = (event.target as HTMLElement).closest<HTMLElement>('[data-session-id]');
    const sessionId = row?.dataset.sessionId;
    if (!sessionId) return;
    event.preventDefault();
    openSessionMenuForId(sessionId, event.clientX, event.clientY);
  };

  const handleSessionListKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if ((event.shiftKey && event.key === 'F10') || event.key === 'ContextMenu') {
      const row = (event.target as HTMLElement).closest<HTMLElement>('[data-session-id]');
      const sessionId = row?.dataset.sessionId;
      if (!sessionId) return;
      event.preventDefault();
      const rect = row.getBoundingClientRect();
      openSessionMenuForId(sessionId, rect.left + 24, rect.bottom + 4);
    }
  };

  const handleChatStreamClick = (event: React.MouseEvent<HTMLDivElement>) => {
    const fileLink = (event.target as HTMLElement).closest<HTMLElement>('[data-workspace-file]');
    if (fileLink) {
      event.preventDefault();
      setArtifactLink(fileLink.dataset.workspaceFile || '');
      setSidePanel('artifacts');
      return;
    }
    toggleChatMeta(event.target);
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

  // The daemon marks a desktop-app or IDE session without a live Gnomcode host as `imported`.
  const imported = activeSession?.source === 'imported';
  const working = !limitReached && (activeLive?.state === 'working' || hostBusy === true);
  const canInterruptActive =
    !imported &&
    (activeHosted !== null || (activeSession as { hosted?: boolean } | null)?.hosted !== false) &&
    (working || isSessionRunningState(activeSession?.state || ''));
  activeCopyPathRef.current = activeHosted?.repo_path || activeSession?.repo_path || copyPath || '';

  useEffect(() => {
    interruptAttemptRef.current = null;
  }, [activeSessionId, canInterruptActive]);

  useEffect(() => {
    const repo = activeHosted?.repo_path || activeSession?.repo_path;
    if (!activeSessionId || !repo) {
      setActiveChangesCount(0);
      return;
    }
    let cancelled = false;
    const timer = window.setTimeout(() => {
      void fetchLocalJson<{ status?: string }>(
        `/api/v1/sessions/${encodeURIComponent(activeSessionId)}/changes`,
      ).then(
        (data) => {
          if (!cancelled) {
            setActiveChangesCount(parseGitStatusLines(data?.status || '').length);
          }
        },
        () => {
          if (!cancelled) setActiveChangesCount(0);
        },
      );
    }, 180);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [
    activeSessionId,
    activeHosted?.repo_path,
    activeSession?.repo_path,
    activeLive?.state,
    chatEvents.length,
  ]);

  useEffect(() => {
    if (!activeSessionId || launcherOpen || terminalStreamOpen || imported) return;
    if (typeof window === 'undefined' || window.innerWidth <= 900) return;
    const frame = window.requestAnimationFrame(() => {
      const activeEl = document.activeElement;
      const insideSidebarList = Boolean(activeEl && activeEl.closest('.sp-workspace-list'));
      const safeTarget = !activeEl || activeEl === document.body || insideSidebarList;
      if (safeTarget) {
        promptInputRef.current?.focus({ preventScroll: true });
      }
    });
    return () => window.cancelAnimationFrame(frame);
  }, [activeSessionId, launcherOpen, terminalStreamOpen, imported]);
  const headerLimits = useMemo(() => {
    if (activeSession?.header_limit !== undefined) {
      return activeSession.header_limit ? [activeSession.header_limit as ConversationLimit] : [];
    }
    return Array.isArray(activeSession?.limits)
      ? (activeSession.limits as ConversationLimit[]).slice(0, 1)
      : [];
  }, [activeSession?.header_limit, activeSession?.limits]);
  const limitsMarkup = useMemo(
    () => renderConversationLimits(headerLimits),
    [headerLimits],
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

  const [historySlot, setHistorySlot] = useState<HTMLElement | null>(null);
  useEffect(() => {
    setHistorySlot(document.getElementById('console-nav-sessions'));
  }, []);

  const usageTriggerRef = useRef<HTMLButtonElement | null>(null);
  const toolsTriggerRef = useRef<HTMLButtonElement | null>(null);

  useEffect(() => {
    if (!overviewOpen) return;
    const dismiss = (event: PointerEvent) => {
      const target = event.target as HTMLElement;
      if (!target.closest('.sp-overview') && !toolsTriggerRef.current?.contains(target)) setOverviewOpen(false);
    };
    const escape = (event: globalThis.KeyboardEvent) => {
      if (event.key !== 'Escape') return;
      setOverviewOpen(false);
      toolsTriggerRef.current?.focus();
    };
    document.addEventListener('pointerdown', dismiss);
    document.addEventListener('keydown', escape);
    return () => {
      document.removeEventListener('pointerdown', dismiss);
      document.removeEventListener('keydown', escape);
    };
  }, [overviewOpen, setOverviewOpen]);

  useEffect(() => {
    const onKeyDown = (event: globalThis.KeyboardEvent) => {
      if (event.key !== 'Escape') return;
      if (sidePanel) {
        event.preventDefault();
        closeRightPane();
        toolsTriggerRef.current?.focus();
      } else if (usagePopoverOpen) {
        event.preventDefault();
        setUsagePopoverOpen(false);
        usageTriggerRef.current?.focus();
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [sidePanel, usagePopoverOpen]);

  const sidebarNode = (
    <aside id="ide-session-sidebar" className="ide-sidebar" aria-label="Session history">
        <div className="ide-sidebar-title">
          <h2>Conversations</h2>
          <div className="ide-sidebar-actions">
            <button
              className="btn-control sp-sidebar-new-btn"
              type="button"
              onClick={() => openLauncher(true)}
            >
              <svg className="gc-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg>New
            </button>
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
        </div>

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

        <div className="sp-sidebar-quick-pills" role="group" aria-label="Quick conversation filters">
          {(
            [
              ['all', 'All'],
              ['active', 'Active'],
              ['pinned', 'Pinned'],
            ] as const
          ).map(([key, label]) => (
            <button
              key={key}
              type="button"
              className="sp-sidebar-quick-pill"
              aria-pressed={sidebarQuickFilter === key}
              onClick={() => {
                setPage(1);
                setSidebarQuickFilter(key);
              }}
            >
              {label}
            </button>
          ))}
        </div>

        <details className="conversation-filters">
          <summary>Filters</summary>
          <div className="filters">
            <label>
              History{' '}
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
            <label className="subagent-filter sp-all-chats-toggle">
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
          onContextMenu={handleSessionListContextMenu}
          onKeyDown={handleSessionListKeyDown}
          onTouchStart={(event) => {
            const row = (event.target as HTMLElement).closest<HTMLElement>('[data-session-id]');
            const sessionId = row?.dataset.sessionId;
            if (!sessionId || !event.touches[0]) return;
            const touch = event.touches[0];
            if (longPressTimerRef.current) window.clearTimeout(longPressTimerRef.current);
            longPressTimerRef.current = window.setTimeout(() => {
              openSessionMenuForId(sessionId, touch.clientX, touch.clientY);
            }, 500);
          }}
          onTouchEnd={() => {
            if (longPressTimerRef.current) {
              window.clearTimeout(longPressTimerRef.current);
              longPressTimerRef.current = null;
            }
          }}
          onTouchMove={() => {
            if (longPressTimerRef.current) {
              window.clearTimeout(longPressTimerRef.current);
              longPressTimerRef.current = null;
            }
          }}
        >
          Loading conversations...
        </div>
        {sessionContextMenu ? (
          <div
            className="sp-session-context-menu"
            role="menu"
            aria-label="Conversation actions"
            style={{ top: `${sessionContextMenu.y}px`, left: `${sessionContextMenu.x}px` }}
          >
            <button
              type="button"
              role="menuitem"
              onClick={() => {
                const target = sessionContextMenu.session.id;
                setSessionContextMenu(null);
                setAllChatsOpen(false);
                void loadChat(target);
              }}
            >
              Open conversation
            </button>
            <button
              type="button"
              role="menuitem"
              onClick={() => {
                const target = sessionContextMenu.session.id;
                setSessionContextMenu(null);
                togglePinSession(target);
              }}
            >
              {pinnedSessionIds.includes(sessionContextMenu.session.id)
                ? 'Unpin conversation'
                : 'Pin conversation'}
            </button>
            {sessionContextMenu.session.source === 'imported' &&
            sessionContextMenu.session.can_continue_here ? (
              <button
                type="button"
                role="menuitem"
                onClick={async () => {
                  const target = sessionContextMenu.session.id;
                  setSessionContextMenu(null);
                  await loadChat(target);
                  void fetchLocalMutation(`/api/v1/sessions/${encodeURIComponent(target)}/continue-here`, {
                    method: 'POST',
                  }).then(() => loadConversations()).then(() => loadChat(target));
                }}
              >
                Continue here
              </button>
            ) : null}
            <button
              type="button"
              role="menuitem"
              onClick={() => {
                const target = sessionContextMenu.session.id;
                setSessionContextMenu(null);
                void fetchLocalMutation(`/api/v1/sessions/${encodeURIComponent(target)}/focus`, {
                  method: 'POST',
                });
              }}
            >
              Focus agent window
            </button>
            <button
              type="button"
              role="menuitem"
              className="sp-menu-danger"
              onClick={() => {
                const { id: target, title } = sessionContextMenu.session;
                setSessionContextMenu(null);
                // Deleting cannot be undone, so it always asks first.
                const sure = window.confirm(
                  `Delete "${title || 'this conversation'}" from Gnomcode?\n\nThis stops the agent if Gnomcode is running it and erases Gnomcode's copy of the chat. The agent's own history files are not touched.`,
                );
                if (!sure) return;
                void fetchLocalMutation(`/api/v1/sessions/${encodeURIComponent(target)}`, { method: 'DELETE' })
                  .then(() => {
                    if (activeSessionIdRef.current === target) window.location.hash = '#sessions';
                    return loadConversations();
                  })
                  .catch((error: unknown) =>
                    setDeliveryStatus(error instanceof Error ? error.message : 'Could not delete the conversation.'),
                  );
              }}
            >
              Delete conversation
            </button>
            {sessionContextMenu.session.repo_path ? (
              <button
                type="button"
                role="menuitem"
                onClick={() => {
                  const path = sessionContextMenu.session.repo_path || '';
                  setSessionContextMenu(null);
                  void copyTextToClipboard(path);
                }}
              >
                Copy workspace path
              </button>
            ) : null}
          </div>
        ) : null}

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
        <button
          type="button"
          className="btn-control sp-all-chats-trigger"
          aria-pressed={allChatsOpen}
          onClick={() => {
            setAllChatsOpen((open) => !open);
            if (window.matchMedia('(max-width: 899px)').matches) collapseChatList();
          }}
        >
          {allChatsOpen ? 'Back to chat' : 'All chats'}
        </button>
      </aside>
  );

  const providerBadgeLabel = (name: string): string => {
    const lower = name.toLowerCase();
    const match = providerOverviews.find(
      (item) =>
        item.display_name?.toLowerCase() === lower ||
        item.provider?.toLowerCase() === lower ||
        (lower.includes('claude') && item.provider === 'claude-code') ||
        (lower.includes('codex') && item.provider === 'codex') ||
        (lower.includes('antigravity') && item.provider === 'antigravity'),
    );
    if (!match) return '';
    if (match.state === 'exhausted') return ' (Out)';
    const hw = match.headline_window as { remaining_pct?: number } | null | undefined;
    const rem = hw?.remaining_pct;
    return Number.isFinite(rem) ? ` (${Math.round(rem as number)}% left)` : '';
  };

  return (
    <section className="ide-layout" aria-label="Agent workspace">
      {historySlot ? createPortal(sidebarNode, historySlot) : null}

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

        {allChatsOpen ? (
          <section className="sp-all-chats-pane" aria-label="All chats">
            <header className="sp-all-chats-head">
              <div>
                <h2>All chats</h2>
                <p className="sp-all-chats-meta">{`${totalCount} conversations`}</p>
              </div>
              <button
                type="button"
                className="btn-control"
                onClick={() => setAllChatsOpen(false)}
              >
                Back to chat
              </button>
            </header>
            <div className="sp-all-chats-toolbar">
              <input
                type="search"
                aria-label="Search all chats"
                placeholder="Search all chats"
                value={searchQuery}
                onChange={(event) => {
                  setPage(1);
                  setSearchQuery(event.target.value);
                }}
              />
              <div className="sp-all-chats-chips" role="group" aria-label="Filter by provider">
                {[
                  { id: 'all', label: 'All' },
                  { id: 'claude-code', label: 'Claude Code' },
                  { id: 'codex', label: 'Codex' },
                  { id: 'antigravity', label: 'Antigravity' },
                ].map((chip) => (
                  <button
                    key={chip.id}
                    type="button"
                    className={`sp-chip sp-filter-chip${agentFilter === chip.id ? ' is-active' : ''}`}
                    aria-pressed={agentFilter === chip.id}
                    onClick={() => {
                      setPage(1);
                      setAgentFilter(chip.id);
                    }}
                  >
                    {chip.label}
                  </button>
                ))}
                <button
                  type="button"
                  className={`sp-chip sp-filter-chip${statusFilter === 'active' ? ' is-active' : ''}`}
                  aria-pressed={statusFilter === 'active'}
                  onClick={() => {
                    setPage(1);
                    setStatusFilter((cur) => (cur === 'active' ? 'all' : 'active'));
                  }}
                >
                  Running
                </button>
              </div>
              {placeChips(sessions).length > 1 ? (
                <div className="sp-all-chats-chips" role="group" aria-label="Filter by where it runs">
                  {[{ id: 'all', label: 'Anywhere' }, ...placeChips(sessions)].map((chip) => (
                    <button
                      key={chip.id}
                      type="button"
                      className={`sp-chip sp-filter-chip${placeFilter === chip.id ? ' is-active' : ''}`}
                      aria-pressed={placeFilter === chip.id}
                      onClick={() => setPlaceFilter(chip.id)}
                    >
                      {chip.label}
                    </button>
                  ))}
                </div>
              ) : null}
              <label className="sp-all-chats-toggle">
                <input
                  type="checkbox"
                  checked={includeSubagents}
                  onChange={(event) => {
                    setPage(1);
                    setIncludeSubagents(event.target.checked);
                  }}
                />
                Include subagents
              </label>
            </div>
            <div className="sp-all-chats-list">
              {sessions.length ? (
                sessions.filter((item) => matchesPlace(item.repo_path, placeFilter)).map((item) => {
                  const distro = wslDistro(item.repo_path);
                  const tokens =
                    item.total_tokens ?? (item.input_tokens ?? 0) + (item.output_tokens ?? 0);
                  const tokenText =
                    tokens > 0
                      ? tokens >= 1000
                        ? `${(tokens / 1000).toFixed(1)}K tokens`
                        : `${tokens} tokens`
                      : '';
                  const costText =
                    item.cost_basis &&
                    item.cost_basis !== 'unknown' &&
                    typeof item.total_cost_usd === 'number'
                      ? `$${item.total_cost_usd.toFixed(2)} ${
                          { estimated: 'EST', exact: 'EXACT', api: 'API' }[item.cost_basis] ?? 'EST'
                        }`
                      : '';
                  const meta = [
                    item.project_name || item.agent,
                    item.model,
                    item.state_label ?? item.state,
                    tokenText,
                    costText,
                  ]
                    .filter(Boolean)
                    .join(' · ');
                  return (
                    <button
                      key={item.id}
                      type="button"
                      className="sp-session-row sp-all-chat-row"
                      aria-current={item.id === activeSessionId}
                      onClick={() => {
                        setAllChatsOpen(false);
                        void loadChat(item.id);
                      }}
                    >
                      <span className="sp-session-main">
                        <strong>{item.title || item.project_name || 'Untitled session'}</strong>
                        <span>
                          {distro ? (
                            <span className="sp-badge sp-wsl-badge" title={item.repo_path}>{`WSL: ${distro}`}</span>
                          ) : null}
                          {meta}
                        </span>
                      </span>
                    </button>
                  );
                })
              ) : (
                <p className="sp-empty">No matching conversations.</p>
              )}
            </div>
          </section>
        ) : null}

        {!allChatsOpen && !launcherOpen && !activeSessionId ? (
          <div className="sp-empty sp-workspace-empty">
            <p>Pick a conversation or start a new one.</p>
            <button type="button" className="btn-primary" onClick={() => openLauncher(true)}>
              New session
            </button>
          </div>
        ) : null}

        {!allChatsOpen && launcherOpen ? (
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
              {wslDistros.length ? (
                <div>
                  <label>
                    Run in
                    <select
                      className="sp-run-in"
                      value={runIn}
                      onChange={(event) => {
                        setRunIn(event.target.value);
                        setLaunchRepo('');
                      }}
                    >
                      <option value="">Windows</option>
                      {wslDistros.map((distro) => (
                        <option key={distro.name} value={distro.name}>
                          {distro.label}
                          {distro.running ? '' : ' (starts on launch)'}
                        </option>
                      ))}
                    </select>
                  </label>
                </div>
              ) : null}
              <div>
                <label>
                  {runIn ? `Linux folder in ${runIn}` : 'Workspace directory'}
                  <div className="workspace-field">
                    <input
                      type="text"
                      placeholder={runIn ? '/home/you/project' : 'Choose a folder or enter its path'}
                      list={runIn ? 'sp-wsl-recent' : undefined}
                      value={launchRepo}
                      onChange={(event) => setLaunchRepo(event.target.value)}
                      required
                    />
                    {runIn ? null : (
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
                    )}
                  </div>
                </label>
                {runIn && launchAgent === 'claude' && wslDistros.find((d) => d.name === runIn)?.claude_hook === 'missing' ? (
                  <div className="sp-field-note sp-wsl-hook" role="note">
                    <span>
                      Approvals and limits from Claude in {runIn} need the Gnomcode hook there. Sessions work without it.
                    </span>
                    <button type="button" className="btn-control" onClick={() => void integrateInWsl(runIn)}>
                      Integrate in WSL
                    </button>
                  </div>
                ) : null}
                {runIn && integrateStatus ? (
                  <p className="sp-field-note" role="status">
                    {integrateStatus}
                  </p>
                ) : null}
                {runIn ? (
                  <datalist id="sp-wsl-recent">
                    {recentWslFolders([...sessions, ...projects.map((p) => ({ repo_path: (p as { repo_root?: string }).repo_root }))], runIn).map((path) => (
                      <option key={path} value={path} />
                    ))}
                  </datalist>
                ) : null}
                {runIn && resolvedLaunchPath ? (
                  <div className="sp-path-line">
                    <code title={resolvedLaunchPath}>{middleTruncate(resolvedLaunchPath, 52)}</code>
                    <button
                      type="button"
                      className="btn-control sp-path-copy"
                      onClick={() => void copyLaunchPath()}
                      aria-label="Copy full path"
                    >
                      {pathCopied ? 'Copied' : 'Copy'}
                    </button>
                  </div>
                ) : null}
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

        {!allChatsOpen && activeSessionId ? (
          <section className="sp-workspace-chat-panel" aria-label="Conversation">
            <div className="conversation-heading">
              <div>
                <h2>
                  {activeSession?.title || activeHosted?.title || 'New conversation'}
                </h2>
                <p>
                  {activeSession?.surface_label ||
                    agentLabel(activeSession?.agent || activeHosted?.agent || '')}
                  {imported ? ' · Observed' : ''}
                  {activeLive ? (
                    <span
                      className="sp-conversation-live"
                      role="status"
                      dangerouslySetInnerHTML={{ __html: renderLiveBadge(activeLive) }}
                    />
                  ) : null}
                </p>
              </div>
              <div className="sp-limits-popover-wrap">
                <button
                  ref={usageTriggerRef}
                  type="button"
                  className="sp-limits-popover-trigger"
                  aria-expanded={usagePopoverOpen}
                  onClick={() => setUsagePopoverOpen((open) => !open)}
                >
                  <span className="visually-hidden">Remaining limits: </span>
                  <span
                    className="sp-conversation-limits"
                    dangerouslySetInnerHTML={{ __html: limitsMarkup }}
                  />
                  <span className="sp-limits-view-link">Usage<svg className="gc-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="m6 9 6 6 6-6" /></svg></span>
                </button>
                {usagePopoverOpen ? (
                  <div className="sp-usage-popover" role="region" aria-label="All provider usage">
                    <div className="sp-usage-popover-head">
                      <strong>Provider usage</strong>
                      <button
                        type="button"
                        className="sp-link"
                        onClick={() => {
                          setUsagePopoverOpen(false);
                          navigateTo('usage');
                        }}
                      >
                        View Usage<svg className="gc-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M7 17 17 7M8 7h9v9" /></svg>
                      </button>
                    </div>
                    <div
                      className="sp-usage-popover-body"
                      dangerouslySetInnerHTML={{
                        __html: renderProviderCards(providerOverviews, { cost: false }),
                      }}
                    />
                  </div>
                ) : null}
              </div>
              {activeSession?.can_resume && !imported ? (
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
                  ref={toolsTriggerRef}
                  type="button"
                  className="btn-control sp-overview-toggle"
                  title="Open workspace tools"
                  aria-pressed={overviewOpen || Boolean(sidePanel)}
                  onClick={() => {
                    if (sidePanel) {
                      closeRightPane();
                      setOverviewOpen(false);
                    } else setOverviewOpen(!overviewOpen);
                  }}
                >
                  <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true">
                    <path d="M3 4h18v16H3z M15 4v16" />
                  </svg>
                  <span className="sp-tools-label">Tools</span>
                  {activeChangesCount > 0 ? (
                    <span className="sp-overview-badge" aria-hidden="true">
                      {activeChangesCount}
                    </span>
                  ) : null}
                </button>
                {canInterruptActive ? (
                  <button
                    type="button"
                    className="btn-control sp-interrupt-btn"
                    aria-label="Interrupt running turn"
                    title="Sends Esc to interrupt the active turn without closing the session"
                    disabled={interruptBusy}
                    onClick={() => void interruptActiveSession()}
                  >
                    Stop
                  </button>
                ) : null}
              <details
                className="conversation-menu"
                onToggle={(event) => {
                  // Keep the menu inside the pane: anchor it to the left when the header has wrapped.
                  const menu = event.currentTarget.querySelector<HTMLElement>(':scope > div');
                  const pane = event.currentTarget.closest('.sp-workspace-chat-panel');
                  if (!menu || !pane || !event.currentTarget.open) return;
                  menu.style.left = '';
                  menu.style.right = '';
                  if (menu.getBoundingClientRect().left < pane.getBoundingClientRect().left) {
                    menu.style.left = '0';
                    menu.style.right = 'auto';
                  }
                }}
              >
                <summary aria-label="Conversation actions" title="More actions"><svg className="gc-icon" viewBox="0 0 24 24" aria-hidden="true"><circle cx="5" cy="12" r="1.4" /><circle cx="12" cy="12" r="1.4" /><circle cx="19" cy="12" r="1.4" /></svg></summary>
                <div>
                  {imported && activeSession?.can_continue_here ? (
                    <button
                      type="button"
                      className="btn-control"
                      disabled={continueHereBusy}
                      onClick={() => void handleContinueHere()}
                    >
                      {continueHereBusy ? 'Continuing...' : 'Continue here'}
                    </button>
                  ) : null}
                  <button
                    type="button"
                    className="btn-control"
                    onClick={() => void handleFocusChat()}
                  >
                    {activeSession?.agent === 'antigravity'
                      ? 'Open in Antigravity'
                      : 'Focus agent window'}
                  </button>
                  <button
                    type="button"
                    className="btn-control"
                    onClick={() => void handleOpenWorkspaceTerminal()}
                  >
                    Terminal
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
                      className="btn-control btn-danger"
                      onClick={() => setConfirmDeleteOpen(true)}
                    >
                      Delete local chat history
                    </button>
                  ) : null}
                </div>
              </details>
            </div>

            <div className={`sp-workspace-body${sidePanel ? ' has-side-panel' : ''}`}>
              <div className="sp-workspace-main-col">
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
                    <code
                      className="sp-worktree-path"
                      title={activeSession?.worktree_path || activeHosted?.worktree_path}
                    >
                      {middleTruncate(activeSession?.worktree_path || activeHosted?.worktree_path, 56)}
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
                    className="btn-primary btn-destructive"
                    onClick={() => void handleDeleteChatConfirmed()}
                  >
                    Delete history
                  </button>
                </div>
              </div>
            ) : null}

            {chatStatus ? <p className="sp-chat-status" data-message-count={/^\d+ messages?$/.test(chatStatus) || undefined} role="status">{chatStatus}</p> : null}
            {actionStatus ? <p role="status">{actionStatus}</p> : null}
            {copyPath ? (
              <button
                type="button"
                className="btn-control"
                onClick={() => {
                  void copyTextToClipboard(copyPath).then((ok) =>
                    setActionStatus(
                      ok
                        ? 'Workspace path copied.'
                        : 'Could not copy the path. Check clipboard access and retry.',
                    ),
                  );
                }}
              >
                Copy workspace path
              </button>
            ) : null}

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
            <div
              ref={chatStreamRef}
              className="sp-chat-stream-box"
              role="log"
              aria-label="Conversation"
              aria-live="polite"
              tabIndex={0}
              onClick={handleChatStreamClick}
            />
            {working ? (
              <div className="sp-activity-inline" role="status" aria-label="Agent activity">
                <span dangerouslySetInnerHTML={{ __html: activityMarkup }} />
                {canInterruptActive ? (
                  <button
                    type="button"
                    className="btn-control sp-activity-stop"
                    disabled={interruptBusy}
                    onClick={() => void interruptActiveSession()}
                  >
                    Stop
                  </button>
                ) : null}
              </div>
            ) : null}
            {!imported && activeSessionId ? (
              <SetupPrompt
                sessionId={activeSessionId}
                onOpenTerminal={() => {
                  if (sidePanel !== 'agent-terminal') handleToggleAgentTerminal();
                }}
                onStatus={(status) => { setHostBusy(status.busy ?? null); setLimitReached(!!status.limit_message); }}
              />
            ) : null}

            {queuedPrompts.length > 0 && !imported ? (
              <div className="sp-queued-list" role="region" aria-label="Queued messages">
                <p className="sp-queued-head">
                  {`${queuedPrompts.length} queued ${queuedPrompts.length === 1 ? 'message' : 'messages'} · sends on next safe turn`}
                </p>
                {queuedPrompts.map((item) => (
                  <div key={item.id} className="sp-queued-item">
                    {editingQueueId === item.id ? (
                      <div className="sp-queued-edit">
                        <textarea
                          rows={2}
                          aria-label="Edit queued message"
                          value={editingQueueText}
                          onChange={(event) => setEditingQueueText(event.target.value)}
                        />
                        <div className="sp-queued-actions">
                          <button
                            type="button"
                            className="btn-primary"
                            onClick={() => void handleSaveQueuedPrompt(item.id)}
                          >
                            Save
                          </button>
                          <button
                            type="button"
                            className="btn-control"
                            onClick={() => {
                              setEditingQueueId(null);
                              setEditingQueueText('');
                            }}
                          >
                            Cancel
                          </button>
                        </div>
                      </div>
                    ) : (
                      <>
                        <span className="sp-queued-text">{item.text}</span>
                        <div className="sp-queued-actions">
                          <button
                            type="button"
                            className="btn-control"
                            onClick={() => {
                              setEditingQueueId(item.id);
                              setEditingQueueText(item.text);
                            }}
                          >
                            Edit
                          </button>
                          <button
                            type="button"
                            className="btn-control"
                            onClick={() => void handleCancelQueuedPrompt(item.id)}
                          >
                            Remove
                          </button>
                        </div>
                      </>
                    )}
                  </div>
                ))}
              </div>
            ) : null}

            {imported ? (
              <div className="sp-readonly-banner" role="region" aria-label="Observed session actions">
                <p>
                  {activeSession?.can_continue_here
                    ? `Observed in ${importedAgentLabel}. Continue this CLI session in Gnomcode to send messages.`
                    : `Observed in ${importedAgentLabel}. External IDE or app sessions are read-only here.`}
                </p>
                <div className="sp-imported-actions">
                  {activeSession?.can_continue_here ? (
                    <button
                      type="button"
                      className="btn-primary"
                      disabled={continueHereBusy}
                      onClick={() => void handleContinueHere()}
                    >
                      {continueHereBusy ? 'Continuing...' : 'Continue here'}
                    </button>
                  ) : (
                    <button
                      type="button"
                      className="btn-control"
                      onClick={() => void handleFocusChat()}
                    >
                      {activeSession?.agent === 'antigravity'
                        ? 'Open in Antigravity'
                        : 'Focus agent window'}
                    </button>
                  )}
                  <button
                    type="button"
                    className="btn-control"
                    onClick={() => openLauncher()}
                  >
                    New conversation
                  </button>
                </div>
              </div>
            ) : (
              <>
                <form className="sp-chat-composer" onSubmit={handleSendPrompt}>
                  <div className="sp-composer-editor">
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
                    placeholder={
                      activeLive?.state === 'working'
                        ? 'Queue a message for when the agent finishes...'
                        : 'Message the agent...'
                    }
                    value={currentDraft}
                    onChange={(event) => {
                      const next = event.target.value;
                      setDraftsBySession((prev) => ({ ...prev, [activeSessionId]: next }));
                      resizePrompt();
                    }}
                    onKeyDown={handlePromptKeyDown}
                  />
                  </div>
                  <div className="composer-toolbar">
                    <div className="composer-model">
                      <select
                        aria-label="Model"
                        disabled={!hasCatalogModels || modelHandoffBusy}
                        value={selectedModelValue}
                        onChange={(event) => void handleModelSwitch(event.target.value)}
                      >
                        <option value="" disabled>
                          {catalogError ? 'Model catalog unavailable' : 'Agent default'}
                        </option>
                        {providers.map((provider) => (
                          <optgroup
                            key={provider.name}
                            label={`${provider.name}${providerBadgeLabel(provider.name)}`}
                          >
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
                    {deliveryStatus ? (
                      <span className="sp-composer-status" role="status">
                        {deliveryStatus}
                      </span>
                    ) : null}
                    <button type="button" className="btn-control" onClick={() => attachInputRef.current?.click()}>
                      Attach
                    </button>
                    <button type="submit" className="btn-primary" disabled={sendBusy || attachments.blocked()}>
                      {working ? 'Queue' : 'Send'}
                    </button>
                  </div>
                  <small className="sp-chat-composer-help">
                    Enter to send · Shift+Enter for a new line
                  </small>
                </form>
              </>
            )}

              </div>
              {!sidePanel && overviewOpen ? (
                <aside className="sp-overview" aria-label="Conversation tools">
                  <div className="sp-overview-head">
                    <span>Overview</span>
                    <button
                      type="button"
                      className="btn-control sp-icon-btn"
                      aria-label="Hide overview"
                      onClick={() => {
                        setOverviewOpen(false);
                        toolsTriggerRef.current?.focus();
                      }}
                    >
                      <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true">
                        <path d="M6 6l12 12M18 6 6 18" />
                      </svg>
                    </button>
                  </div>
                  <button type="button" className="sp-overview-item" onClick={() => openTool('changes')}>
                    <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6 3v12m0 0a3 3 0 1 0 0 6 3 3 0 0 0 0-6Zm12-6a3 3 0 1 0 0-6 3 3 0 0 0 0 6Zm0 0c0 6-12 3-12 9" /></svg>
                    <span>Changes</span>
                    {activeChangesCount > 0 ? (
                      <span className="sp-overview-badge" aria-hidden="true">
                        {activeChangesCount}
                      </span>
                    ) : null}
                  </button>
                  <button type="button" className="sp-overview-item" onClick={() => openTool('files')}>
                    <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6 3h8l4 4v14H6z M14 3v4h4" /></svg>
                    Files
                  </button>
                  <button type="button" className="sp-overview-item" onClick={() => openTool('artifacts')}>
                    <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 4h7v7H4zM13 4h7v7h-7zM4 13h7v7H4zM13 13h7v7h-7z" /></svg>
                    Artifacts
                  </button>
                  {!imported ? (
                    <button type="button" className="sp-overview-item" aria-pressed={terminalStreamOpen} onClick={handleToggleAgentTerminal}>
                      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M3 4h18v16H3z m4 5 3 3-3 3m5 0h5" /></svg>
                      {terminalStreamOpen ? 'Show conversation' : 'Agent terminal'}
                    </button>
                  ) : null}
                  <button type="button" className="sp-overview-item" aria-pressed={workspaceTerminalOpen} onClick={() => void handleOpenWorkspaceTerminal()}>
                    <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M3 4h18v16H3z M7 15h10" /></svg>
                    Workspace terminal
                  </button>
                  <div className="sp-overview-sep" />
                  <button type="button" className="sp-overview-item sp-overview-sub" onClick={() => openTool('automation')}>
                    Schedule and automation
                  </button>
                </aside>
              ) : null}
              {sidePanel ? (
                  <aside className="sp-inspector" aria-label="Workspace tools">
                    <PaneResizer cssVar="--inspector-w" label="Resize tools panel" min={360} max={960} initial={560} direction={-1} maxRatio={0.55} />
                    <div className="sp-workspace-modal-head">
                      <div
                        className="sp-header-panel-toggles"
                        role="tablist"
                        aria-label="Inspector views"
                        onKeyDown={(event) => {
                          const tabs = Array.from(
                            event.currentTarget.querySelectorAll<HTMLButtonElement>('[role="tab"]'),
                          );
                          const current = tabs.indexOf(document.activeElement as HTMLButtonElement);
                          if (current === -1 || tabs.length === 0) return;
                          let next = -1;
                          if (event.key === 'ArrowRight') next = (current + 1) % tabs.length;
                          else if (event.key === 'ArrowLeft') next = (current - 1 + tabs.length) % tabs.length;
                          else if (event.key === 'Home') next = 0;
                          else if (event.key === 'End') next = tabs.length - 1;
                          if (next === -1) return;
                          event.preventDefault();
                          tabs[next].focus();
                        }}
                      >
                        {(
                          [
                            ['files', 'Files'],
                            ['changes', 'Changes'],
                            ['artifacts', 'Artifacts'],
                            ['automation', 'Automation'],
                            ...((imported ? [] : [['agent-terminal', 'Agent terminal']]) as [string, string][]),
                            ['shell', 'Terminal'],
                          ] as const
                        ).map(([tab, label]) => (
                          <button
                            key={tab}
                            type="button"
                            role="tab"
                            id={`workspace-tool-${tab}`}
                            aria-controls="workspace-inspector-content"
                            aria-selected={sidePanel === tab}
                            tabIndex={sidePanel === tab ? 0 : -1}
                            className={`btn-control${sidePanel === tab ? ' is-active' : ''}`}
                            onClick={() => {
                              if (tab === 'agent-terminal') {
                                if (sidePanel !== 'agent-terminal') handleToggleAgentTerminal();
                                return;
                              }
                              if (tab === 'shell') {
                                void handleOpenWorkspaceTerminal();
                                return;
                              }
                              setSidePanel(tab as WorkspaceSidePanel | 'automation');
                            }}
                          >
                            {label}
                          </button>
                        ))}
                      </div>
                      <button
                        type="button"
                        className="btn-control sp-workspace-modal-close"
                        aria-label="Close workspace inspector"
                        onClick={() => {
                          closeRightPane();
                          toolsTriggerRef.current?.focus();
                        }}
                      >
                        Close
                      </button>
                    </div>
                    <div className="sp-workspace-modal-body" id="workspace-inspector-content" role="tabpanel" aria-labelledby={`workspace-tool-${sidePanel}`}>
                      <div
                        ref={chatTerminalRef}
                        className="sp-chat-terminal-box"
                        role="log"
                        aria-label="Agent terminal"
                        tabIndex={0}
                        hidden={sidePanel !== 'agent-terminal'}
                      />
                      <section
                        className="sp-workspace-terminal"
                        hidden={sidePanel !== 'shell'}
                        aria-label="Workspace terminal"
                      >
                        <div className="workspace-terminal-heading">
                          <span>{workspaceTerminalPath || 'Terminal'}</span>
                        </div>
                        {workspaceTerminalError ? (
                          <p className="sp-panel-error" role="status" style={{ padding: '12px' }}>
                            {workspaceTerminalError}
                          </p>
                        ) : null}
                        <div
                          ref={workspaceTerminalScreenRef}
                          className="sp-workspace-terminal-screen"
                          hidden={Boolean(workspaceTerminalError)}
                        />
                      </section>
                      {sidePanel === 'agent-terminal' || sidePanel === 'shell' ? null : sidePanel === 'automation' ? (
                        <section className="sp-automation-panel" aria-label="Schedule and automation">
                        <div className="sp-auto">
                          <section className="sp-auto-section" aria-labelledby="auto-later">
                            <h4 id="auto-later">Send later</h4>
                            <p className="sp-auto-help">
                              Sends this message to the agent at the time you pick. Gnomcode must be running and this computer awake. The saved
                              text is redacted.
                            </p>
                            {schedulingEnabled === false ? (
                              <div className="sp-auto-row">
                                <p className="sp-auto-help">Scheduled messages are off. Turn them on to send messages later from this computer.</p>
                                <button type="button" className="btn-control" onClick={() => void handleEnableScheduling()}>
                                  Turn on scheduled messages
                                </button>
                              </div>
                            ) : (
                              <>
                            <label className="sp-auto-field">
                              <span>Message</span>
                              <textarea
                                rows={3}
                                value={scheduleText}
                                placeholder="What should the agent do then?"
                                onChange={(event) => setScheduleText(event.target.value)}
                              />
                            </label>
                            <div className="sp-auto-row">
                              <label className="sp-auto-field">
                                <span>Send at (your local time)</span>
                                <input type="datetime-local" value={scheduleTime} onChange={(event) => setScheduleTime(event.target.value)} />
                              </label>
                              <button type="button" className="btn-control" disabled={schedulingBusy} onClick={() => void handleScheduleDraft()}>
                                Schedule
                              </button>
                            </div>
                              </>
                            )}
                            <div role="status">
                              {scheduleStatus ? <p className="sp-auto-help">{scheduleStatus}</p> : null}
                            </div>
                            {scheduledMessages.length ? (
                              <ul className="sp-auto-list" aria-label="Scheduled messages">
                                {scheduledMessages.map((msg) => (
                                  <li key={msg.id}>
                                    <span className="sp-auto-when">{msg.due_at > 0 ? new Date(msg.due_at * 1000).toLocaleString() : 'Next turn'}</span>
                                    <span className="sp-chip sp-chip--neutral">{msg.status}</span>
                                    <span className="sp-auto-text">{msg.text ? msg.text.slice(0, 120) : ''}</span>
                                    {msg.status === 'pending' ? (
                                      <button type="button" className="btn-control" onClick={() => void handleCancelSchedule(msg.id)}>
                                        Cancel
                                      </button>
                                    ) : null}
                                  </li>
                                ))}
                              </ul>
                            ) : (
                              <p className="sp-auto-help">Nothing scheduled.</p>
                            )}
                          </section>
      
                          <section className="sp-auto-section" aria-labelledby="auto-limit">
                            <h4 id="auto-limit">When this agent reaches a limit</h4>
                            <div className="sp-auto-choices" role="radiogroup" aria-labelledby="auto-limit">
                              {(
                                [
                                  ['ask_first', 'Ask me', 'Show the best alternative and wait for you to pick.'],
                                  ['automatic', 'Switch automatically', 'Hand off to the model or provider with the most capacity left.'],
                                  ['deny_cross_provider', 'Switch within this provider', 'Only move to another model from the same provider.'],
                                ] as const
                              ).map(([value, label, help]) => (
                                <label key={value} className="sp-auto-choice">
                                  <input
                                    type="radio"
                                    name="routing-policy"
                                    value={value}
                                    checked={routingPolicy === value}
                                    onChange={() => {
                                      setRoutingDirty(true);
                                      setRoutingPolicy(value);
                                    }}
                                  />
                                  <span>
                                    <b>{label}</b>
                                    <small>{help}</small>
                                  </span>
                                </label>
                              ))}
                            </div>
                            <div className="sp-auto-row">
                              <p className="sp-auto-help">Switching needs a fresh limit reading. Earlier tasks are not replayed; your next message continues.</p>
                              <button type="button" className="btn-control" onClick={() => void handleSaveRouting()}>
                                Save
                              </button>
                            </div>
                          </section>
      
                          <section className="sp-auto-section" aria-labelledby="auto-reset">
                            <h4 id="auto-reset">After the limit resets</h4>
                            {!activeSession?.can_reset ? (
                              <p className="sp-auto-help">
                                This agent cannot reconnect on its own after a reset. Pick another model or start a new conversation.
                              </p>
                            ) : (
                              <div className="sp-auto-row">
                                <p className="sp-auto-help" role="status">
                                  {resetArmed
                                    ? 'Armed: Gnomcode reconnects this conversation once a fresh reading confirms the reset.'
                                    : 'Off: this conversation waits for you after a reset.'}
                                </p>
                                <button type="button" className="btn-control" onClick={() => void handleResetReconnect(!resetArmed)}>
                                  {resetArmed ? 'Turn off' : 'Reconnect after reset'}
                                </button>
                              </div>
                            )}
                          </section>
      
                          {automationNotices.length ? (
                            <section className="sp-auto-section" aria-labelledby="auto-events">
                              <h4 id="auto-events">Recent events</h4>
                              <ul className="sp-auto-list" aria-label="Recent events">
                                {automationNotices.map((notice, idx) => (
                                  <li key={`${idx}-${notice.message}`}>
                                    <span className="sp-auto-text">{notice.message}</span>
                                  </li>
                                ))}
                              </ul>
                            </section>
                          ) : null}
                        </div>
                        </section>
                      ) : (
                        <SessionTools
                          sessionId={activeSessionId}
                          artifactLink={artifactLink}
                          repoPath={activeHosted?.repo_path || activeSession?.repo_path || copyPath}
                          canInterrupt={canInterruptActive}
                          panel={sidePanel}
                          onPanelChange={setSidePanel}
                          hideToolbar
                        />
                      )}
                    </div>
                  </aside>
              ) : null}
            </div>
          </section>
        ) : null}
      </div>
    </section>
  );
}

const RUN_IN_KEY = 'gnomcode.runIn';

function readRunIn(): Record<string, string> {
  try {
    const value = JSON.parse(window.localStorage.getItem(RUN_IN_KEY) || '{}');
    return value && typeof value === 'object' ? value : {};
  } catch {
    return {};
  }
}

/** Last "Run in" choice for a folder (per project), '' for Windows. */
function rememberedRunIn(folder: string): string {
  return readRunIn()[folder.trim().toLowerCase()] || '';
}

function rememberRunIn(folder: string, runIn: string) {
  try {
    const all = readRunIn();
    all[folder.toLowerCase()] = runIn;
    window.localStorage.setItem(RUN_IN_KEY, JSON.stringify(all));
  } catch {
    // Per-viewer convenience only.
  }
}

/** Linux folders of recent sessions in one distro, newest first. */
function recentWslFolders(sessions: { repo_path?: string }[], distro: string): string[] {
  const out: string[] = [];
  for (const session of sessions) {
    const path = session.repo_path || '';
    if (wslDistro(path).toLowerCase() !== distro.toLowerCase()) continue;
    const linux = '/' + path.replace(/\\/g, '/').split('/').filter(Boolean).slice(2).join('/');
    if (!out.includes(linux)) out.push(linux);
    if (out.length >= 8) break;
  }
  return out;
}

/** Where-it-runs chips for the loaded sessions: Windows plus each distro seen. */
function placeChips(sessions: { repo_path?: string }[]): { id: string; label: string }[] {
  const distros = [...new Set(sessions.map((s) => wslDistro(s.repo_path)).filter(Boolean))];
  return distros.length ? [{ id: 'windows', label: 'Windows' }, ...distros.map((d) => ({ id: d, label: `WSL: ${d}` }))] : [];
}

function matchesPlace(repoPath: string | undefined, place: string): boolean {
  if (place === 'all') return true;
  const distro = wslDistro(repoPath);
  return place === 'windows' ? !distro : distro === place;
}
