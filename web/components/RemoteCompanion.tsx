'use client';

import { sessionStateLabel } from "../lib/console-presentation.js";
import { useEffect, useRef, useState } from 'react';
import { createChannel, generateKeyPair } from '../lib/remote-crypto.mjs';
import DesktopDevices from './DesktopDevices';
import {supabase} from '../lib/supabase';
import {deviceProof,forgetDevice,rememberDevice,savedDevice} from '../lib/remote-device';
import { reconcileChat, renderConversationLimits, type ConversationLimit, type PendingChat, updateChat } from '../lib/console-presentation';

type Channel = Awaited<ReturnType<typeof createChannel>>;
type Session = { id: string; title?: string | null; agent: string; model?: string | null; state: string; can_prompt: boolean; can_resume?:boolean; requested_model?:string; limits?:ConversationLimit[]; hosted?:boolean };
type ChatEvent = { event_id: string; turn_index: number; created_at: string; role: string; content: string; status?: string | null };
type FolderListing = {path:string; parent:string | null; roots:string[]; folders:{name:string; path:string}[]; truncated:boolean};
type Reply = { type: string; secret?:string; request_type?: string; message?: string; sessions?: Session[]; session_id?: string; new_session_id?: string; events?: ChatEvent[] } & Partial<FolderListing>;

