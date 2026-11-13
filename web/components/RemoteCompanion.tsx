'use client';

import { sessionStateLabel } from "../lib/console-presentation.js";
import { useEffect, useRef, useState } from 'react';
import { createChannel, generateKeyPair } from '../lib/remote-crypto.mjs';
import DesktopDevices from './DesktopDevices';
import ModelChoices, {type ModelChoice, type ProviderCatalog} from './ModelChoices';
import {supabase} from '../lib/supabase';
import {deviceProof,forgetDevice,rememberDevice,savedDevice,type DeviceOffer} from '../lib/remote-device';
import { DEFAULT_QUICK_ACTIONS, loadStoredQuickActions, MAX_QUICK_ACTIONS, toWireQuickActions, normalizeQuickAction, type QuickAction, reconcileChat, renderConversationLimitCard, renderConversationLimits, saveStoredQuickActions, type ConversationLimit, type PendingChat, updateChat } from '../lib/console-presentation';
import { disableDeviceNotifications, getNotificationPermission, requestNotificationPermission, sendAppNotification, subscribeToPushServer, triggerServerPushTest } from '../lib/notifications';

type Channel = Awaited<ReturnType<typeof createChannel>>;
type Session = { source?:string; id: string; title?: string | null; agent: string; model?: string | null; state: string; can_prompt: boolean; can_resume?:boolean; can_reset?:boolean; requested_model?:string; limits?:ConversationLimit[]; hosted?:boolean };
type ChatEvent = { event_id: string; turn_index: number; created_at: string; role: string; content: string; status?: string | null };
type FolderListing = {path:string; parent:string | null; roots:string[]; folders:{name:string; path:string}[]; truncated:boolean};
type ScheduledMessage = {id:string;due_at:number;status:string;text:string};
type AutomationNotice = {id:string;conversation_id:string;message:string;created_at:number};
type PendingApproval = {id:string;agent:string;tool:string;summary:string;expires_at:number};
type Reply = { actions?:QuickAction[]; items?:PendingApproval[]; decision?:string; reset_armed?:boolean; id?:string; messages?:ScheduledMessage[]; policy?:string; notices?:AutomationNotice[]; type: string; secret?:string; request_type?: string; message?: string; sessions?: Session[]; providers?:ProviderCatalog[]; session_id?: string; new_session_id?: string; events?: ChatEvent[]; page?:number; total_pages?:number; total_count?:number } & Partial<FolderListing>;

