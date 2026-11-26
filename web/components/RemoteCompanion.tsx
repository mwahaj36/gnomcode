'use client';

import { approvalCopy, sessionStateLabel, toggleChatMeta } from "../lib/console-presentation.js";
import { useEffect, useRef, useState } from 'react';
import { remoteRequestParts } from '../lib/remote-upload.mjs';
import { createChannel, generateKeyPair } from '../lib/remote-crypto.mjs';
import RemoteAgentTerminal, {type TerminalOutput} from './RemoteAgentTerminal';
import DesktopDevices from './DesktopDevices';
import ModelChoices, {type ModelChoice, type ProviderCatalog} from './ModelChoices';
import { WorkspaceFileTree } from './WorkspaceFileTree';
import { PaneResizer } from '../local-console/pane-resizer';
import {supabase} from '../lib/supabase';
import {deviceProof,forgetDevice,rememberDevice,savedDevice,type DeviceOffer} from '../lib/remote-device';
import { artifactForLink, parentWorkspacePath, reconcileChat, renderChangesView, renderConversationLimitCard, renderConversationLimits, renderProviderCards, type ProviderOverview, cleanSessionTitle, type ConversationLimit, type PendingChat, updateChat, hydrateAttachmentThumbs, bindPreviewInteractions, normalizeWorkspaceRelPath, renderFileViewer, type RepoFileData, type RepoFileEntry, type RepoFileListing, classifyArtifactKind, renderArtifactPreview, type ArtifactItem, formatPromptWithAttachments, type AttachmentItem, validateAttachmentCandidate, createInterruptRequestId, formatInterruptAckMessage, isSessionRunningState, renderLiveBadge, renderActivityIndicator, type LiveStatus, middleTruncate, wslFolder } from '../lib/console-presentation';
import { formatWorktreeBadge, renderWorktreeDiffMarkup, type WorktreeDiffData } from '../lib/worktree-presentation.mjs';
import { disableDeviceNotifications, getNotificationPermission, requestNotificationPermission, sendAppNotification, subscribeToPushServer, triggerServerPushTest } from '../lib/notifications';

type NotificationPreferences = { approval_waiting:boolean;turn_finished:boolean;turn_finished_min_minutes:number;limit_low:boolean;limit_reset:boolean;session_stopped:boolean;needs_attention:boolean;managed_only:boolean };
const NOTIFICATION_CHOICES = [['managed_only','Only Gnomcode-managed conversations'],['approval_waiting','Needs approval'],['needs_attention','Agent errors'],['session_stopped','Session stopped'],['turn_finished','Turn finished after long work'],['limit_low','Provider limit low'],['limit_reset','Provider limit reset']] as const;

type Channel = Awaited<ReturnType<typeof createChannel>>;
function timeAgo(iso: string): string {
  const diff = (Date.now() - new Date(iso).getTime()) / 1000;
  if (!Number.isFinite(diff)) return '';
  if (diff < 60) return 'Just now';
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
  if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
  return `${Math.floor(diff / 86400)}d ago`;
}
type Session = { source?:string; id: string; title?: string | null; agent: string; model?: string | null; state: string; can_prompt: boolean; can_resume?:boolean; can_continue_here?:boolean; can_reset?:boolean; requested_model?:string; limits?:ConversationLimit[]; header_limit?:ConversationLimit | null; hosted?:boolean; surface_label?:string; last_event_at?:string; delivery_capability?:{mode?:string;can_queue?:boolean}|string; live?:LiveStatus; turn_started_at?:string; worktree?:boolean; worktree_branch?:string; worktree_path?:string };
type ChatEvent = { event_id: string; turn_index: number; created_at: string; role: string; content: string; status?: string | null };
type FolderListing = {path:string; parent:string | null; roots:string[]; folders:{name:string; path:string}[]; truncated:boolean};
type ScheduledMessage = {id:string;due_at:number;status:string;text:string};
type AutomationNotice = {id:string;conversation_id:string;message:string;created_at:number};
type PendingApproval = {id:string;agent:string;tool:string;summary:string;expires_at:number};
type SetupStatus = {session_id:string;blocker:'trust_folder'|'setup';agent:string;folder:string};
type Reply = { settings?:NotificationPreferences;terminal_data?:number[];cursor?:number;reset?:boolean;limit_message?:string|null; screen?:string; blocker?:'trust_folder'|'setup'|null; folder?:string; agent?:string; name?:string; size?:number; kind?:string; mime_type?:string; language?:string; data_url?:string; distros?:{name:string;label:string;running:boolean}[]; starting?:string; branch?:string; url?:string; sent?:boolean; request_id?:string; workspace_path?:string; filename?:string; artifacts?:ArtifactItem[]; changes_count?:number; content?:string; entries?:RepoFileEntry[]; repo?:string; diff?:string; status?:string; reason?:string; lossy?:boolean;truncated?:boolean;  items?:PendingApproval[]; decision?:string; reset_armed?:boolean; id?:string; messages?:ScheduledMessage[]; policy?:string; notices?:AutomationNotice[]; type: string; secret?:string; request_type?: string; message?: string; sessions?: Session[]; providers?:ProviderCatalog[]|ProviderOverview[]; session_id?: string; new_session_id?: string; events?: ChatEvent[]; page?:number; total_pages?:number; total_count?:number } & Partial<FolderListing>;

const REMOTE_DRAFTS_KEY = 'gnomcode-remote-drafts';
function loadRemoteDrafts(): Map<string, string> {
  const out = new Map<string, string>();
  try {
    const raw = sessionStorage.getItem(REMOTE_DRAFTS_KEY);
    if (!raw) return out;
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    if (!parsed || typeof parsed !== 'object') return out;
    for (const [k, v] of Object.entries(parsed).slice(-32)) {
      if (typeof k === 'string' && typeof v === 'string' && v.length <= 4000 && v) {
        out.set(k, v);
      }
    }
  } catch {}
  return out;
}
function persistRemoteDrafts(map: Map<string, string>) {
  try {
    const entries = [...map.entries()].filter(([, v]) => Boolean(v)).slice(-32);
    if (entries.length === 0) sessionStorage.removeItem(REMOTE_DRAFTS_KEY);
    else sessionStorage.setItem(REMOTE_DRAFTS_KEY, JSON.stringify(Object.fromEntries(entries)));
  } catch {}
}