export default function RemoteCompanion({ embedded = false }: { embedded?: boolean }) {
  const [link, setLink] = useState<{room:string;key:string;relay:string;protocol_version?:number}>({ room: '', key: '', relay: '' });
  const autoConnect = useRef(false);
  const [optimistic, setOptimistic] = useState<Record<string,PendingChat[]>>({});
  const [phase, setPhase] = useState<'ready' | 'connecting' | 'reconnecting' | 'verify' | 'paired'>('ready');
  const [code, setCode] = useState('');
  const [error, setError] = useState('');
  const [sessions, setSessions] = useState<Session[]>([]);
  const [selected, setSelected] = useState('');
  const [showSessions, setShowSessions] = useState(false);
  const [newSession,setNewSession]=useState(false);
  const [events, setEvents] = useState<ChatEvent[]>([]);
  const [prompt, setPrompt] = useState('');
  const [targetModel, setTargetModel] = useState('');
  const [launchAgent, setLaunchAgent] = useState('claude');
  const [launchRepo, setLaunchRepo] = useState('');
  const [launchModel, setLaunchModel] = useState('');
  const [notice, setNotice] = useState('');
  const [confirmed, setConfirmed] = useState(false);
  const [historyLoading, setHistoryLoading] = useState(false);
  const [sending, setSending] = useState(false);
  const [hasEarlier, setHasEarlier] = useState(false);
  const [folderBrowser, setFolderBrowser] = useState<FolderListing | null>(null);
  const [foldersLoading, setFoldersLoading] = useState(false);
  const promptInput = useRef<HTMLTextAreaElement | null>(null);
  const chatContainer = useRef<HTMLDivElement | null>(null);
  const historyAnchor = useRef<{top:number; height:number} | null>(null);
  const drafts = useRef(new Map<string,string>());
  const promptRef = useRef('');
  const pendingPrompt = useRef<{session:string; text:string; id:string} | null>(null);
  const socket = useRef<WebSocket | null>(null);
  const channel = useRef<Channel | null>(null);
  const phoneConfirmed = useRef(false);
  const desktopConfirmed = useRef(false);
  const selectedRef = useRef('');
  const loadingHistory = useRef(false);
  const poll = useRef<ReturnType<typeof setInterval> | null>(null);
  const incoming = useRef(Promise.resolve());
  const outgoing = useRef(Promise.resolve());
  const chunks = useRef('');
  const proofKey=useRef<CryptoKey|null>(null);
  const reconnectTimer=useRef<ReturnType<typeof setTimeout>|null>(null);
  const manualDisconnect=useRef(false);
  const restoredSession=useRef('');
  const folderDialog=useRef<HTMLDialogElement|null>(null);
  const remoteRoot=useRef<HTMLElement|null>(null);
  const reconnect=useRef(()=>{});
  const connectionEpoch=useRef(0);

  useEffect(() => {
    try {
      const value = sessionStorage.getItem('spawnt-pairing-offer');
      sessionStorage.removeItem('spawnt-pairing-offer');
      if (value) { const offer = JSON.parse(value); if (Date.parse(offer.expires_at) > Date.now()) {autoConnect.current = true; setLink(offer);} }
      else void savedDevice().then(saved=>{if(saved){proofKey.current=saved.proofKey;autoConnect.current=true;setLink(saved.offer);}}).catch(()=>{});
    } catch { setError('Pairing code expired. Generate a fresh code on your desktop.'); }
    return () => {manualDisconnect.current=true;if(reconnectTimer.current)clearTimeout(reconnectTimer.current);if (poll.current) clearInterval(poll.current); socket.current?.close(); };
  }, []);

  useEffect(()=>{
    const update=()=>remoteRoot.current?.style.setProperty('--remote-viewport-height',`${window.visualViewport?.height || innerHeight}px`);
    if(innerWidth>760)setShowSessions(true);
    update();window.visualViewport?.addEventListener('resize',update);window.addEventListener('resize',update);
    return()=>{window.visualViewport?.removeEventListener('resize',update);window.removeEventListener('resize',update);};
  },[]);

  useEffect(()=>{const visible=()=>{if(document.visibilityState==='visible' && proofKey.current && socket.current?.readyState !== WebSocket.OPEN && socket.current?.readyState !== WebSocket.CONNECTING){if(reconnectTimer.current)clearTimeout(reconnectTimer.current);reconnect.current();}};document.addEventListener('visibilitychange',visible);return()=>document.removeEventListener('visibilitychange',visible);},[]);

  useEffect(()=>{const {data:{subscription}}=supabase.auth.onAuthStateChange(event=>{if(event==='SIGNED_OUT')disconnect();});return()=>subscription.unsubscribe();},[]);

  useEffect(()=>{
    if(phase!=='paired')return;
    let last=0;const active=()=>{if(Date.now()-last>60000){last=Date.now();void send({type:'activity'}).catch(()=>{});}};
    window.addEventListener('pointerdown',active);window.addEventListener('keydown',active);window.addEventListener('wheel',active,{passive:true});
    return()=>{window.removeEventListener('pointerdown',active);window.removeEventListener('keydown',active);window.removeEventListener('wheel',active);};
  },[phase]);

  useEffect(()=>{if(folderBrowser && !folderDialog.current?.open)folderDialog.current?.showModal();},[folderBrowser]);

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

  async function submitPrompt() {
    if (!sessions.find(session => session.id === selectedRef.current)?.can_prompt || pendingPrompt.current || !promptRef.current.trim()) return;
    const id = crypto.randomUUID();
    const sourceSession = selectedRef.current;
    pendingPrompt.current = {session:sourceSession,text:promptRef.current,id};
    const pending:PendingChat = {id,text:promptRef.current,created_at:new Date().toISOString(),baseline:events.map(event=>event.event_id),delivered:false};
    setOptimistic(current=>({...current,[selectedRef.current]:[...(current[selectedRef.current] || []),pending]}));
    setSending(true); setError(''); setNotice('');
    try { await send({type:'prompt',session_id:selectedRef.current,text:promptRef.current}); }
    catch { setOptimistic(current=>({...current,[sourceSession]:(current[sourceSession] || []).filter(item=>item.id !== id)})); pendingPrompt.current = null; setSending(false); setError('Prompt could not be sent. Your draft is kept.'); }
  }

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
    void send({ type: 'chat', session_id: sessionId, before, limit: 500 }).catch(() => {if(epoch===connectionEpoch.current && socket.current?.readyState===WebSocket.OPEN)setError('Chat connection lost.');});
  }

  async function beginPaired() {
    const epoch=connectionEpoch.current;
    setPhase('paired');
    await send({type:'sessions'});
    if(epoch!==connectionEpoch.current)return;
    if (poll.current) clearInterval(poll.current);
    poll.current = setInterval(() => {
      void send({type:'sessions'}).catch(() => {if(epoch===connectionEpoch.current && socket.current?.readyState===WebSocket.OPEN)setError('Session refresh failed. Check the desktop connection and reconnect.');});
      if (selectedRef.current && !loadingHistory.current) loadChat(selectedRef.current);
    }, 2000);
  }

  async function handle(reply: Reply, epoch:number) {
    if(epoch!==connectionEpoch.current)return;
    if (reply.type === 'confirm') {
      desktopConfirmed.current = true;
      if(phoneConfirmed.current && link.protocol_version!==2)await beginPaired();
    } else if (reply.type === 'workspaces' && reply.path && reply.roots && reply.folders) {
      setFoldersLoading(false); setFolderBrowser({path:reply.path,parent:reply.parent || null,roots:reply.roots,folders:reply.folders,truncated:Boolean(reply.truncated)});
    } else if (reply.type === 'sessions') {
      setSessions(reply.sessions || []);
      if (!selectedRef.current && reply.sessions?.length) {
        const preferred=reply.sessions.find(session=>session.id===restoredSession.current) || reply.sessions.find(session=>session.can_prompt);
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
    else if(reply.type==='resume_rejected'){manualDisconnect.current=true;proofKey.current=null;await forgetDevice();socket.current?.close();setPhase('ready');setError('Device trust expired or was revoked. Pair this desktop again.');}
    else if (reply.type === 'launch_ack') { setNotice('Agent started. Send its first message below.');setNewSession(false); await send({ type: 'sessions' }); if (reply.session_id) selectSession(reply.session_id); }
    else if (reply.type === 'prompt_ack') {
      const pending = pendingPrompt.current;
      if (pending && (!reply.session_id || reply.session_id === pending.session)) {
        if (drafts.current.get(pending.session) === pending.text) drafts.current.delete(pending.session);
        if (pending.session === selectedRef.current && promptRef.current === pending.text) changePrompt('');
        pendingPrompt.current = null; setSending(false); setOptimistic(current=>({...current,[pending.session]:(current[pending.session] || []).map(item=>item.id === pending.id ? {...item,delivered:true} : item)})); setNotice('');
      }
    }
    else if (reply.type === 'resume_session_ack') { setNotice('Ready to continue.'); await send({type:'sessions'}); if(reply.session_id) loadChat(reply.session_id); }
    else if (reply.type === 'switch_ack') { setNotice('Switching model...'); await send({ type: 'sessions' }); }
    else if (reply.type === 'error') {
      if (!reply.request_type || reply.request_type === 'prompt') { const pending = pendingPrompt.current; if(pending) setOptimistic(current=>({...current,[pending.session]:(current[pending.session] || []).filter(item=>item.id !== pending.id)})); pendingPrompt.current = null; setSending(false); }
      if (!reply.request_type || reply.request_type === 'workspaces') setFoldersLoading(false);
      if (!reply.request_type || reply.request_type === 'chat') { loadingHistory.current = false; setHistoryLoading(false); }
      setError(reply.message || 'Remote request failed. Reconnect and try again.');
    }
  }

  async function connect() {
    if(socket.current?.readyState===WebSocket.OPEN || socket.current?.readyState===WebSocket.CONNECTING)return;
    const epoch=++connectionEpoch.current;
    const previous=socket.current;socket.current=null;previous?.close();channel.current=null;
    manualDisconnect.current=false;
    try{restoredSession.current=sessionStorage.getItem(`spawnt-remote-selected:${link.room}`)||'';}catch{}
    setError('');
    setNotice('');
    if (!/^[a-zA-Z0-9_-]{16,128}$/.test(link.room) || !/^[0-9a-f]{64}$/.test(link.key)
      || !(/^wss:\/\//.test(link.relay) || /^ws:\/\/(127\.0\.0\.1|localhost):/.test(link.relay))) {
      setError('Pairing code is invalid or expired. Generate a new code on your desktop.'); return;
    }
    try {
      setOptimistic({});
      if(selectedRef.current)drafts.current.set(selectedRef.current,promptRef.current);
      selectedRef.current = ''; setSelected(''); setSessions([]); setEvents([]);
      pendingPrompt.current = null; setSending(false); loadingHistory.current = false; setHistoryLoading(false);
      setFolderBrowser(null); setFoldersLoading(false); changePrompt(''); if(!proofKey.current)drafts.current.clear();
      setPhase(proofKey.current ? 'reconnecting' : 'connecting');
      setConfirmed(false);
      phoneConfirmed.current = false;
      desktopConfirmed.current = false;
      chunks.current = '';
      outgoing.current = Promise.resolve();
      incoming.current = Promise.resolve();
      const keys = await generateKeyPair();
      if(epoch!==connectionEpoch.current)return;
      if(link.protocol_version!==2){const legacy=await createChannel(keys,link.key,link.room,'phone');if(epoch!==connectionEpoch.current)return;channel.current=legacy;setCode(legacy.verifyCode);}
      const ws = new WebSocket(`${link.relay.replace(/\/$/, '')}/${link.room}?role=phone`);
      ws.binaryType = 'arraybuffer'; socket.current = ws;
      let helloTimer:ReturnType<typeof setInterval>|undefined;
      const hello=()=>{if(ws.readyState===WebSocket.OPEN)ws.send(new TextEncoder().encode(JSON.stringify({type:'hello',public_key:keys.publicHex})));};
      ws.onopen = () => { hello();if(link.protocol_version===2)helloTimer=setInterval(hello,2000);else setPhase('verify'); };
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
            else {setCode(fresh.verifyCode);setPhase('verify');}
            return;
          }
          const message = await channel.current!.open(frame) as Reply & { data?: string; final?: boolean };
          if(epoch!==connectionEpoch.current || socket.current!==ws)return;
          if (message.type !== 'chunk' || typeof message.data !== 'string') throw new Error('Invalid response');
          chunks.current += message.data;
          if (message.final) {
            const complete = JSON.parse(chunks.current) as Reply;
            chunks.current = '';
            await handle(complete,epoch);
          }
        }).catch(() => {if(epoch!==connectionEpoch.current)return;setError('Could not verify or save this device connection. Try pairing again.'); ws.close(); });
      };
      ws.onclose = () => {clearInterval(helloTimer);if (socket.current !== ws || manualDisconnect.current) return; if (poll.current) clearInterval(poll.current); pendingPrompt.current = null; setSending(false);if(proofKey.current){setPhase('reconnecting');reconnectTimer.current=setTimeout(()=>void connect(),1500);}else{setPhase('ready');setError('Connection closed before pairing completed. Start pairing again.');}};
    } catch { setPhase('ready'); setError('Could not start encrypted pairing.'); }
  }
  reconnect.current=()=>void connect();

  async function confirm() {
    try {
      await send({ type: 'confirm' }); phoneConfirmed.current = true; setConfirmed(true);
      if(desktopConfirmed.current && link.protocol_version!==2)await beginPaired();
    } catch { setError('Could not confirm pairing.'); }
  }

  function selectSession(id: string) {
    setShowSessions(false);
    if (selectedRef.current) drafts.current.set(selectedRef.current, promptRef.current);
    changePrompt(drafts.current.get(id) || '');
    setHasEarlier(false); historyAnchor.current = null;
    selectedRef.current = id; setSelected(id); setEvents([]); loadingHistory.current = true; setHistoryLoading(true);
    try {sessionStorage.setItem(`spawnt-remote-selected:${link.room}`,id);}catch{}
    loadChat(id);
  }

  function disconnect() {
    connectionEpoch.current++;
    manualDisconnect.current=true;if(reconnectTimer.current)clearTimeout(reconnectTimer.current);proofKey.current=null;void forgetDevice().catch(()=>{});
    setOptimistic({});
    const previous = socket.current; socket.current = null; previous?.close(); if (poll.current) clearInterval(poll.current);
    selectedRef.current = ''; setSelected(''); pendingPrompt.current = null; setSending(false); setError(''); setNotice('');
    setPhase('ready'); setCode(''); setSessions([]); setEvents([]); setConfirmed(false); setHistoryLoading(false);
  }

  const active = sessions.find(session => session.id === selected);
  const readOnly=active?.hosted === false;
  const startNew=()=>{setNewSession(true);setShowSessions(true);};
  return <section ref={remoteRoot} className={`sp-remote ${embedded?'is-embedded':''} ${phase === 'paired' ? 'is-paired' : phase==='reconnecting'?'is-reconnecting':''}`} aria-label="Encrypted Remote conversation">
    <header className="remote-heading"><button className="remote-sidebar-toggle" type="button" aria-label="Toggle conversations" aria-controls="remote-session-sidebar" aria-expanded={showSessions} onClick={() => setShowSessions(!showSessions)}><svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true"><path d="M3 4h18v16H3z M9 4v16" /></svg></button><div><h1>{phase === 'paired' ? active?.title || 'Workspace' : 'Connect your desktop'}</h1>{phase === 'paired' && <><p>{active?.agent || 'Encrypted conversation'}</p><div className="sp-conversation-limits" role="status" aria-label="Remaining limits" dangerouslySetInnerHTML={{__html:renderConversationLimits(active?.limits || [])}} /></>}</div>{phase === 'paired' && <button type="button" className="remote-new" aria-label="New conversation" onClick={startNew}>New</button>}<details key={phase} className="remote-actions"><summary>Actions</summary><div><a href="/console?view=settings">Profile</a>{phase === 'paired' && <><p>Encrypted connection active.</p>        <details><summary>Switch model</summary><form onSubmit={event => { event.preventDefault(); void send({ type: 'switch', session_id: selected, model: targetModel }).catch(() => setError('Model switch could not be sent.')); }}><label htmlFor="remote-model">Switch to model</label><input id="remote-model" value={targetModel} onChange={event => setTargetModel(event.target.value)} maxLength={128} /><button type="submit" disabled={!targetModel.trim()}>Switch model</button></form></details><button type="button" onClick={disconnect}>Disconnect</button></>}</div></details></header>
    {error && <p className="remote-feedback" role="alert">{error}</p>}
    {notice && <p className="remote-feedback" role="status">{notice}</p>}
    {phase === 'ready' && <DesktopDevices onPair={offer => {autoConnect.current = true;setLink(offer);setError('');}} />}
    {phase === 'connecting' && <p role="status">Connecting to the relay...</p>}{phase === 'reconnecting' && <div className="remote-empty"><h2>Reconnecting to your desktop</h2><p>Your device is remembered. Keep the desktop running; your conversation will return when connected.</p><button type="button" onClick={disconnect}>Forget this desktop</button></div>}
    {phase === 'verify' && <section><h2>Verify this device</h2><p>Enter this code in desktop Profile, then confirm here. The desktop checks the encrypted connection before allowing access.</p><strong style={{ fontSize: 32, letterSpacing: 6 }}>{code}</strong><p><button type="button" onClick={confirm} disabled={confirmed}>Codes match</button></p>{confirmed && <p role="status">Waiting for desktop confirmation...</p>}<button type="button" onClick={disconnect}>Disconnect</button></section>}
    {phase === 'paired' && <div className={`remote-workspace ${showSessions ? 'show-sessions' : ''}`}>
      <aside className="remote-session-sidebar" id="remote-session-sidebar" aria-label="Conversations">
      <details open={newSession} onToggle={event=>setNewSession(event.currentTarget.open)}><summary>New conversation</summary>
      <form onSubmit={event => { event.preventDefault(); void send({ type:'launch', agent:launchAgent, repo_path:launchRepo.trim(), model:launchModel.trim() || undefined }).catch(() => setError('Agent could not be launched. Check the desktop connection.')); }}>
        <label htmlFor="remote-launch-agent">Agent</label><select id="remote-launch-agent" value={launchAgent} onChange={event => setLaunchAgent(event.target.value)}><option value="claude">Claude Code</option><option value="codex">OpenAI Codex</option><option value="gemini">Gemini CLI</option><option value="antigravity">Antigravity CLI</option></select>
        <label htmlFor="remote-launch-repo">Workspace directory on desktop</label><input id="remote-launch-repo" value={launchRepo} onChange={event => setLaunchRepo(event.target.value)} required />
        <button type="button" disabled={foldersLoading} onClick={() => void browseFolders()}>Browse desktop folders</button>
        {foldersLoading && <p role="status">Loading desktop folders...</p>}
        {folderBrowser && <dialog ref={folderDialog} className="remote-folder-modal" onClose={()=>setFolderBrowser(null)}>
          <header><h2>Choose desktop folder</h2><p>{folderBrowser.path}</p></header>
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
        <button type="submit" disabled={!launchRepo.trim()}>Launch Agent</button>
      </form></details>
        <div className="remote-list-heading"><h2>Conversations</h2><button type="button" onClick={() => void send({type:'sessions'}).catch(() => setError('Sessions could not be refreshed. Reconnect and retry.'))}>Refresh</button></div>
        <div className="remote-session-list">{sessions.map(session => <button type="button" key={session.id} aria-current={session.id === selected ? 'true' : undefined} onClick={() => selectSession(session.id)}><strong>{session.title || 'New conversation'}</strong><span>{session.agent} · {sessionStateLabel(session.state)}</span>{session.state === "ended" && <small>Session ended</small>}</button>)}</div>
        {!sessions.length && <p>No conversations yet. Start an agent here or on your desktop.</p>}
      </aside>
      <div className="remote-conversation">{selected ? <>
        {historyLoading && <p role="status">Loading messages...</p>}
        {hasEarlier && <button type="button" disabled={historyLoading} onClick={() => {
          if (chatContainer.current) historyAnchor.current = {top:chatContainer.current.scrollTop,height:chatContainer.current.scrollHeight};
          loadingHistory.current = true; setHistoryLoading(true); loadChat(selectedRef.current, events[0]?.event_id);
        }}>Load earlier messages</button>}
        <div className="sp-remote-chat" style={{display:'block'}} ref={chatContainer} role="log" aria-label="Conversation" aria-live="polite" />
        {readOnly ? <div className="remote-readonly"><p>{active?.can_resume ? 'Continue where you left off.' : 'Read-only history.'}</p>{active?.can_resume ? <button type="button" onClick={() => { setNotice('Resuming conversation...'); void send({type:'resume_session',session_id:selected}).catch(() => setError('Could not resume. Check your laptop connection and try again.')); }}>Resume conversation</button> : <button type="button" onClick={startNew}>New conversation</button>}</div> : <form className="remote-composer" onSubmit={event => { event.preventDefault(); void submitPrompt(); }}>
          <label htmlFor="remote-prompt">Message the agent</label>
          <textarea placeholder="Message the agent..." onKeyDown={event => { if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) { event.preventDefault(); void submitPrompt(); } }} ref={promptInput} id="remote-prompt" value={prompt} onChange={event => changePrompt(event.target.value)} maxLength={4000} rows={1} />
          <div className="remote-composer-toolbar"><span>{active?.requested_model ? "Switching model..." : active?.model || "Agent default"}</span><button type="submit" disabled={sending || !prompt.trim() || !active?.can_prompt}>{sending ? 'Sending...' : 'Send'}</button></div>
          {!active?.can_prompt && <p>{active?.state === 'waiting_for_history' ? 'Send the first message here. Complete any login or trust prompt on your desktop if needed.' : active?.state === 'idle' ? 'This session is read-only or cannot be uniquely matched to a Spawnt terminal.' : 'The agent is busy or waiting for approval. Prompting becomes available when it is idle.'}</p>}
        </form>}
      </> : <div className="remote-empty"><h2>Your conversations</h2><p>Choose a conversation or start an agent to begin.</p><button type="button" className="sp-primary-action" onClick={startNew}>New conversation</button><button type="button" onClick={() => setShowSessions(true)}>Browse history</button></div>}</div>
    </div>}
  </section>;
}