export default function RemoteCompanion({ embedded = false, visible = true }: { embedded?: boolean; visible?:boolean }) {
  const [link, setLink] = useState<DeviceOffer>({ room: '', key: '', relay: '' });
  const autoConnect = useRef(false);
  const [optimistic, setOptimistic] = useState<Record<string,PendingChat[]>>({});
  const [conversationSearch, setConversationSearch] = useState('');
  const [desktopPanel, setDesktopPanel] = useState(false);
  const [modelPanel, setModelPanel] = useState(false);
  const [limitsPanel, setLimitsPanel] = useState(false);
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
  const [newSession,setNewSession]=useState(false);
  const [events, setEvents] = useState<ChatEvent[]>([]);
  const [prompt, setPrompt] = useState('');
  const [scheduleTime,setScheduleTime]=useState('');
  const [resetArmed,setResetArmed]=useState(false);
  const [schedules,setSchedules]=useState<ScheduledMessage[]>([]);
  const [routing,setRouting]=useState('ask_first');
  const [automationNotices,setAutomationNotices]=useState<AutomationNotice[]>([]);
  const [schedulePending,setSchedulePending]=useState(false);
  const [approvals,setApprovals]=useState<PendingApproval[]>([]);
  const pendingSchedule=useRef<{id:string;session:string;text:string}|null>(null);
  const scheduleTimeout=useRef<ReturnType<typeof setTimeout>|null>(null);
  const [modelProviders,setModelProviders]=useState<ProviderCatalog[]>([]);
  const [modelsLoading,setModelsLoading]=useState(false);
  const [modelsError,setModelsError]=useState('');
  const modelRequest=useRef(false);
  const modelDeadline=useRef(0);
  const [launchAgent, setLaunchAgent] = useState('claude');
  const [launchRepo, setLaunchRepo] = useState('');
  const [launchModel, setLaunchModel] = useState('');
  const [notice, setNotice] = useState('');
  const [historyLoading, setHistoryLoading] = useState(false);
  const [sending, setSending] = useState(false);
  const [quickActions, setQuickActions] = useState<QuickAction[]>(() => [...DEFAULT_QUICK_ACTIONS]);
  const [quickActionsPanel, setQuickActionsPanel] = useState(false);
  const [newQuickLabel, setNewQuickLabel] = useState('');
  const [newQuickPrompt, setNewQuickPrompt] = useState('');
  const [quickActionError, setQuickActionError] = useState('');
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
  const drafts = useRef(new Map<string,string>());
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
  const quickActionsDialog=useRef<HTMLDialogElement|null>(null);
  useEffect(()=>{if(limitsPanel)limitsDialog.current?.showModal();},[limitsPanel]);
  useEffect(()=>{if(quickActionsPanel)quickActionsDialog.current?.showModal();},[quickActionsPanel]);
  useEffect(()=>{setQuickActions(loadStoredQuickActions());},[]);
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
    else if(socket.current?.readyState!==WebSocket.CONNECTING){if(reconnectTimer.current)clearTimeout(reconnectTimer.current);reconnect.current();}
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
    if(!modelPanel || phase!=='paired'){modelRequest.current=false;setModelsLoading(false);return;}
    void loadModels();
    const timer=setInterval(()=>{if(modelRequest.current && Date.now()>=modelDeadline.current){modelRequest.current=false;setModelsLoading(false);setModelsError('The desktop did not return its model list. Refresh or update Gnomcode on your desktop.');}},500);
    return()=>clearInterval(timer);
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
    const result = reconcileChat(events, optimistic[selected] || [], sessions.find(session=>session.id === selected)?.state);
    if (result.pending.length !== (optimistic[selected] || []).length) setOptimistic(current=>({...current,[selected]:result.pending}));
    updateChat(element, result.events);
    if (historyAnchor.current) { element.scrollTop = historyAnchor.current.top + element.scrollHeight - historyAnchor.current.height; historyAnchor.current = null; }
  }, [events, selected, optimistic, sessions]);

  useEffect(() => { if (autoConnect.current && link.room) {autoConnect.current = false; void connect();} });

  useEffect(() => {
    const input = promptInput.current;
    if (!input) return;
    input.style.height = 'auto';
    input.style.height = `${Math.min(160, Math.max(36, input.scrollHeight))}px`;
  }, [prompt, phase, selected]);

  function changePrompt(text: string) { promptRef.current = text; setPrompt(text); }

  async function submitPromptText(rawText: string, fromComposer: boolean) {
    if (!authenticated.current || !sessions.find(session => session.id === selectedRef.current)?.can_prompt || pendingPrompt.current || !rawText.trim()) return;
    const id = crypto.randomUUID();
    const sourceSession = selectedRef.current;
    pendingPrompt.current = {session:sourceSession,text:rawText,id,fromComposer};
    const pending:PendingChat = {id,text:rawText,created_at:new Date().toISOString(),baseline:events.map(event=>event.event_id),delivered:false};
    setOptimistic(current=>({...current,[selectedRef.current]:[...(current[selectedRef.current] || []),pending]}));
    setSending(true); setError(''); setNotice('');
    try { await send({type:'prompt',session_id:selectedRef.current,text:rawText}); }
    catch { setOptimistic(current=>({...current,[sourceSession]:(current[sourceSession] || []).filter(item=>item.id !== id)})); pendingPrompt.current = null; setSending(false); setError('Prompt could not be sent. Your draft is kept.'); }
  }

  function persistQuickActions(items: QuickAction[]) {
    setQuickActions(saveStoredQuickActions(items));
    void send({ type: 'quick_actions', action: 'set', actions: toWireQuickActions(items) }).catch(() => setQuickActionError('Saved on this phone only. Reconnect to sync quick prompts with your laptop.'));
  }

  function submitPrompt() { return submitPromptText(promptRef.current, true); }

  function submitQuickAction(action: QuickAction) {
    const clean = normalizeQuickAction(action);
    if (clean) void submitPromptText(clean.prompt, false);
  }

  function addRemoteQuickAction(event: React.FormEvent) {
    event.preventDefault();
    const clean = normalizeQuickAction({ id: `custom-${crypto.randomUUID().slice(0, 8)}`, label: newQuickLabel, prompt: newQuickPrompt, builtin: false });
    if (!clean) {
      setQuickActionError('Enter a label and plain prompt (up to 4000 characters, never starting with /).');
      return;
    }
    setQuickActionError('');
    if (quickActions.length >= MAX_QUICK_ACTIONS) { setQuickActionError(`Up to ${MAX_QUICK_ACTIONS} quick prompts. Remove one first.`); return; }
    persistQuickActions([...quickActions, clean]);
    setNewQuickLabel('');
    setNewQuickPrompt('');
  }

  async function scheduleDraft() {
    if(!authenticated.current || schedulePending || !selectedRef.current)return;
    const text=promptRef.current.trim(),due=Math.floor(new Date(scheduleTime).getTime()/1000);
    if(!text || !Number.isFinite(due) || due<=Date.now()/1000){setError('Enter a message and a future delivery time.');return;}
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
      const frame = await expectedChannel.seal(value);
      if(expectedSocket!==socket.current || epoch!==connectionEpoch.current)throw new Error('Connection changed');
      expectedSocket.send(new TextEncoder().encode(JSON.stringify(frame)));
    });
    return outgoing.current;
  }

  function loadChat(sessionId: string, before: string | null = null) {
    const epoch=connectionEpoch.current;
    void send({ type: 'chat', session_id: sessionId, before, limit: 500 }).catch(() => {if(epoch===connectionEpoch.current && socket.current?.readyState===WebSocket.OPEN && authenticated.current)setError('Chat connection lost.');});
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
    void send({type:'quick_actions',action:'get'}).catch(()=>{});
    await send({type:'routing'});
    if(epoch!==connectionEpoch.current)return;
    if (poll.current) clearInterval(poll.current);
    poll.current = setInterval(() => {
      if(launchPending.current && Date.now()>launchDeadline.current){launchPending.current=false;setLaunching(false);setError('Launch is not confirmed. Check conversations before retrying; the agent may have started.');}
      void requestSessions().catch(() => {if(epoch===connectionEpoch.current && socket.current?.readyState===WebSocket.OPEN)setError('Session refresh failed. Check the desktop connection and reconnect.');});
      if (selectedRef.current && !loadingHistory.current) loadChat(selectedRef.current);
      if(selectedRef.current)void send({type:'schedules',session_id:selectedRef.current}).catch(()=>{});
      void send({type:'notices'}).catch(()=>{});
      void send({type:'approvals'}).catch(()=>{});
    }, 2000);
    void send({type:'approvals'}).catch(()=>{});
  }

  // Decisions are sent for one exact request id; the desktop decides whether it still applies.
  function decideApproval(id:string,decision:'approve'|'deny'){
    setApprovals(current=>current.filter(item=>item.id!==id));
    void send({type:'approval',id,decision}).catch(()=>{setError('Could not reach your laptop. Claude will ask there instead.');});
  }

  async function handle(reply: Reply, epoch:number) {
    if(epoch!==connectionEpoch.current)return;
    if (reply.type === 'schedules') { if(reply.session_id===selectedRef.current){setSchedules(reply.messages || []);setResetArmed(Boolean(reply.reset_armed));} }
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
    else if(reply.type==='approval_ack'){setNotice(reply.decision==='deny'?'Denied.':'Approved once.');void send({type:'approvals'}).catch(()=>{});}
    else if(reply.type==='routing'){setRouting(reply.policy || 'ask_first');}
    else if(reply.type==='schedule_ack') {
      const pending=pendingSchedule.current;
      if(pending && pending.id===reply.id){
        if(scheduleTimeout.current)clearTimeout(scheduleTimeout.current);
        setSchedulePending(false);pendingSchedule.current=null;
        if(pending.session===selectedRef.current && promptRef.current.trim()===pending.text)changePrompt('');
        if(drafts.current.get(pending.session)?.trim()===pending.text)drafts.current.delete(pending.session);
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
    else if (reply.type === 'launch_ack') { launchPending.current=false;setLaunching(false);setNotice('Agent started. Send its first message below.');setNewSession(false); await requestSessions(); if (reply.session_id) selectSession(reply.session_id); }
    else if (reply.type === 'prompt_ack') {
      const pending = pendingPrompt.current;
      if (pending && (!reply.session_id || reply.session_id === pending.session)) {
        if (pending.fromComposer) {
          if (drafts.current.get(pending.session) === pending.text) drafts.current.delete(pending.session);
          if (pending.session === selectedRef.current && promptRef.current === pending.text) changePrompt('');
        }
        pendingPrompt.current = null; setSending(false); setOptimistic(current=>({...current,[pending.session]:(current[pending.session] || []).map(item=>item.id === pending.id ? {...item,delivered:true} : item)})); setNotice('');
      }
    }
    else if (reply.type === 'resume_session_ack') { setNotice('Ready to continue.'); await requestSessions(); if(reply.session_id) loadChat(reply.session_id); }
    else if (reply.type === 'quick_actions' && Array.isArray(reply.actions)) { setQuickActionError(''); setQuickActions(saveStoredQuickActions(reply.actions)); }
    else if (reply.type === 'models') {modelRequest.current=false;setModelsLoading(false);setModelsError('');setModelProviders(Array.isArray(reply.providers)?reply.providers:[]);}
    else if (reply.type === 'switch_ack') { setNotice('Switching model...'); await requestSessions(); }
    else if (reply.type === 'error') {
      if(reply.request_type==='approval'){void send({type:'approvals'}).catch(()=>{});}
      if(reply.request_type==='launch'){launchPending.current=false;setLaunching(false);}
      if(reply.request_type==='schedule'){if(scheduleTimeout.current)clearTimeout(scheduleTimeout.current);setSchedulePending(false);setNotice('');}
      if(reply.request_type==='models'){modelRequest.current=false;setModelsLoading(false);setModelsError('Your desktop could not provide a model list. Update Gnomcode on your desktop, then refresh.');return;}
      if (!reply.request_type || reply.request_type === 'prompt') { const pending = pendingPrompt.current; if(pending) setOptimistic(current=>({...current,[pending.session]:(current[pending.session] || []).filter(item=>item.id !== pending.id)})); pendingPrompt.current = null; setSending(false); }
      if (!reply.request_type || reply.request_type === 'workspaces') setFoldersLoading(false);
      if (!reply.request_type || reply.request_type === 'chat') { loadingHistory.current = false; setHistoryLoading(false); }
      setError(reply.message || 'Remote request failed. Reconnect and try again.');
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
      setPhase('unavailable');setError('Your desktop is not responding. Check that it is awake and Gnomcode is running. Your account connection is still saved.');return;
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
      setFolderBrowser(null); setFoldersLoading(false); if(!proofKey.current){changePrompt('');drafts.current.clear();}
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
        if(Date.now()-lastReply.current<=15000)return;
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
    setShowSessions(false);
    if (selectedRef.current) drafts.current.set(selectedRef.current, promptRef.current);
    changePrompt(drafts.current.get(id) || '');
    setHasEarlier(false); historyAnchor.current = null;
    selectedRef.current = id; setSchedules([]); setResetArmed(false); setSelected(id); setEvents([]); loadingHistory.current = true; setHistoryLoading(true);
    try {sessionStorage.setItem(`gnomcode-remote-selected:${link.room}`,id);}catch{}
    if(phase==='paired')loadChat(id);else{loadingHistory.current=false;setHistoryLoading(false);}
  }

  function disconnect() {
    try {sessionStorage.removeItem('gnomcode-pairing-offer');}catch{}
    modelRequest.current=false;setModelProviders([]);setModelsError('');setModelsLoading(false);
    authenticated.current=false;connectionEpoch.current++;
    manualDisconnect.current=true;if(reconnectTimer.current)clearTimeout(reconnectTimer.current);if(connectionWatch.current)clearInterval(connectionWatch.current);proofKey.current=null;void forgetDevice(accountOwner.current,connectedDesktop.current).catch(()=>{});
    setOptimistic({});
    const previous = socket.current; socket.current = null; previous?.close(); if (poll.current) clearInterval(poll.current);
    selectedRef.current = ''; setSelected(''); pendingPrompt.current = null; setSending(false); setError(''); setNotice('');
    setPhase('ready');setSessions([]);setEvents([]);setHistoryLoading(false);retryStart.current=0;
  }

  const active = sessions.find(session => session.id === selected);
  const readOnly=active?.hosted === false;
  const connected=phase==='paired';
  const workspace=connected || ((phase==='reconnecting'||phase==='unavailable') && sessions.length>0);
  const startNew=()=>{setNewSession(true);setShowSessions(true);};
  const Heading = embedded ? 'h2' : 'h1';
  return <section ref={remoteRoot} className={`sp-remote ${embedded?'is-embedded':''} ${phase === 'paired' ? 'is-paired' : (phase==='reconnecting'||(phase==='unavailable'&&workspace))?'is-reconnecting':''}`} aria-label="Encrypted Remote conversation">
    <header className="remote-heading">
      {workspace && <button ref={sidebarToggle} className="remote-sidebar-toggle" type="button" aria-label={showSessions ? "Close conversations" : "Open conversations"} aria-controls="remote-session-sidebar" aria-expanded={showSessions} onClick={() => setShowSessions(!showSessions)}><svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true"><path d="M4 6h16M4 12h16M4 18h16"/></svg></button>}
      <div><Heading aria-live="polite">{workspace ? active?.title || 'Conversations' : 'Connect your desktop'}</Heading>{!workspace && <p className="remote-heading-sub">Open a linked desktop to chat with your agents from this browser.</p>}{connected && <button type="button" className="sp-conversation-limits sp-limits-trigger" aria-label="Remaining limits, open details" onClick={()=>setLimitsPanel(true)} dangerouslySetInnerHTML={{__html:renderConversationLimits(active?.limits || [])}} />}</div>
      {connected && <button type="button" className="remote-new" aria-label="New conversation" onClick={startNew}><svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true"><path d="M12 4v16M4 12h16"/></svg></button>}
      <button type="button" className="remote-notif-button" aria-label="Notifications" title={notifGranted ? "Notifications active (click to test)" : "Enable notifications"} onClick={() => void toggleOrTestNotifications()}><svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke={notifGranted ? "var(--sp-accent)" : "currentColor"} strokeWidth="1.6" aria-hidden="true"><path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9m4.3 13a2 2 0 0 0 3.4 0"/></svg><span className="remote-btn-label">Notifications</span></button>
      <button type="button" className="remote-desktop-button" aria-label="Desktop connection" title="Desktop connection" onClick={()=>setDesktopPanel(true)}><svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true"><rect x="3" y="4" width="18" height="13" rx="2"/><path d="M8 21h8m-4-4v4"/></svg><span className="remote-btn-label">Connection</span></button>
    </header>
    {desktopPanel && <dialog ref={desktopDialog} className="remote-sheet" aria-labelledby="remote-desktop-heading" onClose={()=>setDesktopPanel(false)}><header><h2 id="remote-desktop-heading">Your desktop</h2><button type="button" aria-label="Close desktop connection" onClick={()=>desktopDialog.current?.close()}>Close</button></header><p>{connected ? 'Connected securely.' : proofKey.current ? 'Your connection is recovering. Your device is still remembered.' : 'Open any linked desktop through your signed-in account.'}</p><nav aria-label="Remote navigation"><a href="/console">Dashboard</a><a href="/console?view=settings">Profile</a><button type="button" onClick={()=>{desktopDialog.current?.close();void connect(true);}}>Try again</button><button type="button" onClick={()=>{desktopDialog.current?.close();disconnect();}}>Forget this desktop</button></nav></dialog>}
    {limitsPanel && <dialog ref={limitsDialog} className="remote-sheet" aria-labelledby="remote-limits-heading" onClose={()=>setLimitsPanel(false)}><header><h2 id="remote-limits-heading">Limits</h2><button type="button" aria-label="Close limits" onClick={()=>limitsDialog.current?.close()}>Close</button></header><div dangerouslySetInnerHTML={{__html:renderConversationLimitCard(active?.limits || [], active?.agent || '')}} /></dialog>}
    {modelPanel && <dialog ref={modelDialog} className="remote-sheet" aria-labelledby="remote-model-heading" onClose={()=>setModelPanel(false)}><header><h2 id="remote-model-heading">Change model</h2><button type="button" aria-label="Close model selection" onClick={()=>modelDialog.current?.close()}>Close</button></header><ModelChoices providers={modelProviders} loading={modelsLoading} error={modelsError} disabled={!connected} onRefresh={()=>void loadModels()} onSelect={selectModel}/></dialog>}
    {quickActionsPanel && <dialog ref={quickActionsDialog} className="remote-sheet" aria-labelledby="remote-quick-actions-heading" onClose={()=>setQuickActionsPanel(false)}>
      <header><h2 id="remote-quick-actions-heading">Quick prompts</h2><button type="button" aria-label="Close quick prompts" onClick={()=>quickActionsDialog.current?.close()}>Close</button></header>
      <p className="sp-field-note">Canned prompts are sent as plain chat messages, never shell commands.</p>
      <div className="sp-quick-actions-list">{quickActions.map(item=><div key={item.id} className="sp-quick-action-row"><div><strong>{item.label}</strong><p>{item.prompt}</p></div><button type="button" disabled={quickActions.length<=1} onClick={()=>persistQuickActions(quickActions.filter(row=>row.id!==item.id))}>Remove</button></div>)}</div>
      <form className="sp-quick-actions-form" onSubmit={addRemoteQuickAction}>
        <label htmlFor="remote-quick-label">Button label</label>
        <input id="remote-quick-label" value={newQuickLabel} onChange={event=>setNewQuickLabel(event.target.value)} maxLength={40} required placeholder="Review tests" />
        <label htmlFor="remote-quick-prompt">Prompt message</label>
        <textarea id="remote-quick-prompt" value={newQuickPrompt} onChange={event=>setNewQuickPrompt(event.target.value)} maxLength={4000} rows={3} required placeholder="Run the test suite and summarize any failures." />
        {quickActionError && <p className="remote-feedback" role="alert">{quickActionError}</p>}
        <div className="dialog-actions"><button type="button" onClick={()=>{setQuickActionError('');persistQuickActions([...DEFAULT_QUICK_ACTIONS]);}}>Reset defaults</button><button type="submit" className="sp-primary-action">Add prompt</button></div>
      </form>
    </dialog>}

    {error && <p className="remote-feedback" role="alert">{error}</p>}
    {notice && <p className="remote-feedback" role="status">{notice}</p>}
    {(phase==='ready'||(phase==='unavailable'&&!workspace)) && <DesktopDevices autoOpen={phase==='ready'&&!manualDisconnect.current&&!link.room} onPair={offer => {autoConnect.current = true;setLink(offer);setError('');}} />}
    {phase === 'connecting' && <p role="status">Connecting to your desktop...</p>}{(phase==='reconnecting'||phase==='unavailable') && <div className="remote-reconnect" role="status"><div><strong>{phase==='unavailable'?'Desktop unavailable':'Reconnecting to your desktop'}</strong><p>Your chat and draft are saved here. Mobile networks can interrupt the connection even when your laptop is on.</p></div><button type="button" onClick={()=>void connect(true)}>Try again</button><a href="/console">Dashboard</a><a href="/console?view=settings">Profile</a></div>}
    {workspace && <div className={`remote-workspace ${showSessions ? 'show-sessions' : ''}`}>
      <aside className="remote-session-sidebar" id="remote-session-sidebar" aria-label="Conversations"><div className="remote-drawer-heading"><h2>Conversations</h2><button ref={sidebarClose} type="button" aria-label="Close conversations" onClick={()=>{setShowSessions(false);sidebarToggle.current?.focus();}}>Close</button></div>
      <details open={newSession} onToggle={event=>setNewSession(event.currentTarget.open)}><summary>New conversation</summary>
      <form onSubmit={event => { event.preventDefault(); if(!connected || launchPending.current)return; launchPending.current=true;launchDeadline.current=Date.now()+15000;setLaunching(true);setNotice('Starting agent on your desktop...'); void send({ type:'launch', agent:launchAgent, repo_path:launchRepo.trim(), model:launchModel.trim() || undefined }).catch(() => {launchPending.current=false;setLaunching(false);setError('Agent could not be launched. Check the desktop connection.');}); }}>
        <label htmlFor="remote-launch-agent">Agent</label><select id="remote-launch-agent" value={launchAgent} onChange={event => setLaunchAgent(event.target.value)}><option value="claude">Claude Code</option><option value="codex">OpenAI Codex</option><option value="gemini">Gemini CLI</option><option value="antigravity">Antigravity CLI</option></select>
        <label htmlFor="remote-launch-repo">Workspace directory on desktop</label><input id="remote-launch-repo" value={launchRepo} onChange={event => setLaunchRepo(event.target.value)} required />
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
        <button type="submit" disabled={!connected || launching || !launchRepo.trim()}>{launching?'Launching...':'Launch Agent'}</button>
      </form></details>
        <label className="remote-history-search" htmlFor="remote-history-search">Search titles and agents<input id="remote-history-search" type="search" value={conversationSearch} onChange={event=>{setConversationSearch(event.target.value);historyQuery.current={...historyQuery.current,page:1,search:event.target.value};void requestSessions();}} placeholder="Search conversation metadata" /></label>
        <label>Show<select defaultValue="managed" onChange={event=>{historyQuery.current={...historyQuery.current,page:1,source:event.target.value};void requestSessions();}}><option value="managed">Gnomcode conversations</option><option value="imported">Imported sessions (view-only)</option><option value="all">All sessions</option></select></label>
        <label className="subagent-filter"><input type="checkbox" checked={includeSubagents} onChange={event=>{setIncludeSubagents(event.target.checked);historyQuery.current={...historyQuery.current,page:1,include_subagents:event.target.checked};void requestSessions();}} /> Include subagents</label><div className="remote-session-list">{historyRows.filter(session=>`${session.title || ''} ${session.agent}`.toLowerCase().includes(conversationSearch.toLowerCase())).map(session => <button type="button" key={session.id} aria-current={session.id === selected ? 'true' : undefined} onClick={() => { selectSession(session.id); if (matchMedia('(max-width: 760px)').matches) sidebarToggle.current?.focus(); }}><strong>{session.title || 'New conversation'}</strong><span>{session.agent} · {sessionStateLabel(session.state)}</span>{session.state === "ended" && <small>Session ended</small>}</button>)}</div>
        <nav aria-label="Conversation pages"><button type="button" disabled={!connected || historyPage<=1} onClick={()=>{historyQuery.current.page=historyPage-1;void requestSessions();}}>Previous</button><span>{historyPage} / {historyPages} · {historyTotal} conversations</span><button type="button" disabled={!connected || historyPage>=historyPages} onClick={()=>{historyQuery.current.page=historyPage+1;void requestSessions();}}>Next</button></nav>{!historyRows.length ? <p>No conversations yet. Start an agent here or on your desktop.</p> : !historyRows.some(session=>`${session.title || ''} ${session.agent}`.toLowerCase().includes(conversationSearch.toLowerCase())) && <p role="status">No conversations match your search.</p>}
      </aside>
      <div className="remote-conversation">
        {approvals.length>0 && <section className="sp-approvals" aria-label="Waiting for your approval">{approvals.map(item=><article key={item.id} className="sp-approval">
          <div className="sp-approval-body"><p className="sp-approval-title">{item.agent==='claude-code'?'Claude Code':item.agent} wants to run <strong>{item.tool}</strong></p>{item.summary && <code className="sp-approval-summary">{item.summary}</code>}<p className="sp-approval-meta">Then Claude asks on your laptop if you do not answer.</p></div>
          <div className="sp-approval-actions"><button type="button" disabled={!connected} aria-label={`Deny ${item.tool}`} onClick={()=>decideApproval(item.id,'deny')}>Deny</button><button type="button" disabled={!connected} aria-label={`Approve ${item.tool} once`} onClick={()=>decideApproval(item.id,'approve')}>Approve once</button></div>
        </article>)}</section>}
        {selected ? <>
        {historyLoading && <p role="status">Loading messages...</p>}
        {hasEarlier && <button type="button" disabled={!connected || historyLoading} onClick={() => {
          if (chatContainer.current) historyAnchor.current = {top:chatContainer.current.scrollTop,height:chatContainer.current.scrollHeight};
          loadingHistory.current = true; setHistoryLoading(true); loadChat(selectedRef.current, events[0]?.event_id);
        }}>Load earlier messages</button>}
        <div className="sp-remote-chat" style={{display:'block'}} ref={chatContainer} role="log" aria-label="Conversation" aria-live="polite" />
        <details hidden={active?.source === 'imported'} className="remote-automation"><summary>Schedule and automation</summary>
          <p>Saved on your laptop after confirmation. Keep it awake with Gnomcode running. Message text is redacted before storage.</p>
          <label htmlFor="remote-schedule-time">Delivery time (your local time)</label><input id="remote-schedule-time" type="datetime-local" value={scheduleTime} onChange={event=>setScheduleTime(event.target.value)} />
          <button type="button" disabled={!connected || schedulePending || !prompt.trim()} onClick={()=>void scheduleDraft()}>{schedulePending?'Saving...':'Schedule current draft'}</button>
          <button type="button" disabled={!connected} onClick={()=>requestAutomation({type:'schedules',session_id:selected})}>Refresh saved messages</button>
          <div aria-label="Scheduled messages">{schedules.map(message=><p key={message.id}>{new Date(message.due_at*1000).toLocaleString()} - {message.status} {message.text && <span>{message.text.slice(0,120)}</span>} {message.status==='pending' && <button type="button" disabled={!connected} onClick={()=>requestAutomation({type:'cancel_schedule',session_id:selected,id:message.id})}>Cancel</button>}</p>)}</div>
          <label htmlFor="remote-routing">When an agent reaches a limit</label><select id="remote-routing" value={routing} disabled={!connected} onChange={event=>{setRouting(event.target.value);requestAutomation({type:'routing',policy:event.target.value});}}><option value="ask_first">Let me choose</option><option value="automatic">Automatically switch model or provider</option><option value="deny_cross_provider">Automatically switch within provider</option></select>
          <p>Switching needs fresh known quota. Send your next message to continue; previous tasks are not replayed.</p>
          <p role="status">{resetArmed?'Reset reconnection armed. Waiting for fresh quota confirmation.':'Reset reconnection is not armed.'}</p>
          <button type="button" disabled={!connected || !active?.can_reset} onClick={()=>requestAutomation({type:'resume_on_reset',session_id:selected,enabled:true})}>Reconnect after reset</button><button type="button" disabled={!connected} onClick={()=>requestAutomation({type:'resume_on_reset',session_id:selected,enabled:false})}>Cancel reset reconnection</button>
          <div aria-label="Recent events">{automationNotices.filter(item=>(item.conversation_id===selected || !item.conversation_id)).slice(0,8).map(item=><p key={item.id}>{item.message}</p>)}</div>
        </details>{readOnly ? <div className="remote-readonly"><p>{active?.can_resume ? 'Continue where you left off.' : 'Read-only history.'}</p>{active?.can_resume ? <button type="button" disabled={!connected} onClick={() => { setNotice('Resuming conversation...'); void send({type:'resume_session',session_id:selected}).catch(() => setError('Could not resume. Check your laptop connection and try again.')); }}>Resume conversation</button> : <button type="button" onClick={startNew}>New conversation</button>}</div> : <>
          <div className="sp-quick-actions" role="toolbar" aria-label="Quick prompts">
            {quickActions.map(action => <button key={action.id} type="button" className="sp-quick-action" title={action.prompt} disabled={!connected || sending || !active?.can_prompt} onClick={() => submitQuickAction(action)}>{action.label}</button>)}
            <button type="button" className="sp-quick-action sp-quick-action--manage" aria-label="Manage quick prompts" onClick={() => setQuickActionsPanel(true)}>Customize</button>
          </div>
          <form className="remote-composer" onSubmit={event => { event.preventDefault(); void submitPrompt(); }}>
          <label htmlFor="remote-prompt">Message the agent</label>
          <textarea placeholder="Message the agent..." onKeyDown={event => { if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) { event.preventDefault(); void submitPrompt(); } }} ref={promptInput} id="remote-prompt" value={prompt} onChange={event => changePrompt(event.target.value)} maxLength={4000} rows={1} />
          <div className="remote-composer-toolbar"><button type="button" className="remote-model-button" disabled={!connected} onClick={()=>setModelPanel(true)} aria-label="Change model">{active?.requested_model ? "Switching model..." : active?.model || "Agent default"}<svg viewBox="0 0 20 20" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden="true"><path d="m5 8 5 5 5-5"/></svg></button><button type="submit" disabled={!connected || sending || !prompt.trim() || !active?.can_prompt}>{sending ? 'Sending...' : 'Send'}</button></div>
          {!connected ? <p>Reconnect to send. Your draft stays here.</p> : !active?.can_prompt && <p>{active?.state === 'waiting_for_history' ? 'Send the first message here. Complete any login or trust prompt on your desktop if needed.' : active?.state === 'idle' ? 'This session is read-only or cannot be uniquely matched to a Gnomcode terminal.' : 'The agent is busy or waiting for approval. Prompting becomes available when it is idle.'}</p>}
        </form></>}
      </> : <div className="remote-empty"><h2>Your conversations</h2><p>Choose a conversation or start an agent to begin.</p><button type="button" className="sp-primary-action" onClick={startNew}>New conversation</button><button type="button" onClick={() => setShowSessions(true)}>Browse history</button></div>}</div>
    </div>}
  </section>;
}