export default function RemoteCompanion({ embedded = false, visible = true }: { embedded?: boolean; visible?:boolean }) {
  const [link, setLink] = useState<DeviceOffer>({ room: '', key: '', relay: '' });
  const autoConnect = useRef(false);
  const [optimistic, setOptimistic] = useState<Record<string,PendingChat[]>>({});
  const [conversationSearch, setConversationSearch] = useState('');
  const [desktopPanel, setDesktopPanel] = useState(false);
  const [modelPanel, setModelPanel] = useState(false);
  const [limitsPanel, setLimitsPanel] = useState(false);
  const [changesPanel, setChangesPanel] = useState(false);
  const [changesLoading, setChangesLoading] = useState(false);
  const [changesState, setChangesState] = useState<{ session_id: string; repo?: string; diff?: string; status?: string; lossy?: boolean; truncated?: boolean; error?: string } | null>(null);
  const changesRequested = useRef(false);
  const changesFileMode = useRef(false);
  const [changesSelected, setChangesSelected] = useState('');
  const [filesPanel, setFilesPanel] = useState(false);
  const [filePathInput, setFilePathInput] = useState('');
  const [fileListState, setFileListState] = useState<RepoFileListing | null>(null);
  const [fileData, setFileData] = useState<RepoFileData | null>(null);
  const filesRequested = useRef(false);
  const thumbRequests = useRef(new Map<string, (preview: RepoFileData) => void>());
  const [liveTick, setLiveTick] = useState(() => Date.now());
  const [artifactsPanel, setArtifactsPanel] = useState(false);
  const [artifacts, setArtifacts] = useState<ArtifactItem[]>([]);
  const artifactLink = useRef('');
  const [selectedArtifactId, setSelectedArtifactId] = useState('');
  const [artifactsLoading, setArtifactsLoading] = useState(false);
  const [artifactsError, setArtifactsError] = useState('');
  const [artifactsChangesCount, setArtifactsChangesCount] = useState<number | null>(null);
  const [notificationPreferences, setNotificationPreferences] = useState<NotificationPreferences|null>(null);
  const [notificationPreferenceStatus, setNotificationPreferenceStatus] = useState('');
  const [limitMessage, setLimitMessage] = useState('');
  const [terminalOpen, setTerminalOpen] = useState(false);
  const terminalDialog=useRef<HTMLDialogElement|null>(null);
  useEffect(()=>{if(terminalOpen)terminalDialog.current?.show();},[terminalOpen]);
  const [terminalOutput, setTerminalOutput] = useState<TerminalOutput|null>(null);
  const terminalCursor = useRef<number|undefined>(undefined);
  const terminalDimensions = useRef({cols:160,rows:24});
  const [terminalError, setTerminalError] = useState('');

  const [attachments, setAttachments] = useState<AttachmentItem[]>([]);
  const [attachError, setAttachError] = useState('');
  const [phase, setPhase] = useState<'ready' | 'connecting' | 'reconnecting' | 'unavailable' | 'paired'>('ready');
  const [error, setError] = useState('');
  const [sessions, setSessions] = useState<Session[]>([]);
  const [historyRows,setHistoryRows]=useState<Session[]>([]);
  const [historyPage,setHistoryPage]=useState(1);
  const [historyPages,setHistoryPages]=useState(1);
  const [historyTotal,setHistoryTotal]=useState(0);
  const [includeSubagents,setIncludeSubagents]=useState(false);
  const historyQuery=useRef({page:1,search:'',include_subagents:false,source:'managed'});
  const launchPending=useRef(false);
  const launchDeadline=useRef(0);
  const [launching,setLaunching]=useState(false);
  const notifiedEvents=useRef(new Set<string>());
  const backgroundNotifications=useRef(false);
  const [selected, setSelected] = useState('');
  const [showSessions, setShowSessions] = useState(false);
  useEffect(() => {
    if (typeof window !== 'undefined' && window.matchMedia('(min-width: 761px)').matches) {
      setShowSessions(true);
    }
  }, []);
  const [newSession,setNewSession]=useState(false);
  const [overviewOpen,setOverviewOpen]=useState(false);
  const [quickFilter,setQuickFilter]=useState<'all'|'active'|'pinned'>('all');
  const [pinned,setPinned]=useState<string[]>(()=>{try{const raw=JSON.parse(localStorage.getItem('gnomcode.pinnedSessions')||'[]');return Array.isArray(raw)?raw.filter((item):item is string=>typeof item==='string'):[];}catch{return [];}});
  const togglePin=(id:string)=>setPinned(current=>{const next=current.includes(id)?current.filter(item=>item!==id):[...current,id];try{localStorage.setItem('gnomcode.pinnedSessions',JSON.stringify(next));}catch{}return next;});
  const [events, setEvents] = useState<ChatEvent[]>([]);
  const [prompt, setPrompt] = useState('');
  const [scheduleTime,setScheduleTime]=useState('');
  const [scheduleText,setScheduleText]=useState('');
  const [resetArmed,setResetArmed]=useState(false);
  const [schedules,setSchedules]=useState<ScheduledMessage[]>([]);
  const [routing,setRouting]=useState('ask_first');
  const [automationNotices,setAutomationNotices]=useState<AutomationNotice[]>([]);
  const [schedulePending,setSchedulePending]=useState(false);
  const [approvals,setApprovals]=useState<PendingApproval[]>([]);
  const [setup,setSetup]=useState<SetupStatus|null>(null);
  const pendingSchedule=useRef<{id:string;session:string;text:string}|null>(null);
  const scheduleTimeout=useRef<ReturnType<typeof setTimeout>|null>(null);
  const [modelProviders,setModelProviders]=useState<ProviderCatalog[]>([]);
  const [providerUsage,setProviderUsage]=useState<ProviderOverview[]|null>(null);
  const [modelsLoading,setModelsLoading]=useState(false);
  const [modelsError,setModelsError]=useState('');
  const modelRequest=useRef(false);
  const modelDeadline=useRef(0);
  const [launchAgent, setLaunchAgent] = useState('claude');
  const [launchRepo, setLaunchRepo] = useState('');
  // "Run in" (ADR 0060): '' is Windows, otherwise a WSL distro; the folder is then a Linux path.
  const [runIn, setRunIn] = useState('');
  const [wslDistros, setWslDistros] = useState<{name:string;label:string;running:boolean}[]>([]);
  const [launchModel, setLaunchModel] = useState('');
  const [launchWorktree, setLaunchWorktree] = useState(false);
  const [launchWorktreeBranch, setLaunchWorktreeBranch] = useState('');
  const [worktreePanelOpen, setWorktreePanelOpen] = useState(false);
  const [worktreeDiffData, setWorktreeDiffData] = useState<WorktreeDiffData | null>(null);
  const [worktreeConfirm, setWorktreeConfirm] = useState<'worktree_merge' | 'worktree_discard' | 'worktree_open_pr' | null>(null);
  const [worktreeActionBusy, setWorktreeActionBusy] = useState(false);
  const [continueHereBusy, setContinueHereBusy] = useState(false);
  const [notice, setNotice] = useState('');
  const [historyLoading, setHistoryLoading] = useState(false);
  const [sending, setSending] = useState(false);
  const [interrupting, setInterrupting] = useState(false);
  const [hasEarlier, setHasEarlier] = useState(false);
  const [folderBrowser, setFolderBrowser] = useState<FolderListing | null>(null);
  const [foldersLoading, setFoldersLoading] = useState(false);
  const [notifGranted, setNotifGranted] = useState(false);
  useEffect(() => {
    setNotifGranted(getNotificationPermission() === 'granted');
  }, []);
  async function toggleOrTestNotifications() {
    const granted = getNotificationPermission() === 'granted' || await requestNotificationPermission();
    setNotifGranted(granted);
    if (!granted) {
      setError(getNotificationPermission() === 'unsupported'
        ? 'Notifications are unavailable in this browser. On iPhone, add Gnomcode to the Home Screen and open it there.'
        : 'Notifications are blocked. Allow notifications for Gnomcode in this browser and device settings, then test again.');
      return;
    }
    setNotice('Registering this device and sending a background test...');
    const result=await triggerServerPushTest();
    if(result.ok){backgroundNotifications.current=true;setNotice(result.message+' Agent alerts also work with Remote closed while your laptop stays online.');}
    else {setNotice('');setError(result.message);}
  }
  const promptInput = useRef<HTMLTextAreaElement | null>(null);
  const chatContainer = useRef<HTMLDivElement | null>(null);
  const historyAnchor = useRef<{top:number; height:number} | null>(null);
  const drafts = useRef(loadRemoteDrafts());
  const promptRef = useRef('');
  const pendingPrompt = useRef<{session:string; text:string; id:string; fromComposer:boolean} | null>(null);
  const socket = useRef<WebSocket | null>(null);
  const channel = useRef<Channel | null>(null);
  const selectedRef = useRef('');
  const loadingHistory = useRef(false);
  const poll = useRef<ReturnType<typeof setInterval> | null>(null);
  const incoming = useRef(Promise.resolve());
  const outgoing = useRef(Promise.resolve());
  const chunks = useRef('');
  const proofKey=useRef<CryptoKey|null>(null);
  const accountOwner=useRef('');
  const connectedDesktop=useRef('');
  const retryStart=useRef(0);
  const reconnectTimer=useRef<ReturnType<typeof setTimeout>|null>(null);
  const manualDisconnect=useRef(false);
  const restoredSession=useRef('');
  const folderDialog=useRef<HTMLDialogElement|null>(null);
  const remoteRoot=useRef<HTMLElement|null>(null);
  const reconnect=useRef(()=>{});
  const connectionEpoch=useRef(0);
  const lastReply=useRef(Date.now());
  const authenticated=useRef(false);
  const connectionWatch=useRef<ReturnType<typeof setInterval>|null>(null);
  const desktopDialog=useRef<HTMLDialogElement|null>(null);
  const modelDialog=useRef<HTMLDialogElement|null>(null);
  const limitsDialog=useRef<HTMLDialogElement|null>(null);
  const changesDialog=useRef<HTMLDialogElement|null>(null);
  const artifactsDialog=useRef<HTMLDialogElement|null>(null);
  const artifactsRequest=useRef(false);
  const attachInput=useRef<HTMLInputElement|null>(null);
  const pendingUploadIds=useRef(new Set<string>());
  const uploadTimers=useRef(new Map<string, ReturnType<typeof setTimeout>>());
  useEffect(() => () => { for (const timer of uploadTimers.current.values()) clearTimeout(timer); }, []);
  const attachmentsRef=useRef<AttachmentItem[]>([]);
  attachmentsRef.current = attachments;
  const interruptAttempt=useRef<{sessionId:string; requestId:string}|null>(null);
  const interruptInFlight=useRef(false);
  useEffect(()=>{
    if(!limitsPanel)return;
    limitsDialog.current?.showModal();
    // The sheet lists every provider, as the desktop app does; a Remote client has no local API.
    void send({type:'provider_usage'}).catch(()=>setProviderUsage([]));
  },[limitsPanel]);
  useEffect(()=>{if(changesPanel)changesDialog.current?.show();},[changesPanel]);
  const filesDialog=useRef<HTMLDialogElement|null>(null);
  useEffect(()=>{if(filesPanel)filesDialog.current?.show();},[filesPanel]);
  const sidebarToggle=useRef<HTMLButtonElement|null>(null);
  const sidebarClose=useRef<HTMLButtonElement|null>(null);

  useEffect(() => {
    let active=true;
    manualDisconnect.current=false;
    void supabase.auth.getSession().then(async({data:{session}})=>{
      if(!active || !session)return;
      accountOwner.current=session.user.id;
      const value=sessionStorage.getItem('gnomcode-pairing-offer');sessionStorage.removeItem('gnomcode-pairing-offer');
      const offer=value?JSON.parse(value):null;
      if(offer?.owner_id===session.user.id){if(active){autoConnect.current=true;setLink(offer);}return;}
    }).catch(()=>{if(active)setError('Could not restore this device. Open Devices to try again.');});
    return () => {if(scheduleTimeout.current)clearTimeout(scheduleTimeout.current);active=false;manualDisconnect.current=true;connectionEpoch.current++;if(reconnectTimer.current)clearTimeout(reconnectTimer.current);if(connectionWatch.current)clearInterval(connectionWatch.current);if (poll.current) clearInterval(poll.current); socket.current?.close();};
  }, []);

  useEffect(()=>{
    const update=()=>{
      remoteRoot.current?.style.setProperty('--remote-viewport-height',`${window.visualViewport?.height || innerHeight}px`);
      remoteRoot.current?.style.setProperty('--remote-viewport-top',`${window.visualViewport?.offsetTop || 0}px`);
    };
    if(innerWidth>760)setShowSessions(true);
    update();window.visualViewport?.addEventListener('resize',update);window.visualViewport?.addEventListener('scroll',update);window.addEventListener('resize',update);
    return()=>{window.visualViewport?.removeEventListener('resize',update);window.visualViewport?.removeEventListener('scroll',update);window.removeEventListener('resize',update);};
  },[]);

  useEffect(()=>{
    if(!visible)return;
    if(phase!=='paired' && phase!=='reconnecting' && !(phase==='unavailable' && sessions.length))return;
    const body=document.body,html=document.documentElement;
    const previous={position:body.style.position,top:body.style.top,width:body.style.width,overflow:body.style.overflow,htmlOverflow:html.style.overflow};
    const y=window.scrollY;
    body.style.position='fixed';body.style.top=`${-y}px`;body.style.width='100%';body.style.overflow='hidden';html.style.overflow='hidden';
    return()=>{body.style.position=previous.position;body.style.top=previous.top;body.style.width=previous.width;body.style.overflow=previous.overflow;html.style.overflow=previous.htmlOverflow;window.scrollTo(0,y);};
  },[phase,sessions.length,visible]);

  useEffect(()=>{const visible=()=>{
    if(document.visibilityState!=='visible' || !proofKey.current)return;
    if(socket.current?.readyState===WebSocket.OPEN && channel.current && authenticated.current){lastReply.current=Date.now();void requestSessions().catch(()=>reconnect.current());}
    else if(socket.current?.readyState!==WebSocket.CONNECTING){if(reconnectTimer.current)clearTimeout(reconnectTimer.current);retryStart.current=0;reconnect.current();}
  };document.addEventListener('visibilitychange',visible);window.addEventListener('online',visible);return()=>{document.removeEventListener('visibilitychange',visible);window.removeEventListener('online',visible);};},[]);

  useEffect(()=>{const {data:{subscription}}=supabase.auth.onAuthStateChange((event,session)=>{
    const previous=accountOwner.current;
    if(event==='SIGNED_OUT'){backgroundNotifications.current=false;void disableDeviceNotifications();}
    if(event==='SIGNED_OUT' || (previous && session?.user.id && previous!==session.user.id)){
      disconnect();if(previous)void forgetDevice(previous);accountOwner.current=session?.user.id||'';
    }
  });return()=>subscription.unsubscribe();},[]);

  useEffect(()=>{
    if(phase!=='paired')return;
    let last=0;const active=()=>{if(Date.now()-last>60000){last=Date.now();void send({type:'activity'}).catch(()=>{});}};
    window.addEventListener('pointerdown',active);window.addEventListener('keydown',active);window.addEventListener('wheel',active,{passive:true});
    return()=>{window.removeEventListener('pointerdown',active);window.removeEventListener('keydown',active);window.removeEventListener('wheel',active);};
  },[phase]);

  useEffect(()=>{if(desktopPanel)desktopDialog.current?.showModal();},[desktopPanel]);
  useEffect(()=>{if(modelPanel)modelDialog.current?.showModal();},[modelPanel]);
  useEffect(()=>{
    if(!artifactsPanel){artifactsRequest.current=false;setArtifactsLoading(false);return;}
    artifactsDialog.current?.show();
    if(phase==='paired' && selectedRef.current)void loadArtifacts();
  },[artifactsPanel,phase]);

  async function loadArtifacts(){
    if(!authenticated.current || !selectedRef.current || artifactsRequest.current)return;
    artifactsRequest.current=true;setArtifactsLoading(true);setArtifactsError('');
    try{await send({type:'artifacts',session_id:selectedRef.current});}catch{artifactsRequest.current=false;setArtifactsLoading(false);setArtifactsError('Could not load artifacts. Check your desktop connection and retry.');}
  }
  useEffect(()=>{
    if(!modelPanel || phase!=='paired'){modelRequest.current=false;setModelsLoading(false);return;}
    void loadModels();
    const timer=setInterval(()=>{if(modelRequest.current && Date.now()>=modelDeadline.current){modelRequest.current=false;setModelsLoading(false);setModelsError('The desktop did not return its model list. Refresh or update Gnomcode on your desktop.');}},500);
    return()=>clearInterval(timer);
    // loadModels reads refs only; re-running on its identity would refetch on every render.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  },[modelPanel,phase]);

  async function loadModels(){
    if(!authenticated.current || modelRequest.current)return;
    modelRequest.current=true;modelDeadline.current=Date.now()+12000;setModelsLoading(true);setModelsError('');setModelProviders([]);
    try{await send({type:'models'});}catch{modelRequest.current=false;setModelsLoading(false);setModelsError('Could not load models. Check your desktop connection and refresh.');}
  }

  function selectModel(model:ModelChoice){
    if(!authenticated.current)return;
    modelDialog.current?.close();
    void send({type:'switch',session_id:selectedRef.current,agent:model.agent,model:model.model}).catch(()=>setError('Could not change model. Your draft is saved. Try again.'));
  }
  useEffect(()=>{if(folderBrowser && !folderDialog.current?.open)folderDialog.current?.showModal();},[folderBrowser]);
  useEffect(()=>{
    if (!showSessions || !matchMedia('(max-width: 760px)').matches) return;
    sidebarClose.current?.focus();
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape' && !document.querySelector('dialog[open]')) {
        setShowSessions(false);
        sidebarToggle.current?.focus();
      }
    };
    document.addEventListener('keydown', closeOnEscape);
    return () => document.removeEventListener('keydown', closeOnEscape);
  },[showSessions]);

  useEffect(() => {
    const element = chatContainer.current;
    if (!element) return;
    const result = reconcileChat(events, optimistic[selected] || [], limitMessage ? 'error' : sessions.find(session=>session.id === selected)?.state);
    if (limitMessage) result.events = result.events.filter(event=>!(event as {waiting?:boolean}).waiting);
    if (result.pending.length !== (optimistic[selected] || []).length) setOptimistic(current=>({...current,[selected]:result.pending}));
    updateChat(element, result.events);
    const thumbSession = selected;
    hydrateAttachmentThumbs(element, path => new Promise((resolve, reject) => {
      const timer = window.setTimeout(() => { thumbRequests.current.delete(path); reject(new Error('timeout')); }, 20000);
      thumbRequests.current.set(path, preview => { window.clearTimeout(timer); resolve(preview); });
      send({ type: 'repo_file', session_id: thumbSession, path }).catch(reject);
    }));
    if (historyAnchor.current) { element.scrollTop = historyAnchor.current.top + element.scrollHeight - historyAnchor.current.height; historyAnchor.current = null; }
  }, [events, selected, optimistic, sessions, limitMessage]);

  useEffect(() => bindPreviewInteractions(document.body), []);
  const working = sessions.find(session => session.id === selected)?.live?.state === 'working';
  useEffect(() => {
    if (!working) return;
    const timer = window.setInterval(() => setLiveTick(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [working]);
  useEffect(() => { if (autoConnect.current && link.room) {autoConnect.current = false; void connect();} });

  useEffect(() => {
    const input = promptInput.current;
    if (!input) return;
    input.style.height = 'auto';
    input.style.height = `${Math.min(160, Math.max(36, input.scrollHeight))}px`;
  }, [prompt, phase, selected]);

  function changePrompt(text: string) {
    promptRef.current = text;
    setPrompt(text);
    if (selectedRef.current) {
      if (text) drafts.current.set(selectedRef.current, text);
      else drafts.current.delete(selectedRef.current);
      persistRemoteDrafts(drafts.current);
    }
  }

  async function submitPromptText(rawText: string, fromComposer: boolean) {
    if (fromComposer && (pendingUploadIds.current.size > 0 || attachmentsRef.current.some(item => item.status !== 'saved' || !item.workspace_path))) return;
    const ready = fromComposer ? attachmentsRef.current.filter(item => item.status === 'saved' && item.workspace_path) : [];
    const composed = formatPromptWithAttachments(rawText, ready);
    const activeSession = sessions.find(session => session.id === selectedRef.current);
    if (!authenticated.current || !activeSession || activeSession.source === 'imported' || pendingPrompt.current || !composed) return;
    const id = crypto.randomUUID();
    const sourceSession = selectedRef.current;
    pendingPrompt.current = {session:sourceSession,text:rawText,id,fromComposer};
    const pending:PendingChat = {id,text:composed,created_at:new Date().toISOString(),baseline:events.map(event=>event.event_id),delivered:false};
    setOptimistic(current=>({...current,[selectedRef.current]:[...(current[selectedRef.current] || []),pending]}));
    setSending(true); setError(''); setNotice('');
    try { await send({type:'prompt',session_id:selectedRef.current,text:composed,id,queue_if_busy:true}); }
    catch { setOptimistic(current=>({...current,[sourceSession]:(current[sourceSession] || []).filter(item=>item.id !== id)})); pendingPrompt.current = null; setSending(false); setError('Prompt could not be sent. Your draft is kept.'); }
  }
  async function pickPhoneAttachment(file: File | null | undefined) {
    if (!file || !authenticated.current || !selectedRef.current) return;
    const checked = validateAttachmentCandidate(file);
    if (!checked.ok) { setAttachError(checked.reason); return; }
    setAttachError('');
    const requestId = crypto.randomUUID();
    const sessionId = selectedRef.current;
    const item: AttachmentItem = { id: requestId, filename: checked.filename, size: checked.size, mime_type: checked.mime_type, status: 'uploading' };
    attachmentsRef.current = [...attachmentsRef.current, item];
    setAttachments(attachmentsRef.current);
    pendingUploadIds.current.add(requestId);
    uploadTimers.current.set(requestId, setTimeout(() => {
      uploadTimers.current.delete(requestId);
      if (!pendingUploadIds.current.delete(requestId)) return;
      setAttachments(current => current.map(entry => entry.id === requestId ? { ...entry, status: 'error' } : entry));
      setAttachError('Upload timed out. Remove the failed file and attach it again after reconnecting.');
    }, 60000));
    try {
      const data_base64 = await new Promise<string>((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = () => {
          const res = String(reader.result || '');
          const comma = res.indexOf(',');
          resolve(comma >= 0 ? res.slice(comma + 1) : res);
        };
        reader.onerror = () => reject(new Error('Could not read file.'));
        reader.readAsDataURL(file);
      });
      if (sessionId !== selectedRef.current || !pendingUploadIds.current.has(requestId)) { pendingUploadIds.current.delete(requestId); return; }
      await send({ type: 'upload_attachment', session_id: sessionId, request_id: requestId, filename: checked.filename, mime_type: checked.mime_type, size: checked.size, data_base64 });
    } catch {
      if (!pendingUploadIds.current.delete(requestId)) return;
      clearTimeout(uploadTimers.current.get(requestId)); uploadTimers.current.delete(requestId);
      setAttachments(current => current.map(entry => entry.id === requestId ? { ...entry, status: 'error' } : entry));
      setAttachError('Could not upload attachment. Check your laptop connection.');
    }
  }

  function submitPrompt() { return submitPromptText(promptRef.current, true); }

  async function requestInterrupt() {
    const activeSession = sessions.find(session => session.id === selectedRef.current);
    if (!authenticated.current || !activeSession || !isSessionRunningState(activeSession.state) || interruptInFlight.current) return;
    if (!interruptAttempt.current || interruptAttempt.current.sessionId !== activeSession.id) {
      interruptAttempt.current = { sessionId: activeSession.id, requestId: createInterruptRequestId() };
    }
    const requestId = interruptAttempt.current.requestId;
    interruptInFlight.current = true;
    setInterrupting(true);
    setError('');
    setNotice('Sending interrupt signal (Esc)...');
    try {
      await send({ type: 'interrupt', session_id: activeSession.id, request_id: requestId });
    } catch {
      interruptInFlight.current = false;
      setInterrupting(false);
      setNotice('');
      setError('Interrupt could not be sent. Tap Interrupt to retry.');
    }
  }

  async function scheduleDraft() {
    if(!authenticated.current || schedulePending || !selectedRef.current)return;
    const text=scheduleText.trim(),due=Math.floor(new Date(scheduleTime).getTime()/1000);
    if(!text || !Number.isFinite(due) || due<=Date.now()/1000){setError(!text?'Write the message to send.':'Pick a time in the future.');return;}
    const id=crypto.randomUUID(),session=selectedRef.current;
    pendingSchedule.current={id,session,text};setSchedulePending(true);setError('');setNotice('Saving on your laptop...');
    scheduleTimeout.current=setTimeout(()=>{setSchedulePending(false);setNotice('');setError('Schedule not confirmed. Check saved messages before retrying. Your draft is kept.');},15000);
    try{await send({type:'schedule',session_id:session,id,text,due_at:due});}catch{if(scheduleTimeout.current)clearTimeout(scheduleTimeout.current);setSchedulePending(false);setError('Schedule could not be sent. Your draft is kept.');}
  }
  function requestAutomation(value:object) {void send(value).catch(()=>setError('Could not reach your laptop. Reconnect and check saved settings.'));}

  async function browseFolders(path?: string) {
    setFoldersLoading(true); setError('');
    try { await send({type:'workspaces',path}); }
    catch { setFoldersLoading(false); setError('Desktop folders unavailable. Check the connection and retry.'); }
  }

  async function send(value: object) {
    const expectedSocket=socket.current,expectedChannel=channel.current,epoch=connectionEpoch.current;
    outgoing.current = outgoing.current.catch(() => {}).then(async () => {
      if (!expectedChannel || expectedSocket?.readyState !== WebSocket.OPEN || expectedSocket!==socket.current || epoch!==connectionEpoch.current) throw new Error('Connection closed');
      for (const part of remoteRequestParts(value)) {
        const upload = value as {type?:string;request_id?:string};
        if (upload.type === 'upload_attachment' && !pendingUploadIds.current.has(upload.request_id || '')) throw new Error('Upload cancelled');
        const frame = await expectedChannel.seal(part);
        if(expectedSocket!==socket.current || epoch!==connectionEpoch.current)throw new Error('Connection changed');
        expectedSocket.send(new TextEncoder().encode(JSON.stringify(frame)));
        while (expectedSocket.bufferedAmount > 256 * 1024) {
          await new Promise(resolve => setTimeout(resolve, 20));
          if (expectedSocket.readyState !== WebSocket.OPEN || expectedSocket !== socket.current) throw new Error('Connection closed');
        }
      }
    });
    return outgoing.current;
  }

  function loadChat(sessionId: string, before: string | null = null) {
    const epoch=connectionEpoch.current;
    void send({ type: 'chat', session_id: sessionId, before, limit: 500 }).catch(() => {if(epoch===connectionEpoch.current && socket.current?.readyState===WebSocket.OPEN && authenticated.current)setError('Chat connection lost.');});
  }

  async function loadChanges(path = '') {
    if (!authenticated.current || !selectedRef.current) return;
    changesRequested.current = true;
    if (!path) changesFileMode.current = false;
    setChangesSelected(path);
    setChangesPanel(true);
    setChangesLoading(true);
    setChangesState(null);
    try { await send({ type: 'changes', session_id: selectedRef.current, ...(path ? { path } : {}) }); }
    catch { changesRequested.current = false; setChangesLoading(false); setChangesState({ session_id: selectedRef.current, error: 'Could not reach your desktop for workspace changes.' }); }
  }

  async function browseRepoFiles(path = '') {
    if (!authenticated.current || !selectedRef.current) return;
    filesRequested.current = true;
    setFilesPanel(true);
    setFileListState({ loading: true });
    try { await send({ type: 'repo_files', session_id: selectedRef.current, path }); }
    catch { filesRequested.current = false; setFileListState({ error: 'Could not reach your desktop for workspace files.' }); }
  }

  async function readRepoFile(rawPath: string) {
    if (!authenticated.current || !selectedRef.current) return;
    const check = normalizeWorkspaceRelPath(rawPath);
    if (!check.ok) { setFileData({ error: check.error }); return; }
    setFilePathInput(check.path);
    filesRequested.current = true;
    setFileData({ loading: true });
    try { await send({ type: 'repo_file', session_id: selectedRef.current, path: check.path }); }
    catch { filesRequested.current = false; setFileData({ error: 'Could not read workspace file from your desktop.' }); }
  }

  useEffect(() => {
    setTerminalOutput(null); terminalCursor.current=undefined; setTerminalError('');
    if (!terminalOpen || phase !== 'paired' || !selected) return;
    let stopped = false, pending = false;
    const refresh = async () => {
      if (stopped || pending || document.hidden) return;
      pending = true;
      try { await send({type:'terminal_screen',session_id:selected,after:terminalCursor.current,...terminalDimensions.current}); }
      catch { if (!stopped) setTerminalError('Terminal connection lost. Reconnect and reopen it.'); }
      finally { pending = false; }
    };
    void refresh(); const timer = setInterval(() => void refresh(), 300);
    return () => { stopped = true; clearInterval(timer); };
  }, [terminalOpen, phase, selected]);
  async function sendTerminalInput(text: string) {
    if (!connected || !selected || !text) return;
    setTerminalError('');
    let chunk = '';
    try {
      for (const char of text) {
        if (chunk.length + char.length > 1000) { await send({type:'terminal_input',session_id:selected,text:chunk}); chunk=''; }
        chunk += char;
      }
      if (chunk) await send({type:'terminal_input',session_id:selected,text:chunk});
    } catch { setTerminalError('Terminal input could not be sent. Reconnect before retrying.'); }
  }

  function requestSessions(){return send({type:'sessions',...historyQuery.current});}

  async function beginPaired() {
    const epoch=connectionEpoch.current;
    authenticated.current=true;retryStart.current=0;setError('');setPhase('paired');
    try {sessionStorage.setItem('gnomcode-pairing-offer',JSON.stringify(link));}catch{}
    if(getNotificationPermission()==='granted')void subscribeToPushServer().then(result=>{
      if(epoch!==connectionEpoch.current)return;
      backgroundNotifications.current=result.ok;
      if(!result.ok)setNotice(result.error || 'Background notification setup could not be confirmed. Use the notification button to retry.');
    });
    await requestSessions();
    await send({type:'routing'});
    if(epoch!==connectionEpoch.current)return;
    if (poll.current) clearInterval(poll.current);
    poll.current = setInterval(() => {
      if(launchPending.current && Date.now()>launchDeadline.current){launchPending.current=false;setLaunching(false);setError('Launch is not confirmed. Check conversations before retrying; the agent may have started.');}
      void requestSessions().catch(() => {if(epoch===connectionEpoch.current && socket.current?.readyState===WebSocket.OPEN)setError('Session refresh failed. Check the desktop connection and reconnect.');});
      if (selectedRef.current && !loadingHistory.current) loadChat(selectedRef.current);
      if(selectedRef.current)void send({type:'schedules',session_id:selectedRef.current}).catch(()=>{});
      if(selectedRef.current)void send({type:'setup',session_id:selectedRef.current}).catch(()=>{});
      void send({type:'notices'}).catch(()=>{});
      void send({type:'approvals'}).catch(()=>{});
    }, 2000);
    void send({type:'approvals'}).catch(()=>{});
  }

  // Decisions are sent for one exact request id; the desktop decides whether it still applies.
  function decideApproval(id:string,decision:'approve'|'deny'){
    const agent=approvals.find(item=>item.id===id)?.agent ?? 'claude-code';
    setApprovals(current=>current.filter(item=>item.id!==id));
    void send({type:'approval',id,decision}).catch(()=>{setError(approvalCopy(agent).unreachable);});
  }

  async function handle(reply: Reply, epoch:number) {
    if(epoch!==connectionEpoch.current)return;
    if (reply.type === 'terminal_screen') { if(reply.session_id===selectedRef.current && Array.isArray(reply.terminal_data) && typeof reply.cursor==='number') { terminalCursor.current=reply.cursor;setTerminalOutput({data:reply.terminal_data,cursor:reply.cursor,reset:!!reply.reset});setTerminalError(''); } }
    else if (reply.type === 'notification_settings') { if(reply.settings) {setNotificationPreferences(reply.settings);setNotificationPreferenceStatus('Preferences loaded.');} }
    else if (reply.type === 'terminal_input_ack') { /* The agent echoes accepted input through the terminal stream. */ }
    else if (reply.type === 'schedules') { if(reply.session_id===selectedRef.current){setSchedules(reply.messages || []);setResetArmed(Boolean(reply.reset_armed));} }
    else if(reply.type==='notices'){
      const notices=reply.notices || [];
      setAutomationNotices(notices);
      for(const item of notices){
        if(notifiedEvents.current.has(item.id))continue;
        notifiedEvents.current.add(item.id);
        if(!backgroundNotifications.current && document.hidden && item.created_at*1000 >= Date.now()-120000){
          void sendAppNotification({title:'Gnomcode',body:item.message,tag:item.id,data:{url:'/remote'}});
        }
      }
    }
    else if(reply.type==='approvals'){setApprovals(Array.isArray(reply.items)?reply.items:[]);}
    else if(reply.type==='setup'){if(reply.session_id===selectedRef.current)setLimitMessage(reply.limit_message || '');if(reply.session_id===selectedRef.current)setSetup(reply.blocker?{session_id:reply.session_id||'',blocker:reply.blocker,agent:reply.agent||'The agent',folder:reply.folder||''}:null);}
    else if(reply.type==='trust_folder_ack'){setSetup(null);setNotice('Folder trusted. Your messages can go through now.');}
    else if(reply.type==='approval_ack'){setNotice(reply.decision==='deny'?'Denied.':'Approved once.');void send({type:'approvals'}).catch(()=>{});}
    else if(reply.type==='routing'){setRouting(reply.policy || 'ask_first');}
    else if(reply.type==='schedule_ack') {
      const pending=pendingSchedule.current;
      if(pending && pending.id===reply.id){
        if(scheduleTimeout.current)clearTimeout(scheduleTimeout.current);
        setSchedulePending(false);pendingSchedule.current=null;
        setScheduleText(current=>current.trim()===pending.text?'':current);
        setNotice('Schedule confirmed on your laptop. You can close your phone.');
        requestAutomation({type:'schedules',session_id:selectedRef.current});
      }
    }
    else if(reply.type==='cancel_schedule_ack'){setNotice('Scheduled message cancelled.');requestAutomation({type:'schedules',session_id:selectedRef.current});}
    else if(reply.type==='resume_on_reset_ack'){setNotice('Reset reconnection setting saved on your laptop.');}
    else if (reply.type === 'workspaces' && reply.path && reply.roots && reply.folders) {
      setFoldersLoading(false); setFolderBrowser({path:reply.path,parent:reply.parent || null,roots:reply.roots,folders:reply.folders,truncated:Boolean(reply.truncated)});
    } else if (reply.type === 'sessions') {
      setHistoryRows(reply.sessions || []);
      setHistoryPage(reply.page || 1);setHistoryPages(reply.total_pages || 1);setHistoryTotal(reply.total_count ?? reply.sessions?.length ?? 0);
      setSessions(previous=>{const selected=previous.find(row=>row.id===selectedRef.current);const rows=reply.sessions || [];return selected && !rows.some(row=>row.id===selected.id)?[...rows,selected]:rows;});
      if (!selectedRef.current && reply.sessions?.length) {
        const preferred=reply.sessions.find(session=>session.id===restoredSession.current) || reply.sessions.find(session=>session.can_prompt) || reply.sessions[0];
        if(preferred)selectSession(preferred.id);
      }
    } else if (reply.type === 'chat' && reply.session_id === selectedRef.current) {
      const page = reply.events || [];
      setEvents(current => {
        const all = new Map(current.map(event => [event.event_id, event]));
        page.forEach(event => all.set(event.event_id, event));
        return [...all.values()].sort((a, b) => a.turn_index - b.turn_index || a.created_at.localeCompare(b.created_at) || a.event_id.localeCompare(b.event_id));
      });
      if (loadingHistory.current) {
        setHasEarlier(page.length === 500);
        loadingHistory.current = false;
        setHistoryLoading(false);
      }
    } else if(reply.type==='trusted' && reply.secret){const key=await rememberDevice(link,reply.secret,()=>epoch===connectionEpoch.current && !manualDisconnect.current);if(epoch!==connectionEpoch.current)return;proofKey.current=key;await beginPaired();}
    else if(reply.type==='resumed'){await beginPaired();}
    else if(reply.type==='resume_rejected'){manualDisconnect.current=true;proofKey.current=null;await forgetDevice(accountOwner.current,connectedDesktop.current);socket.current?.close();setPhase('ready');setError('Access was revoked or this browser credential is no longer recognized. Open Devices to reconnect through your account.');}
    else if (reply.type === 'wsl_distros') { setWslDistros(Array.isArray(reply.distros) ? reply.distros : []); }
    else if (reply.type === 'launch_ack') { launchPending.current=false;setLaunching(false);setNotice(reply.starting ? `${reply.starting} Agent is starting.` : 'Agent started. Send its first message below.');setNewSession(false); await requestSessions(); if (reply.session_id) selectSession(reply.session_id); }
    else if (reply.type === 'prompt_ack') {
      const pending = pendingPrompt.current;
      if (pending && (!reply.session_id || reply.session_id === pending.session)) {
        if (pending.fromComposer) {
          if (drafts.current.get(pending.session) === pending.text) { drafts.current.delete(pending.session); persistRemoteDrafts(drafts.current); }
          if (pending.session === selectedRef.current && promptRef.current === pending.text) changePrompt('');
          setAttachments(current => current.filter(item => item.status !== 'saved')); setAttachError('');
        }
        pendingPrompt.current = null; setSending(false);
        if (reply.status === 'queued') {
          setOptimistic(current=>({...current,[pending.session]:(current[pending.session] || []).filter(item=>item.id !== pending.id)}));
          setNotice(reply.reason || 'Agent is busy; queued for next safe turn boundary.');
          requestAutomation({type:'schedules',session_id:pending.session});
        } else {
          setOptimistic(current=>({...current,[pending.session]:(current[pending.session] || []).map(item=>item.id === pending.id ? {...item,delivered:true} : item)}));
          setNotice('');
        }
      }
    }
    else if (reply.type === 'resume_session_ack') { setNotice('Ready to continue.'); await requestSessions(); if(reply.session_id) loadChat(reply.session_id); }
    else if (reply.type === 'changes') {
      changesRequested.current = false;
      setChangesLoading(false);
      if (!reply.session_id || reply.session_id === selectedRef.current) {
        changesFileMode.current ||= Boolean(reply.truncated);
        setChangesState({ session_id: reply.session_id || selectedRef.current, repo: reply.repo, diff: reply.diff || '', status: reply.status || '', lossy: Boolean(reply.lossy), truncated: Boolean(reply.truncated) });
      }
    }
    else if (reply.type === 'repo_files') {
      filesRequested.current = false;
      if (!reply.session_id || reply.session_id === selectedRef.current) {
        setFileListState({ path: reply.path || '', parent: parentWorkspacePath(reply.path || ''), entries: reply.entries || [], truncated: Boolean(reply.truncated) });
      }
    }
    else if (reply.type === 'repo_file') {
      const thumb = thumbRequests.current.get(reply.path || '');
      if (thumb) { thumbRequests.current.delete(reply.path || ''); thumb(reply); }
      else if (!reply.session_id || reply.session_id === selectedRef.current) {
        filesRequested.current = false;
        setFileData({ path: reply.path || '', name: reply.name, size: reply.size, kind: reply.kind, mime_type: reply.mime_type, language: reply.language, content: reply.content, data_url: reply.data_url, lossy: Boolean(reply.lossy), truncated: Boolean(reply.truncated) });
      }
    }
    else if (reply.type === 'continue_here_ack') {
      setContinueHereBusy(false);
      setNotice('Continued imported session in Gnomcode.');
      await requestSessions();
      const targetId = reply.new_session_id || reply.session_id;
      if (targetId) selectSession(targetId);
    }
    else if (reply.type === 'provider_usage') {setProviderUsage(Array.isArray(reply.providers)?reply.providers as ProviderOverview[]:[]);}
    else if (reply.type === 'models') {modelRequest.current=false;setModelsLoading(false);setModelsError('');setModelProviders(Array.isArray(reply.providers)?reply.providers as ProviderCatalog[]:[]);}
    else if (reply.type === 'artifacts' || reply.type === 'artifacts_result') {
      artifactsRequest.current = false;
      setArtifactsLoading(false);
      setArtifactsError('');
      const list = Array.isArray(reply.artifacts) ? reply.artifacts : [];
      setArtifacts(list);
      const linked = artifactLink.current ? artifactForLink(list, artifactLink.current) : null;
      const first = artifactLink.current ? linked : list[0];
      setSelectedArtifactId(first ? String(first.id || first.path || 0) : '');
      artifactLink.current = '';
      setArtifactsChangesCount(typeof reply.changes_count === 'number' ? reply.changes_count : null);
    }
    else if (reply.type === 'switch_ack') { setNotice('Switching model...'); await requestSessions(); }
    else if (reply.type === 'attachment_ack') {
      const reqId = reply.request_id || reply.id || '';
      if (!pendingUploadIds.current.delete(reqId)) return;
      clearTimeout(uploadTimers.current.get(reqId)); uploadTimers.current.delete(reqId);
      const saved = typeof reply.workspace_path === 'string' && reply.workspace_path.length > 0;
      setAttachments(current => current.map(entry => entry.id === reqId ? { ...entry, status: saved ? 'saved' : 'error', workspace_path: reply.workspace_path } : entry));
      if (!saved) setAttachError('Desktop did not confirm a saved file. Remove it and attach it again.');
    }
    else if (reply.type === 'error') {
      if(reply.request_type==='notification_settings'){setNotificationPreferenceStatus(reply.message || 'Could not load or save notification preferences.');return;}
      if(reply.request_type==='terminal_screen' || reply.request_type==='terminal_input') { setTerminalError(reply.message || 'Agent terminal unavailable.'); return; }
      if(reply.request_type==='changes' || (changesRequested.current && /unsupported remote command/i.test(reply.message || ''))){
        changesRequested.current=false;setChangesLoading(false);
        setChangesState({session_id:selectedRef.current,error:/unsupported/i.test(reply.message || '') ? 'Update Gnomcode on your desktop to inspect git changes from your phone.' : (reply.message || 'Could not load git changes.')});
        return;
      }
      if (reply.request_type === 'repo_files' || reply.request_type === 'repo_file' || (filesRequested.current && /unsupported remote command/i.test(reply.message || ''))) {
        filesRequested.current = false;
        const msg = /unsupported/i.test(reply.message || '')
          ? 'Update Gnomcode on your desktop to inspect workspace files from your phone.'
          : (reply.message || 'Could not inspect workspace file.');
        if (reply.request_type === 'repo_file') setFileData({ error: msg });
        else setFileListState({ error: msg });
        return;
      }
      // Older desktops have no WSL picker; the form stays Windows-only.
      if(reply.request_type==='wsl_distros'){setWslDistros([]);return;}
      if(reply.request_type==='artifacts' || (artifactsRequest.current && /Unsupported remote command/i.test(reply.message || ''))){
        artifactsRequest.current=false;setArtifactsLoading(false);
        setArtifactsError(/Unsupported remote command/i.test(reply.message || '') ? 'Artifacts view requires a newer Gnomcode desktop update.' : (reply.message || 'Could not load artifacts.'));
        return;
      }
      if(reply.request_type==='upload_attachment' || reply.request_type==='upload_chunk' || (pendingUploadIds.current.size > 0 && /Unsupported remote command/i.test(reply.message || ''))){
        const reqId = reply.request_id || [...pendingUploadIds.current][0];
        if (reqId) pendingUploadIds.current.delete(reqId);
        setAttachments(current => current.map(entry => (!reqId || entry.id === reqId) && entry.status === 'uploading' ? { ...entry, status: 'error' } : entry));
        setAttachError(/Unsupported remote command/i.test(reply.message || '') ? 'Phone attachments require a newer Gnomcode desktop update.' : (reply.message || 'Attachment upload failed.'));
        return;
      }
      if(reply.request_type==='interrupt' || (interruptInFlight.current && /Unsupported remote command/i.test(reply.message || ''))){
        interruptInFlight.current=false;setInterrupting(false);
        if(/Unsupported remote command/i.test(reply.message || '')){interruptAttempt.current=null;setNotice('Interrupt requires a newer Gnomcode desktop build.');return;}
      }
      if(reply.request_type && reply.request_type.startsWith('worktree_')){setWorktreeActionBusy(false);setWorktreeConfirm(null);}
      if(reply.request_type==='continue_here'){setContinueHereBusy(false);}
      if(reply.request_type==='session_changes'){setWorktreeDiffData({error: reply.message || 'Could not load worktree diff.'});return;}
      if(reply.request_type==='approval'){void send({type:'approvals'}).catch(()=>{});}
      if(reply.request_type==='launch'){launchPending.current=false;setLaunching(false);}
      if(reply.request_type==='schedule'){if(scheduleTimeout.current)clearTimeout(scheduleTimeout.current);setSchedulePending(false);setNotice('');}
      if(reply.request_type==='models'){modelRequest.current=false;setModelsLoading(false);setModelsError('Your desktop could not provide a model list. Update Gnomcode on your desktop, then refresh.');return;}
      if (!reply.request_type || reply.request_type === 'prompt') { const pending = pendingPrompt.current; if(pending) setOptimistic(current=>({...current,[pending.session]:(current[pending.session] || []).filter(item=>item.id !== pending.id)})); pendingPrompt.current = null; setSending(false); }
      if (!reply.request_type || reply.request_type === 'workspaces') setFoldersLoading(false);
      if (!reply.request_type || reply.request_type === 'chat') { loadingHistory.current = false; setHistoryLoading(false); }
      setError(reply.message || 'Remote request failed. Reconnect and try again.');
    }
    else if (reply.type === 'interrupt_ack') {
      if (!reply.request_id || interruptAttempt.current?.requestId === reply.request_id) {
        interruptAttempt.current = null;
      }
      interruptInFlight.current = false;
      setInterrupting(false);
      setNotice(formatInterruptAckMessage({ sent: reply.sent }));
    }
    else if (reply.type === 'session_changes') {
      if (!reply.session_id || reply.session_id === selectedRef.current) {
        setWorktreeDiffData({
          repo: reply.repo,
          branch: reply.branch,
          status: reply.status,
          diff: reply.diff,
          lossy: Boolean(reply.lossy),
        });
      }
    }
    else if (reply.type === 'worktree_action_ack') {
      setWorktreeActionBusy(false);
      setWorktreeConfirm(null);
      setNotice(reply.url ? `${reply.message || 'Pull request ready:'} ${reply.url}` : reply.message || 'Worktree action completed.');
      await requestSessions();
    }
  }

  async function connect(force=false) {
    if(!force && (socket.current?.readyState===WebSocket.OPEN || socket.current?.readyState===WebSocket.CONNECTING))return;
    if(force || !retryStart.current)retryStart.current=Date.now();
    if(Date.now()-retryStart.current>=30000){
      authenticated.current=false;manualDisconnect.current=true;connectionEpoch.current++;
      if(reconnectTimer.current)clearTimeout(reconnectTimer.current);
      if(connectionWatch.current)clearInterval(connectionWatch.current);
      if(poll.current)clearInterval(poll.current);
      const previous=socket.current;socket.current=null;previous?.close();
      setPhase('unavailable');setError('Your desktop is not responding. Check that it is awake and Gnomcode is running. Your account connection is still saved.');
      // Keep trying quietly: a desktop that wakes or starts later must not need a manual retry.
      if(proofKey.current)reconnectTimer.current=setTimeout(()=>{if(document.visibilityState==='visible'){retryStart.current=0;void connect(true);}},15000);
      return;
    }
    if(reconnectTimer.current)clearTimeout(reconnectTimer.current);
    if(connectionWatch.current)clearInterval(connectionWatch.current);
    authenticated.current=false;
    const epoch=++connectionEpoch.current;
    const previous=socket.current;socket.current=null;previous?.close();channel.current=null;
    manualDisconnect.current=false;
    try{restoredSession.current=sessionStorage.getItem(`gnomcode-remote-selected:${link.room}`)||'';}catch{}
    setError('');
    setNotice('');
    if (!/^[a-zA-Z0-9_-]{16,128}$/.test(link.room) || !/^[0-9a-f]{64}$/.test(link.key)
      || !(/^wss:\/\//.test(link.relay) || /^ws:\/\/(127\.0\.0\.1|localhost):/.test(link.relay))) {
      setPhase('ready');setError('Open Devices to select your desktop.');return;
    }
    let discoveryPending=false;
    try {
      setOptimistic({});
      if(selectedRef.current)drafts.current.set(selectedRef.current,promptRef.current);
      if(connectedDesktop.current && connectedDesktop.current!==link.desktop_id){proofKey.current=null;}
      if(!proofKey.current){selectedRef.current = ''; setSelected(''); setSessions([]); setEvents([]);}
      pendingPrompt.current = null; setSending(false); loadingHistory.current = false; setHistoryLoading(false);
      setFolderBrowser(null); setFoldersLoading(false); if(!proofKey.current){promptRef.current='';setPrompt('');}
      setPhase(proofKey.current ? 'reconnecting' : 'connecting');
      chunks.current='';outgoing.current=Promise.resolve();incoming.current=Promise.resolve();
      const {data:{session}}=await supabase.auth.getSession();
      if(epoch!==connectionEpoch.current)return;
      if(!session || session.user.id!==link.owner_id)throw new Error('Sign in to the desktop account.');
      accountOwner.current=session.user.id;connectedDesktop.current=link.desktop_id||'';
      const saved=await savedDevice(session.user.id,link.desktop_id!);
      if(epoch!==connectionEpoch.current)return;
      if(!saved)throw new Error('This browser credential is missing. Open Devices to reconnect automatically through your account.');
      proofKey.current=saved.proofKey||null;
      discoveryPending=true;
      const {data:request,error:directoryError}=await supabase.from('spawnt_device_requests').select('offer,revoked,state').eq('user_id',session.user.id).eq('desktop_id',link.desktop_id!).eq('id',link.request_id!).abortSignal(AbortSignal.timeout(10000)).maybeSingle();
      if(epoch!==connectionEpoch.current)return;
      if(directoryError)throw new Error('Device discovery is unavailable. Check your connection and try again.');
      discoveryPending=false;
      if(!request || request.revoked)throw new Error('Access was revoked. Open Devices to reconnect through your account.');
      if(request.state!=='approved' || !request.offer)throw new Error('Your desktop is not ready. Keep it awake with Gnomcode running.');
      if(request.offer.key!==link.key || request.offer.room!==link.room || request.offer.relay!==link.relay)throw new Error('The desktop identity changed. Open Devices to review its connection.');
      const keys=proofKey.current?await generateKeyPair():saved.enrollment;
      if(!keys || link.protocol_version!==3)throw new Error('Update your desktop and open Devices to reconnect.');
      if(epoch!==connectionEpoch.current)return;
      setPhase(proofKey.current?'reconnecting':'connecting');
      const ws = new WebSocket(`${link.relay.replace(/\/$/, '')}/${link.room}?role=phone`);
      ws.binaryType = 'arraybuffer'; socket.current = ws;
      lastReply.current=Date.now();
      connectionWatch.current=setInterval(()=>{
        if(epoch!==connectionEpoch.current || document.visibilityState!=='visible')return;
        if(Date.now()-lastReply.current<=15000 || pendingUploadIds.current.size > 0)return;
        ws.close();
        if(socket.current===ws){
          socket.current=null;authenticated.current=false;setError('');setPhase(proofKey.current?'reconnecting':'ready');
          reconnectTimer.current=setTimeout(()=>void connect(),1500);
        }
      },3000);
      let helloTimer:ReturnType<typeof setInterval>|undefined;
      const hello=()=>{if(ws.readyState===WebSocket.OPEN)ws.send(new TextEncoder().encode(JSON.stringify({type:'hello',public_key:keys.publicHex})));};
      ws.onopen = () => {hello();helloTimer=setInterval(hello,2000);};
      ws.onmessage = event => {
        incoming.current = incoming.current.then(async () => {
          if (socket.current !== ws || epoch!==connectionEpoch.current) return;
          const frame = JSON.parse(new TextDecoder().decode(event.data as ArrayBuffer));
          if(frame.type==='challenge') {
            clearInterval(helloTimer);
            if(!/^[a-f0-9]{32}$/.test(frame.nonce))throw new Error('Invalid challenge');
            const fresh=await createChannel(keys,link.key,`${link.room}:${frame.nonce}`,'phone');
            if(epoch!==connectionEpoch.current || socket.current!==ws)return;
            channel.current=fresh;
            if(proofKey.current){const proof=await deviceProof(proofKey.current,`${link.room}:${frame.nonce}:${keys.publicHex}`);if(epoch!==connectionEpoch.current || socket.current!==ws)return;await send({type:'resume',proof});}
            else {await send({type:'enroll'});}
            return;
          }
          const message = await channel.current!.open(frame) as Reply & { data?: string; final?: boolean };
          if(epoch!==connectionEpoch.current || socket.current!==ws)return;
          lastReply.current=Date.now();
          if (message.type !== 'chunk' || typeof message.data !== 'string') throw new Error('Invalid response');
          chunks.current += message.data;
          if (message.final) {
            const complete = JSON.parse(chunks.current) as Reply;
            chunks.current = '';
            await handle(complete,epoch);
          }
        }).catch(() => {if(epoch!==connectionEpoch.current)return;setError('Could not verify or save this device connection. Open Devices to retry.'); ws.close(); });
      };
      ws.onclose = () => {clearInterval(helloTimer);if(socket.current===ws && connectionWatch.current)clearInterval(connectionWatch.current);if (socket.current !== ws || manualDisconnect.current) return;authenticated.current=false;setError(''); if (poll.current) clearInterval(poll.current); loadingHistory.current=false;setHistoryLoading(false);pendingPrompt.current = null; setSending(false);setPhase('reconnecting');reconnectTimer.current=setTimeout(()=>void connect(),1500);};
    } catch(error) {
      if(epoch!==connectionEpoch.current)return;
      if(discoveryPending){setPhase('reconnecting');setError('');reconnectTimer.current=setTimeout(()=>void connect(),1500);return;}
      setPhase('unavailable');setError(error instanceof Error?error.message:'Could not connect to your desktop. Try again.');
    }
  }
  reconnect.current=()=>void connect();

  function selectSession(id: string) {
    setLimitMessage('');
    // The conversation column is permanent on desktop, as in the app; only the phone drawer closes.
    if (matchMedia('(max-width: 760px)').matches) setShowSessions(false);
    if (selectedRef.current) drafts.current.set(selectedRef.current, promptRef.current);
    changePrompt(drafts.current.get(id) || '');
    setHasEarlier(false); historyAnchor.current = null;
    changesRequested.current = false; setChangesPanel(false); setChangesLoading(false); setChangesState(null);
    filesRequested.current = false; setFilesPanel(false); setFileListState(null); setFileData(null); setFilePathInput('');
    selectedRef.current = id; setSchedules([]); setResetArmed(false);
    setTerminalOpen(false); setTerminalOutput(null); setTerminalError(''); terminalCursor.current = undefined;
    setArtifactsPanel(false); setArtifacts([]); setSelectedArtifactId(''); setArtifactsError(''); setArtifactsChangesCount(null);
    pendingUploadIds.current.clear(); setAttachments([]); setAttachError('');
    interruptAttempt.current = null; interruptInFlight.current = false; setInterrupting(false);
    setSelected(id); setEvents([]); loadingHistory.current = true; setHistoryLoading(true);
    setWorktreePanelOpen(false); setWorktreeDiffData(null); setWorktreeConfirm(null); setWorktreeActionBusy(false);
    try {sessionStorage.setItem(`gnomcode-remote-selected:${link.room}`,id);}catch{}
    if(phase==='paired')loadChat(id);else{loadingHistory.current=false;setHistoryLoading(false);}
  }

  function disconnect() {
    try {sessionStorage.removeItem('gnomcode-pairing-offer');}catch{}
    modelRequest.current=false;setModelProviders([]);setModelsError('');setModelsLoading(false);
    artifactsRequest.current=false;setArtifactsPanel(false);setArtifacts([]);setSelectedArtifactId('');setArtifactsError('');setArtifactsChangesCount(null);
    pendingUploadIds.current.clear();setAttachments([]);setAttachError('');
    authenticated.current=false;connectionEpoch.current++;
    manualDisconnect.current=true;if(reconnectTimer.current)clearTimeout(reconnectTimer.current);if(connectionWatch.current)clearInterval(connectionWatch.current);proofKey.current=null;void forgetDevice(accountOwner.current,connectedDesktop.current).catch(()=>{});
    setOptimistic({});
    const previous = socket.current; socket.current = null; previous?.close(); if (poll.current) clearInterval(poll.current);
    selectedRef.current = ''; setSelected(''); pendingPrompt.current = null; setSending(false); interruptAttempt.current = null; interruptInFlight.current = false; setInterrupting(false); setError(''); setNotice('');
    setPhase('ready');setSessions([]);setEvents([]);setHistoryLoading(false);retryStart.current=0;
  }

  const active = sessions.find(session => session.id === selected);
  const currentArtifact = artifacts.find((item, idx) => String(item.id || item.path || idx) === selectedArtifactId) || null;
  const hasSavedAttachments = attachments.some(item => item.status === 'saved' && item.workspace_path);
  const visibleRows=historyRows.filter(session=>`${session.title || ''} ${session.agent} ${session.surface_label || ''}`.toLowerCase().includes(conversationSearch.toLowerCase())).filter(session=>quickFilter==='all'||(quickFilter==='pinned'?pinned.includes(session.id):session.live?.state==='working'||session.live?.state==='waiting_for_you'||isSessionRunningState(session.state)));
  const readOnly=active?.hosted === false || active?.source === 'imported' || (typeof active?.delivery_capability === 'object' && active.delivery_capability?.mode === 'observed_read_only');
  const connected=phase==='paired';
  const workspace=connected || ((phase==='reconnecting'||phase==='unavailable') && sessions.length>0);
  // The conversation column is part of the desktop layout, not a drawer: open it as soon as the
  // workspace exists, since a mount-time check runs before the connection brings the panes up.
  useEffect(() => {
    if (workspace && window.matchMedia('(min-width: 761px)').matches) setShowSessions(true);
  }, [workspace]);
  const startNew=()=>{setNewSession(true);setShowSessions(true);void send({type:'wsl_distros'}).catch(()=>setWslDistros([]));};
  const Heading = embedded ? 'h2' : 'h1';
  return <section ref={remoteRoot} className={`sp-remote ${embedded?'is-embedded':''} ${phase === 'paired' ? 'is-paired' : ((phase==='reconnecting'||phase==='unavailable')&&workspace)?'is-reconnecting':''}`} aria-label="Encrypted Remote conversation">
    <header className="remote-heading">
      {workspace && <button ref={sidebarToggle} className="remote-sidebar-toggle" type="button" aria-label={showSessions ? "Close conversations" : "Open conversations"} aria-controls="remote-session-sidebar" aria-expanded={showSessions} aria-pressed={showSessions} onClick={() => setShowSessions(!showSessions)}><svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true"><path d="M4 6h16M4 12h16M4 18h16"/></svg></button>}
      <div><Heading aria-live="polite">{workspace ? active?.title || 'Conversations' : 'Connect your desktop'}</Heading>{workspace && active?.live && <span className="sp-conversation-live" role="status" dangerouslySetInnerHTML={{__html:renderLiveBadge(active.live)}} />}{!workspace && <p className="remote-heading-sub">Open a linked desktop to chat with your agents from this browser.</p>}{connected && <button type="button" className="sp-conversation-limits sp-limits-trigger" aria-label="Remaining limits, open details" onClick={()=>setLimitsPanel(true)} dangerouslySetInnerHTML={{__html:renderConversationLimits(active?.header_limit !== undefined ? (active.header_limit ? [active.header_limit] : []) : (active?.limits || []).slice(0, 1))}} />}</div>
      
      {workspace && active && <button type="button" className="btn-control sp-overview-toggle" aria-pressed={overviewOpen} aria-label="Overview" title="Overview" onClick={()=>setOverviewOpen(open=>!open)}><svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true"><path d="M3 4h18v16H3z M15 4v16" /></svg><span className="visually-hidden">Overview</span></button>}<button type="button" className="remote-notif-button" aria-label="Notifications" title={notifGranted ? "Notifications active (click to test)" : "Enable notifications"} onClick={() => void toggleOrTestNotifications()}><svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke={notifGranted ? "var(--sp-accent)" : "currentColor"} strokeWidth="1.6" aria-hidden="true"><path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9m4.3 13a2 2 0 0 0 3.4 0"/></svg><span className="remote-btn-label">Notifications</span></button>
      <button type="button" className="remote-desktop-button" aria-label="Desktop connection" title="Desktop connection" onClick={()=>setDesktopPanel(true)}><svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true"><rect x="3" y="4" width="18" height="13" rx="2"/><path d="M8 21h8m-4-4v4"/></svg><span className="remote-btn-label">Connection</span></button>
    </header>
    {desktopPanel && <dialog ref={desktopDialog} className="remote-sheet" aria-labelledby="remote-desktop-heading" onClose={()=>setDesktopPanel(false)}><header><h2 id="remote-desktop-heading">Your desktop</h2><button type="button" aria-label="Close desktop connection" onClick={()=>desktopDialog.current?.close()}>Close</button></header><p>{connected ? 'Connected securely.' : proofKey.current ? 'Your connection is recovering. Your device is still remembered.' : 'Open any linked desktop through your signed-in account.'}</p><nav className="remote-sheet-links" aria-label="Remote navigation"><a href="/console"><span><strong>Dashboard</strong><small>Usage, limits and activity</small></span><svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true"><path d="m9 6 6 6-6 6"/></svg></a><a href="/console?view=settings"><span><strong>Profile</strong><small>Devices, appearance and account</small></span><svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true"><path d="m9 6 6 6-6 6"/></svg></a></nav><div className="remote-sheet-actions"><button type="button" className="remote-sheet-forget" onClick={()=>{desktopDialog.current?.close();disconnect();}}>Forget this desktop</button><button type="button" className="remote-sheet-retry" onClick={()=>{desktopDialog.current?.close();void connect(true);}}>Try again</button></div></dialog>}
    {limitsPanel && <dialog ref={limitsDialog} className="remote-sheet" aria-labelledby="remote-limits-heading" onClose={()=>setLimitsPanel(false)}><header><h2 id="remote-limits-heading">Limits</h2><button type="button" aria-label="Close limits" onClick={()=>limitsDialog.current?.close()}>Close</button></header><div dangerouslySetInnerHTML={{__html: providerUsage && providerUsage.length ? renderProviderCards(providerUsage, {cost:false}) : renderConversationLimitCard(active?.limits || [], active?.agent || '')}} /></dialog>}
    {modelPanel && <dialog ref={modelDialog} className="remote-sheet" aria-labelledby="remote-model-heading" onClose={()=>setModelPanel(false)}><header><h2 id="remote-model-heading">Change model</h2><button type="button" aria-label="Close model selection" onClick={()=>modelDialog.current?.close()}>Close</button></header><ModelChoices providers={modelProviders} loading={modelsLoading} error={modelsError} disabled={!connected} onRefresh={()=>void loadModels()} onSelect={selectModel}/></dialog>}
    {changesPanel && <dialog ref={changesDialog} className="remote-sheet sp-changes-sheet" aria-labelledby="remote-changes-heading" onClose={()=>setChangesPanel(false)}><PaneResizer cssVar="--inspector-w" label="Resize workspace panel" min={360} max={960} initial={560} direction={-1} />
      <header><h2 id="remote-changes-heading">Workspace changes</h2><div className="sp-changes-actions"><button type="button" disabled={!connected || changesLoading} onClick={()=>void loadChanges()}>Refresh</button><button type="button" aria-label="Close workspace changes" onClick={()=>changesDialog.current?.close()}>Close</button></div></header>
      <div className="sp-changes-sheet-body" onClick={event => {
        const button = (event.target as HTMLElement).closest<HTMLElement>('[data-change-file]');
        if (!button || changesLoading) return;
        const path = button.dataset.changeFile || '';
        if (changesFileMode.current) void loadChanges(path);
        else setChangesSelected(path);
      }} dangerouslySetInnerHTML={{__html:renderChangesView(changesLoading ? { loading: true } : changesState, { selected: changesSelected })}} />
    </dialog>}
    {filesPanel && <dialog ref={filesDialog} className="remote-sheet sp-files-sheet" aria-labelledby="remote-files-heading" onClose={()=>setFilesPanel(false)}><PaneResizer cssVar="--inspector-w" label="Resize workspace panel" min={360} max={960} initial={560} direction={-1} />
      <header><h2 id="remote-files-heading">Workspace files</h2><button type="button" aria-label="Close workspace files" onClick={()=>filesDialog.current?.close()}>Close</button></header>
      <form className="sp-file-form" onSubmit={event=>{event.preventDefault();void readRepoFile(filePathInput);}}>
        <label className="visually-hidden" htmlFor="remote-file-path">Relative workspace file path</label>
        <input id="remote-file-path" type="text" value={filePathInput} onChange={event=>setFilePathInput(event.target.value)} placeholder="src/main.rs" autoComplete="off" />
        <button type="submit" disabled={!connected}>Open</button>
      </form>
      <div className="sp-files-sheet-body">
        <div className="sp-files-layout">
          <WorkspaceFileTree key={selected} listing={fileListState} onBrowse={path => void browseRepoFiles(path)} onRead={path => void readRepoFile(path)} selected={fileData?.path || ''} />
          <div className="sp-file-viewer" dangerouslySetInnerHTML={{__html:renderFileViewer(fileData)}} />
        </div>
      </div>
    </dialog>}
    {artifactsPanel && <dialog ref={artifactsDialog} className="remote-sheet sp-artifacts-sheet" aria-labelledby="remote-artifacts-heading" onClose={()=>setArtifactsPanel(false)}><PaneResizer cssVar="--inspector-w" label="Resize workspace panel" min={360} max={960} initial={560} direction={-1} /><header><h2 id="remote-artifacts-heading">Artifacts{artifactsChangesCount !== null ? ` (${artifactsChangesCount} changes)` : ''}</h2><button type="button" disabled={!connected || artifactsLoading} onClick={()=>void loadArtifacts()}>Refresh</button><button type="button" aria-label="Close artifacts" onClick={()=>artifactsDialog.current?.close()}>Close</button></header><div className="sp-artifacts-sheet-body">{artifactsLoading ? <p role="status">Loading artifacts...</p> : artifactsError ? <p className="sp-empty" role="status">{artifactsError}</p> : !artifacts.length ? <p className="sp-empty">No artifacts recorded for this session yet.</p> : <><div className="sp-artifacts-list">{artifacts.map((item, idx) => { const id = String(item.id || item.path || idx); const kind = classifyArtifactKind(item); const isSelected = id === selectedArtifactId; return <button key={id} type="button" className={`sp-artifact-item${isSelected ? ' is-selected' : ''}`} aria-pressed={isSelected} onClick={()=>setSelectedArtifactId(id)}><span className="sp-artifact-badge">{kind === 'html' ? '[HTML]' : kind === 'markdown' ? '[MD]' : kind === 'image' ? '[IMG]' : '[FILE]'}</span><span className="sp-artifact-name">{item.title || item.path}</span>{item.status && <span className="sp-artifact-status">{item.status}</span>}</button>; })}</div><div className="sp-artifact-preview" dangerouslySetInnerHTML={{ __html: renderArtifactPreview(currentArtifact) }} /></>}</div></dialog>}
    {terminalOpen && <dialog ref={terminalDialog} className="remote-sheet sp-terminal-sheet" aria-labelledby="remote-terminal-heading" onClose={()=>setTerminalOpen(false)}><PaneResizer cssVar="--inspector-w" label="Resize workspace panel" min={360} max={960} initial={560} direction={-1} /><header><h2 id="remote-terminal-heading">Agent terminal</h2><button type="button" aria-label="Close terminal" onClick={()=>terminalDialog.current?.close()}>Close terminal</button></header><div className="sp-terminal-sheet-body" role="region" aria-label="Agent terminal"><p className="sp-field-note">Live agent terminal with recent history. Login links open here; desktop localhost callbacks may still need the desktop.</p><RemoteAgentTerminal key={selected} output={terminalOutput} onInput={text=>void sendTerminalInput(text)} onResize={(cols,rows)=>{terminalDimensions.current={cols,rows};}} />{terminalError && <p role="status">{terminalError}</p>}<div className="remote-terminal-keys">{[['Enter','\r'],['Tab','\t'],['Escape','\u001b'],['Up','\u001b[A'],['Down','\u001b[B'],['Ctrl+C','\u0003']].map(([label,key])=><button key={label} type="button" disabled={!connected} onClick={()=>void sendTerminalInput(key)}>{label}</button>)}</div></div></dialog>}

    {error && <p className="remote-feedback" role="alert">{error}</p>}
    {notice && <p className="remote-feedback" role="status">{notice}</p>}
    {(phase==='ready'||((phase==='unavailable'||phase==='reconnecting')&&!workspace)) && <DesktopDevices autoOpen={phase==='ready'&&!manualDisconnect.current&&!link.room} onPair={offer => {autoConnect.current = true;setLink(offer);setError('');}} />}
    {phase === 'connecting' && <p role="status">Connecting to your desktop...</p>}{(phase==='reconnecting'||phase==='unavailable') && <div className="remote-reconnect" role="status"><div><strong>{phase==='unavailable'?'Desktop unavailable':'Reconnecting to your desktop'}</strong><p>Your chat and draft are saved here. Mobile networks can interrupt the connection even when your laptop is on.</p></div><button type="button" className="btn-control" onClick={()=>void connect(true)}>Try again</button><a className="btn-control" href="/console">Dashboard</a><a className="btn-control" href="/console?view=settings">Settings</a></div>}
    {workspace && <div className={`remote-workspace ${showSessions ? 'show-sessions' : ''}`}>
      <aside className="remote-session-sidebar sp-chat-column" id="remote-session-sidebar" aria-label="Conversations"><PaneResizer cssVar="--chat-w" label="Resize conversations" min={220} max={440} initial={280} direction={1} /><div className="ide-sidebar">
      <div className="ide-sidebar-title"><h2>Conversations</h2><div className="ide-sidebar-actions"><button type="button" className="btn-control sp-sidebar-new-btn" aria-expanded={newSession} onClick={()=>{if(newSession)setNewSession(false);else startNew();}}><svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" aria-hidden="true"><path d="M12 5v14M5 12h14"/></svg>New</button><button ref={sidebarClose} type="button" className="btn-control" aria-label="Collapse conversations" title="Collapse conversations" onClick={()=>{setShowSessions(false);sidebarToggle.current?.focus();}}><svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true"><path d="M3 4h18v16H3z M9 4v16 m-2-7-2 2 2 2" /></svg></button></div></div>
      <details className="remote-new-conversation" open={newSession} onToggle={event=>setNewSession(event.currentTarget.open)}><summary className="visually-hidden">New conversation</summary>
      <form onSubmit={event => { event.preventDefault(); if(!connected || launchPending.current)return; launchPending.current=true;launchDeadline.current=Date.now()+15000;setLaunching(true);setNotice('Starting agent on your desktop...'); void send({ type:'launch', agent:launchAgent, repo_path:runIn ? wslFolder(runIn, launchRepo) : launchRepo.trim(), model:launchModel.trim() || undefined, worktree: launchWorktree || undefined, worktree_branch: launchWorktree && launchWorktreeBranch.trim() ? launchWorktreeBranch.trim() : undefined }).catch(() => {launchPending.current=false;setLaunching(false);setError('Agent could not be launched. Check the desktop connection.');}); }}>
        <label htmlFor="remote-launch-agent">Agent</label><select id="remote-launch-agent" value={launchAgent} onChange={event => setLaunchAgent(event.target.value)}><option value="claude">Claude Code</option><option value="codex">OpenAI Codex</option><option value="antigravity">Antigravity CLI</option></select>
        {wslDistros.length>0 && <><label htmlFor="remote-launch-run-in">Run in</label><select id="remote-launch-run-in" value={runIn} onChange={event => {setRunIn(event.target.value);setLaunchRepo('');}}><option value="">Windows</option>{wslDistros.map(d => <option key={d.name} value={d.name}>{d.label}{d.running?'':' (starts on launch)'}</option>)}</select></>}
        <label htmlFor="remote-launch-repo">{runIn ? `Linux folder in ${runIn}` : 'Workspace directory on desktop'}</label><input id="remote-launch-repo" value={launchRepo} placeholder={runIn ? '/home/you/project' : undefined} onChange={event => setLaunchRepo(event.target.value)} required />
        {runIn && wslFolder(runIn, launchRepo) && <p className="sp-path-line"><code title={wslFolder(runIn, launchRepo)}>{middleTruncate(wslFolder(runIn, launchRepo), 40)}</code></p>}
        <button type="button" disabled={foldersLoading} onClick={() => void browseFolders()}>Browse desktop folders</button>
        {foldersLoading && <p role="status">Loading desktop folders...</p>}
        {folderBrowser && <dialog ref={folderDialog} className="remote-folder-modal" aria-labelledby="remote-folder-heading" onClose={()=>setFolderBrowser(null)}>
          <header><h2 id="remote-folder-heading">Choose desktop folder</h2><p>{folderBrowser.path}</p></header>
          <div className="remote-folder-content">
            <div className="remote-folder-roots">{folderBrowser.roots.map(root => <button type="button" key={root} disabled={foldersLoading} onClick={() => void browseFolders(root)}>{root}</button>)}</div>
            {folderBrowser.parent && <button type="button" disabled={foldersLoading} onClick={() => void browseFolders(folderBrowser.parent!)}>Up one folder</button>}
            <div className="sp-folder-list">{folderBrowser.folders.map(folder => <button type="button" key={folder.path} disabled={foldersLoading} onClick={() => void browseFolders(folder.path)}>{folder.name}</button>)}</div>
            {!folderBrowser.folders.length && <p>No subfolders.</p>}
            {folderBrowser.truncated && <p>More folders exist. Enter a full path if your folder is not listed.</p>}
          </div>
          <footer><button type="button" onClick={()=>folderDialog.current?.close()}>Cancel</button><button type="button" className="sp-primary-action" disabled={foldersLoading} onClick={()=>{setLaunchRepo(folderBrowser.path);folderDialog.current?.close();}}>Use this folder</button></footer>
        </dialog>}
        <label htmlFor="remote-launch-model">Model (optional)</label><input id="remote-launch-model" value={launchModel} onChange={event => setLaunchModel(event.target.value)} maxLength={128} />
        <label className="sp-worktree-toggle"><input type="checkbox" checked={launchWorktree} onChange={event => setLaunchWorktree(event.target.checked)} /> Isolate in git worktree</label>
        {launchWorktree && <><label htmlFor="remote-launch-wt-branch">Worktree branch (optional)</label><input id="remote-launch-wt-branch" value={launchWorktreeBranch} onChange={event => setLaunchWorktreeBranch(event.target.value)} placeholder="gnomcode/session-branch" maxLength={120} /></>}
        <button type="submit" disabled={!connected || launching || !launchRepo.trim()}>{launching?'Launching...':'Launch Agent'}</button>
      </form></details>
        <input id="remote-history-search" type="search" aria-label="Search conversations" placeholder="Search conversations" value={conversationSearch} onChange={event=>{setConversationSearch(event.target.value);historyQuery.current={...historyQuery.current,page:1,search:event.target.value};void requestSessions();}} />
        <div className="sp-sidebar-quick-pills" role="group" aria-label="Quick conversation filters">{([['all','All'],['active','Active'],['pinned','Pinned']] as const).map(([key,label])=><button key={key} type="button" className="sp-sidebar-quick-pill" aria-pressed={quickFilter===key} onClick={()=>setQuickFilter(key)}>{label}</button>)}</div>
        <details className="conversation-filters"><summary>Filters</summary><div className="filters"><label>History <select defaultValue="managed" onChange={event=>{historyQuery.current={...historyQuery.current,page:1,source:event.target.value};void requestSessions();}}><option value="managed">Gnomcode conversations</option><option value="imported">Imported sessions (view-only)</option><option value="all">All sessions</option></select></label><label className="subagent-filter"><input type="checkbox" checked={includeSubagents} onChange={event=>{setIncludeSubagents(event.target.checked);historyQuery.current={...historyQuery.current,page:1,include_subagents:event.target.checked};void requestSessions();}} /> Include subagents</label></div></details>
        <div className="remote-session-list ide-session-list">{visibleRows.map(session => <div key={session.id} className="ide-session-row"><button type="button" className="ide-session" aria-current={session.id === selected ? 'true' : undefined} onClick={() => { selectSession(session.id); if (matchMedia('(max-width: 760px)').matches) sidebarToggle.current?.focus(); }}><span className="ide-session-title">{cleanSessionTitle(session.title) || 'New conversation'}</span>{session.last_event_at && <time dateTime={session.last_event_at}>{timeAgo(session.last_event_at)}</time>}<span className="ide-session-meta">{session.surface_label || session.agent} · {sessionStateLabel(session.state)}</span></button><button type="button" className="ide-session-more" aria-pressed={pinned.includes(session.id)} aria-label={pinned.includes(session.id)?`Unpin ${session.title || 'conversation'}`:`Pin ${session.title || 'conversation'}`} title={pinned.includes(session.id)?'Unpin':'Pin'} onClick={()=>togglePin(session.id)}><svg viewBox="0 0 24 24" width="14" height="14" fill={pinned.includes(session.id)?'currentColor':'none'} stroke="currentColor" strokeWidth="1.6" aria-hidden="true"><path d="M9 4h6l-1 6 3 3H7l3-3-1-6zM12 13v7" /></svg></button></div>)}</div>
        {!historyRows.length ? <p className="sp-field-note">No conversations yet. Start an agent here or on your desktop.</p> : !visibleRows.length && <p className="sp-field-note" role="status">No conversations match these filters.</p>}
        <nav className="pagination" aria-label="Conversation pages"><span>{historyPage} / {historyPages} · {historyTotal} conversations</span><span><button type="button" disabled={!connected || historyPage<=1} onClick={()=>{historyQuery.current.page=historyPage-1;void requestSessions();}}>Previous</button><button type="button" disabled={!connected || historyPage>=historyPages} onClick={()=>{historyQuery.current.page=historyPage+1;void requestSessions();}}>Next</button></span></nav>
      </div></aside>
      <div className="remote-conversation">
        {overviewOpen && <div className="sp-overview remote-overview" role="dialog" aria-label="Overview"><div className="sp-overview-head"><span>Overview</span><button type="button" className="sp-icon-btn" aria-label="Close overview" onClick={()=>setOverviewOpen(false)}>&#x2715;</button></div>
          <button type="button" className="sp-overview-item" aria-pressed={filesPanel} disabled={!connected} onClick={() => {setOverviewOpen(false);setTerminalOpen(false);if(filesPanel){filesDialog.current?.close();setFilesPanel(false);}else{void browseRepoFiles('');}}}>Files</button>
          <button type="button" className="sp-overview-item" aria-pressed={changesPanel} disabled={!connected} onClick={() => {setOverviewOpen(false);setTerminalOpen(false);if(changesPanel){changesDialog.current?.close();setChangesPanel(false);}else{void loadChanges();}}}>Changes</button>
          <button type="button" className="sp-overview-item" aria-pressed={artifactsPanel} disabled={!connected} onClick={() => {setOverviewOpen(false);setTerminalOpen(false);if(artifactsPanel){artifactsDialog.current?.close();setArtifactsPanel(false);}else{setArtifactsPanel(true);}}}>Artifacts</button>
          {!readOnly && <button type="button" className="sp-overview-item" aria-pressed={terminalOpen} disabled={!connected} onClick={() => {setOverviewOpen(false);if(terminalOpen){terminalDialog.current?.close();setTerminalOpen(false);}else{setFilesPanel(false);setChangesPanel(false);setArtifactsPanel(false);setTerminalOpen(true);}}}>Agent terminal</button>}
          <details className="remote-notification-settings" onToggle={event=>{if(event.currentTarget.open) {setNotificationPreferenceStatus('Loading...');void send({type:'notification_settings'}).catch(()=>setNotificationPreferenceStatus('Reconnect to load preferences.'));}}}>
            <summary>Notification preferences</summary>
            <p className="sp-field-note">Conversation scope and provider-wide warnings are separate. These choices apply to desktop and phone alerts.</p>
            {notificationPreferences && <div className="sp-notify-list">{NOTIFICATION_CHOICES.map(([key,label])=><label key={key} className="sp-notify-toggle"><input type="checkbox" checked={notificationPreferences[key]} onChange={event=>setNotificationPreferences(current=>current?{...current,[key]:event.target.checked}:current)} /><span>{label}</span></label>)}<label>Minimum turn duration (minutes)<input type="number" min={0} max={1440} value={notificationPreferences.turn_finished_min_minutes} onChange={event=>setNotificationPreferences(current=>current?{...current,turn_finished_min_minutes:Number(event.target.value)}:current)} /></label><button type="button" disabled={!connected} onClick={()=>{setNotificationPreferenceStatus('Saving...');void send({type:'notification_settings',settings:notificationPreferences}).catch(()=>setNotificationPreferenceStatus('Could not save. Reconnect and retry.'));}}>Save notification preferences</button></div>}
            <p role="status">{notificationPreferenceStatus}</p>
          </details>
          <div className="sp-overview-sep" />
          <details hidden={readOnly} className="remote-automation sp-chat-automation"><summary>Schedule and automation</summary>
          <div className="sp-auto">
          <section className="sp-auto-section" aria-labelledby="remote-auto-later"><h4 id="remote-auto-later">Send later</h4>
          <p className="sp-auto-help">Sends this message to the agent at the time you pick. Your laptop must be awake with Gnomcode running. The saved text is redacted.</p>
          <label className="sp-auto-field" htmlFor="remote-schedule-text"><span>Message</span><textarea id="remote-schedule-text" rows={3} value={scheduleText} placeholder="What should the agent do then?" onChange={event=>setScheduleText(event.target.value)} /></label>
          <div className="sp-auto-row"><label className="sp-auto-field" htmlFor="remote-schedule-time"><span>Send at (your local time)</span><input id="remote-schedule-time" type="datetime-local" value={scheduleTime} onChange={event=>setScheduleTime(event.target.value)} /></label>
          <button type="button" disabled={!connected || schedulePending || !scheduleText.trim()} onClick={()=>void scheduleDraft()}>{schedulePending?'Saving...':'Schedule'}</button>
          <button type="button" disabled={!connected} onClick={()=>requestAutomation({type:'schedules',session_id:selected})}>Refresh</button></div>
          {schedules.length ? <ul className="sp-auto-list" aria-label="Scheduled messages">{schedules.map(message=><li key={message.id}><span className="sp-auto-when">{message.due_at > 0 ? new Date(message.due_at*1000).toLocaleString() : 'Next turn'}</span><span className="sp-chip sp-chip--neutral">{message.status}</span><span className="sp-auto-text">{message.text ? message.text.slice(0,120) : ''}</span>{message.status==='pending' && <button type="button" disabled={!connected} onClick={()=>requestAutomation({type:'cancel_schedule',session_id:selected,id:message.id})}>Cancel</button>}</li>)}</ul> : <p className="sp-auto-help">Nothing scheduled.</p>}
          </section>
          <section className="sp-auto-section" aria-labelledby="remote-auto-limit"><h4 id="remote-auto-limit">When this agent reaches a limit</h4>
          <div className="sp-auto-choices" role="radiogroup" aria-labelledby="remote-auto-limit">{([['ask_first','Ask me','Show the best alternative and wait for you to pick.'],['automatic','Switch automatically','Hand off to the model or provider with the most capacity left.'],['deny_cross_provider','Switch within this provider','Only move to another model from the same provider.']] as const).map(([value,label,help])=><label key={value} className="sp-auto-choice"><input type="radio" name="remote-routing" value={value} checked={routing===value} disabled={!connected} onChange={()=>{setRouting(value);requestAutomation({type:'routing',policy:value});}} /><span><b>{label}</b><small>{help}</small></span></label>)}</div>
          <p className="sp-auto-help">Saved as soon as you pick. Switching needs a fresh limit reading; your next message continues.</p>
          </section>
          <section className="sp-auto-section" aria-labelledby="remote-auto-reset"><h4 id="remote-auto-reset">After the limit resets</h4>
          {!active?.can_reset ? <p className="sp-auto-help">This agent cannot reconnect on its own after a reset. Pick another model or start a new conversation.</p> : <div className="sp-auto-row"><p className="sp-auto-help" role="status">{resetArmed?'Armed: your laptop reconnects this conversation once a fresh reading confirms the reset.':'Off: this conversation waits for you after a reset.'}</p><button type="button" disabled={!connected} onClick={()=>requestAutomation({type:'resume_on_reset',session_id:selected,enabled:!resetArmed})}>{resetArmed?'Turn off':'Reconnect after reset'}</button></div>}
          </section>
          {automationNotices.some(item=>item.conversation_id===selected || !item.conversation_id) && <section className="sp-auto-section" aria-labelledby="remote-auto-events"><h4 id="remote-auto-events">Recent events</h4><ul className="sp-auto-list" aria-label="Recent events">{automationNotices.filter(item=>(item.conversation_id===selected || !item.conversation_id)).slice(0,8).map(item=><li key={item.id}><span className="sp-auto-text">{item.message}</span></li>)}</ul></section>}
          </div>
        </details>
        </div>}
        
        {approvals.length>0 && <section className="sp-approvals" aria-label="Waiting for your approval">{approvals.map(item=><article key={item.id} className="sp-approval">
          <div className="sp-approval-body"><p className="sp-approval-title">{approvalCopy(item.agent).agent} wants to run <strong>{item.tool}</strong></p>{item.summary && <code className="sp-approval-summary">{item.summary}</code>}<p className="sp-approval-meta">{approvalCopy(item.agent).unansweredPhone}</p></div>
          <div className="sp-approval-actions"><button type="button" disabled={!connected} aria-label={`Deny ${item.tool}`} onClick={()=>decideApproval(item.id,'deny')}>Deny</button><button type="button" disabled={!connected} aria-label={`Approve ${item.tool} once`} onClick={()=>decideApproval(item.id,'approve')}>Approve once</button></div>
        </article>)}</section>}
        {setup && setup.session_id===selected && <section className="sp-approvals" aria-label="Agent setup needed"><article className="sp-approval">
          <div className="sp-approval-body"><p className="sp-approval-title">{setup.blocker==='trust_folder'?`${setup.agent} asks whether to trust this folder`:`${setup.agent} is waiting on a setup step`}</p>{setup.blocker==='trust_folder' && setup.folder && <code className="sp-approval-summary">{setup.folder}</code>}<p className="sp-approval-meta">{setup.blocker==='trust_folder'?'Your messages wait until you answer. Trust only folders whose contents you know.':'Answer it in the agent terminal on your laptop. Your messages wait until then.'}</p></div>
          {setup.blocker==='trust_folder' && <div className="sp-approval-actions"><button type="button" disabled={!connected} onClick={()=>{void send({type:'trust_folder',session_id:setup.session_id}).catch(()=>{setError('Could not reach your laptop. Answer in the agent terminal there.');});}}>Trust folder</button></div>}
        </article></section>}
        {selected ? <>
        
        {(active?.worktree || active?.worktree_branch || active?.worktree_path) && <div className="sp-worktree-bar" role="region" aria-label="Session git worktree">
          <span className="sp-chip sp-chip--neutral">{formatWorktreeBadge(active)}</span>
          <div className="sp-worktree-actions">
            <button type="button" className="sp-remote-tool-btn" disabled={!connected || worktreeActionBusy} onClick={() => {
              if (worktreePanelOpen) { setWorktreePanelOpen(false); return; }
              setWorktreePanelOpen(true); setWorktreeDiffData({ loading: true });
              void send({ type: 'session_changes', session_id: selected }).catch(() => setWorktreeDiffData({ error: 'Worktree diff unavailable.' }));
            }}>{worktreePanelOpen ? 'Hide diff' : 'Review diff'}</button>
            <button type="button" className="sp-remote-tool-btn" disabled={!connected || worktreeActionBusy} onClick={() => setWorktreeConfirm('worktree_merge')}>Merge</button>
            <button type="button" className="sp-remote-tool-btn" disabled={!connected || worktreeActionBusy} onClick={() => setWorktreeConfirm('worktree_open_pr')}>Open PR</button>
            <button type="button" className="sp-remote-tool-btn sp-tone-red" disabled={!connected || worktreeActionBusy} onClick={() => setWorktreeConfirm('worktree_discard')}>Discard</button>
          </div>
          {worktreeConfirm && <div className="sp-confirm-inline"><p>Confirm {worktreeConfirm.replace('worktree_', '').replace('_', ' ')}?</p><button type="button" onClick={() => setWorktreeConfirm(null)}>Cancel</button><button type="button" disabled={!connected || worktreeActionBusy} onClick={() => { setWorktreeActionBusy(true); void send({ type: worktreeConfirm, session_id: selected }).catch(() => { setWorktreeActionBusy(false); setError('Worktree action requires desktop support.'); }); }}>Confirm</button></div>}
          {worktreePanelOpen && <div className="sp-worktree-diff-panel" dangerouslySetInnerHTML={{ __html: renderWorktreeDiffMarkup(worktreeDiffData) }} />}
        </div>}
        {limitMessage && <div className="sp-limit-reached" role="alert"><strong>Limit reached</strong><p>{limitMessage.replace(/^Limit reached\.\s*/, '')}</p><button type="button" onClick={()=>{setFilesPanel(false);setChangesPanel(false);setArtifactsPanel(false);setTerminalOpen(true);}}>Open agent terminal</button></div>}
        {historyLoading && <p role="status">Loading messages...</p>}
        {hasEarlier && <button type="button" disabled={!connected || historyLoading} onClick={() => {
          if (chatContainer.current) historyAnchor.current = {top:chatContainer.current.scrollTop,height:chatContainer.current.scrollHeight};
          loadingHistory.current = true; setHistoryLoading(true); loadChat(selectedRef.current, events[0]?.event_id);
        }}>Load earlier messages</button>}
        <div className="sp-remote-chat" style={{display:'block'}} ref={chatContainer} role="log" aria-label="Conversation" aria-live="polite" onClick={event=>{
          const link = (event.target as HTMLElement).closest<HTMLElement>('[data-workspace-file]');
          if (link) { event.preventDefault(); artifactLink.current = link.dataset.workspaceFile || ''; setFilesPanel(false); setChangesPanel(false); setArtifactsPanel(true); void loadArtifacts(); }
          else toggleChatMeta(event.target);
        }} />
        {active?.live?.state === 'working' && <div className="sp-activity" role="status" aria-label="Agent activity"><span dangerouslySetInnerHTML={{__html:renderActivityIndicator(active.live, active.turn_started_at, liveTick)}} />{isSessionRunningState(active.state) && <button type="button" className="btn-control sp-activity-stop" disabled={!connected || interrupting} onClick={()=>void requestInterrupt()}>Stop</button>}</div>}
        {readOnly ? <div className="remote-readonly sp-readonly-banner" role="region" aria-label="Observed session actions"><p>{active?.can_resume ? 'Continue where you left off.' : active?.can_continue_here ? 'Imported CLI session history.' : 'Observed external session history (read-only).'}</p><div className="sp-imported-actions">{active?.source === 'imported' && active?.can_continue_here && <button type="button" className="sp-primary-action" disabled={!connected || continueHereBusy} onClick={() => { setContinueHereBusy(true); setNotice('Continuing imported session in Gnomcode...'); void send({type:'continue_here',session_id:selected}).catch(() => { setContinueHereBusy(false); setError('Continue here requires desktop support.'); }); }}>{continueHereBusy ? 'Continuing...' : 'Continue here'}</button>}{active?.can_resume ? <button type="button" disabled={!connected} onClick={() => { setNotice('Resuming conversation...'); void send({type:'resume_session',session_id:selected}).catch(() => setError('Could not resume. Check your laptop connection and try again.')); }}>Resume conversation</button> : <button type="button" onClick={startNew}>New conversation</button>}</div></div> : <>
          <form className="remote-composer" onSubmit={event => { event.preventDefault(); void submitPrompt(); }}>
          {(attachments.length > 0 || attachError) && <div className="sp-attachments-bar" role="region" aria-label="Staged attachments">{attachments.map(item => <span key={item.id} className={`sp-attachment-pill sp-attachment-pill--${item.status}`}><span className="sp-attachment-name">{item.filename}</span><span className="sp-attachment-meta">{item.status === 'saved' ? 'Saved' : item.status === 'error' ? 'Failed' : 'Uploading'}</span><button type="button" className="sp-attachment-remove" aria-label={`Remove ${item.filename}`} onClick={() => { pendingUploadIds.current.delete(item.id); clearTimeout(uploadTimers.current.get(item.id)); uploadTimers.current.delete(item.id); setAttachments(current => current.filter(entry => entry.id !== item.id)); }}>Remove</button></span>)}{attachError && <span className="sp-attach-status" role="status">{attachError}</span>}</div>}
          <label htmlFor="remote-prompt">Message the agent</label>
          <textarea placeholder="Message the agent..." onKeyDown={event => { if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) { event.preventDefault(); void submitPrompt(); } }} ref={promptInput} id="remote-prompt" value={prompt} onChange={event => changePrompt(event.target.value)} maxLength={4000} rows={1} />
          <input ref={attachInput} type="file" hidden accept=".png,.jpg,.jpeg,.webp,.gif,.txt,.md,.log,.json,.csv,.pdf" onChange={event => { const file = event.target.files?.[0]; event.target.value = ''; void pickPhoneAttachment(file); }} />
          <div className="remote-composer-toolbar"><button type="button" className="remote-model-button" disabled={!connected} onClick={()=>setModelPanel(true)} aria-label="Change model">{active?.requested_model ? `${active.requested_model} (requested)` : active?.model || "Agent default"}<svg viewBox="0 0 20 20" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden="true"><path d="m5 8 5 5 5-5"/></svg></button><button type="button" className="sp-attach-trigger" disabled={!connected} onClick={()=>attachInput.current?.click()} aria-label="Attach file" title="Attach file"><svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="m21 11-8.6 8.6a5.5 5.5 0 0 1-7.8-7.8l8.6-8.6a3.7 3.7 0 0 1 5.2 5.2l-8.6 8.6a1.8 1.8 0 0 1-2.6-2.6L15 6.6"/></svg><span className="remote-btn-text">Attach</span></button>{active && isSessionRunningState(active.state) && active?.live?.state !== 'working' && <button type="button" className="sp-interrupt-btn" disabled={!connected || interrupting} aria-label="Interrupt running turn" title="Sends Esc to interrupt the active turn without closing the session" onClick={()=>void requestInterrupt()}>{interrupting ? 'Interrupting...' : 'Interrupt'}</button>}<button type="submit" disabled={!connected || sending || attachments.some(item => item.status !== 'saved' || !item.workspace_path) || (!prompt.trim() && !hasSavedAttachments) || readOnly}>{active?.can_prompt ? (sending ? 'Sending...' : 'Send') : (sending ? 'Queueing...' : 'Queue')}</button></div>
          {!connected ? <p>Reconnect to send. Your draft stays here.</p> : !active?.can_prompt && <p>{active?.state === 'waiting_for_history' ? 'Send the first message here. Complete any login or trust prompt on your desktop if needed.' : active?.state === 'idle' ? 'This session is read-only or cannot be uniquely matched to a Gnomcode terminal.' : 'Busy: messages you send now queue for the next turn.'}</p>}
        </form></>}
      </> : <div className="remote-empty"><h2>Your conversations</h2><p>Choose a conversation or start an agent to begin.</p><button type="button" className="sp-primary-action" onClick={startNew}>New conversation</button><button type="button" onClick={() => setShowSessions(true)}>Browse history</button></div>}</div>
    </div>}
  </section>;
}
