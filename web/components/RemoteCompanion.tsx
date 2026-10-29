'use client';

import { useEffect, useRef, useState } from 'react';
import { createChannel, generateKeyPair } from '../lib/remote-crypto.mjs';
import DesktopDevices from './DesktopDevices';
import { reconcileChat, type PendingChat, updateChat } from '../lib/console-presentation';

type Channel = Awaited<ReturnType<typeof createChannel>>;
type Session = { id: string; title?: string | null; agent: string; model?: string | null; state: string; can_prompt: boolean };
type ChatEvent = { event_id: string; turn_index: number; created_at: string; role: string; content: string; status?: string | null };
type FolderListing = {path:string; parent:string | null; roots:string[]; folders:{name:string; path:string}[]; truncated:boolean};
type Reply = { type: string; request_type?: string; message?: string; sessions?: Session[]; session_id?: string; new_session_id?: string; events?: ChatEvent[] } & Partial<FolderListing>;

export default function RemoteCompanion({ embedded = false }: { embedded?: boolean }) {
  const [link, setLink] = useState({ room: '', key: '', relay: '' });
  const autoConnect = useRef(false);
  const [optimistic, setOptimistic] = useState<Record<string,PendingChat[]>>({});
  const [phase, setPhase] = useState<'ready' | 'connecting' | 'verify' | 'paired'>('ready');
  const [code, setCode] = useState('');
  const [error, setError] = useState('');
  const [sessions, setSessions] = useState<Session[]>([]);
  const [selected, setSelected] = useState('');
  const [showSessions, setShowSessions] = useState(false);
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

  useEffect(() => {
    try {
      const value = sessionStorage.getItem('spawnt-pairing-offer');
      sessionStorage.removeItem('spawnt-pairing-offer');
      if (value) { const offer = JSON.parse(value); if (Date.parse(offer.expires_at) > Date.now()) {autoConnect.current = true; setLink(offer);} }
    } catch { setError('Pairing code expired. Generate a fresh code on your desktop.'); }
    return () => { if (poll.current) clearInterval(poll.current); socket.current?.close(); };
  }, []);

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
    outgoing.current = outgoing.current.catch(() => {}).then(async () => {
      if (!channel.current || socket.current?.readyState !== WebSocket.OPEN) throw new Error('Connection closed');
      const frame = await channel.current.seal(value);
      socket.current.send(new TextEncoder().encode(JSON.stringify(frame)));
    });
    return outgoing.current;
  }

  function loadChat(sessionId: string, before: string | null = null) {
    void send({ type: 'chat', session_id: sessionId, before, limit: 500 }).catch(() => setError('Chat connection lost.'));
  }

  async function beginPaired() {
    setPhase('paired');
    await send({type:'sessions'});
    if (poll.current) clearInterval(poll.current);
    poll.current = setInterval(() => {
      void send({type:'sessions'}).catch(() => setError('Session refresh failed.'));
      if (selectedRef.current && !loadingHistory.current) loadChat(selectedRef.current);
    }, 2000);
  }

  async function handle(reply: Reply) {
    if (reply.type === 'confirm') {
      desktopConfirmed.current = true;
      if (phoneConfirmed.current) await beginPaired();
    } else if (reply.type === 'workspaces' && reply.path && reply.roots && reply.folders) {
      setFoldersLoading(false); setFolderBrowser({path:reply.path,parent:reply.parent || null,roots:reply.roots,folders:reply.folders,truncated:Boolean(reply.truncated)});
    } else if (reply.type === 'sessions') {
      setSessions(reply.sessions || []);
      if (!selectedRef.current && reply.sessions?.length) selectSession(reply.sessions[0].id);
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
    } else if (reply.type === 'launch_ack') { setNotice('Agent launched. Waiting for its conversation...'); await send({ type: 'sessions' }); if (reply.session_id) selectSession(reply.session_id); }
    else if (reply.type === 'prompt_ack') {
      const pending = pendingPrompt.current;
      if (pending && (!reply.session_id || reply.session_id === pending.session)) {
        if (drafts.current.get(pending.session) === pending.text) drafts.current.delete(pending.session);
        if (pending.session === selectedRef.current && promptRef.current === pending.text) changePrompt('');
        pendingPrompt.current = null; setSending(false); setOptimistic(current=>({...current,[pending.session]:(current[pending.session] || []).map(item=>item.id === pending.id ? {...item,delivered:true} : item)})); setNotice('');
      }
    }
    else if (reply.type === 'switch_ack') { setNotice('New model session started. Waiting for the agent to confirm it.'); await send({ type: 'sessions' }); if (reply.new_session_id) selectSession(reply.new_session_id); }
    else if (reply.type === 'error') {
      if (!reply.request_type || reply.request_type === 'prompt') { const pending = pendingPrompt.current; if(pending) setOptimistic(current=>({...current,[pending.session]:(current[pending.session] || []).filter(item=>item.id !== pending.id)})); pendingPrompt.current = null; setSending(false); }
      if (!reply.request_type || reply.request_type === 'workspaces') setFoldersLoading(false);
      if (!reply.request_type || reply.request_type === 'chat') { loadingHistory.current = false; setHistoryLoading(false); }
      setError(reply.message || 'Remote request failed.');
    }
  }

  async function connect() {
    setError('');
    setNotice('');
    if (!/^[a-zA-Z0-9_-]{16,128}$/.test(link.room) || !/^[0-9a-f]{64}$/.test(link.key)
      || !(/^wss:\/\//.test(link.relay) || /^ws:\/\/(127\.0\.0\.1|localhost):/.test(link.relay))) {
      setError('Pairing code is invalid or expired. Generate a new code on your desktop.'); return;
    }
    try {
      setOptimistic({});
      selectedRef.current = ''; setSelected(''); setSessions([]); setEvents([]);
      pendingPrompt.current = null; setSending(false); loadingHistory.current = false; setHistoryLoading(false);
      setFolderBrowser(null); setFoldersLoading(false); changePrompt(''); drafts.current.clear();
      setPhase('connecting');
      setConfirmed(false);
      phoneConfirmed.current = false;
      desktopConfirmed.current = false;
      chunks.current = '';
      outgoing.current = Promise.resolve();
      incoming.current = Promise.resolve();
      const keys = await generateKeyPair();
      channel.current = await createChannel(keys, link.key, link.room, 'phone');
      setCode(channel.current.verifyCode);
      const ws = new WebSocket(`${link.relay.replace(/\/$/, '')}/${link.room}?role=phone`);
      ws.binaryType = 'arraybuffer'; socket.current = ws;
      ws.onopen = () => { ws.send(new TextEncoder().encode(JSON.stringify({ type: 'hello', public_key: keys.publicHex }))); setPhase('verify'); };
      ws.onmessage = event => {
        incoming.current = incoming.current.then(async () => {
          if (socket.current !== ws) return;
          const frame = JSON.parse(new TextDecoder().decode(event.data as ArrayBuffer));
          const message = await channel.current!.open(frame) as Reply & { data?: string; final?: boolean };
          if (message.type !== 'chunk' || typeof message.data !== 'string') throw new Error('Invalid response');
          chunks.current += message.data;
          if (message.final) {
            const complete = JSON.parse(chunks.current) as Reply;
            chunks.current = '';
            await handle(complete);
          }
        }).catch(() => { setError('Encrypted message verification failed.'); ws.close(); });
      };
      ws.onclose = () => { if (socket.current !== ws) return; if (poll.current) clearInterval(poll.current); pendingPrompt.current = null; setSending(false); setPhase('ready'); setError('Connection closed. Pair again from the desktop.'); };
    } catch { setPhase('ready'); setError('Could not start encrypted pairing.'); }
  }

  async function confirm() {
    try {
      await send({ type: 'confirm' }); phoneConfirmed.current = true; setConfirmed(true);
      if (desktopConfirmed.current) await beginPaired();
    } catch { setError('Could not confirm pairing.'); }
  }

  function selectSession(id: string) {
    setShowSessions(false);
    if (selectedRef.current) drafts.current.set(selectedRef.current, promptRef.current);
    changePrompt(drafts.current.get(id) || '');
    setHasEarlier(false); historyAnchor.current = null;
    selectedRef.current = id; setSelected(id); setEvents([]); loadingHistory.current = true; setHistoryLoading(true);
    loadChat(id);
  }

  function disconnect() {
    setOptimistic({});
    const previous = socket.current; socket.current = null; previous?.close(); if (poll.current) clearInterval(poll.current);
    selectedRef.current = ''; setSelected(''); pendingPrompt.current = null; setSending(false); setError(''); setNotice('');
    setPhase('ready'); setCode(''); setSessions([]); setEvents([]); setConfirmed(false); setHistoryLoading(false);
  }

  const active = sessions.find(session => session.id === selected);
  return <section className={`sp-remote ${phase === 'paired' ? 'is-paired' : ''}`} aria-label="Encrypted Remote conversation">
    <header className="remote-heading"><button className="remote-sidebar-toggle" type="button" aria-label="Toggle conversations" aria-controls="remote-session-sidebar" aria-expanded={showSessions} onClick={() => setShowSessions(!showSessions)}><svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true"><path d="M3 4h18v16H3z M9 4v16" /></svg></button><div><h1>{phase === 'paired' ? active?.title || 'Workspace' : 'Connect your desktop'}</h1>{phase === 'paired' && <p>{active?.agent || 'Encrypted conversation'}</p>}</div><details key={phase} className="remote-actions"><summary>Actions</summary><div><a href="/console?view=settings">Profile</a>{phase === 'paired' && <><p>Encrypted connection active.</p>        <details><summary>Switch model</summary><form onSubmit={event => { event.preventDefault(); void send({ type: 'switch', session_id: selected, model: targetModel }).catch(() => setError('Model switch could not be sent.')); }}><label htmlFor="remote-model">Switch to model</label><input id="remote-model" value={targetModel} onChange={event => setTargetModel(event.target.value)} maxLength={128} /><button type="submit" disabled={!targetModel.trim()}>Handoff and switch</button></form></details><button type="button" onClick={disconnect}>Disconnect</button></>}</div></details></header>
    {error && <p className="remote-feedback" role="alert">{error}</p>}
    {notice && <p className="remote-feedback" role="status">{notice}</p>}
    {phase === 'ready' && <DesktopDevices onPair={offer => {autoConnect.current = true;setLink(offer);setError('');}} />}
    {phase === 'connecting' && <p role="status">Connecting to the relay...</p>}
    {phase === 'verify' && <section><h2>Verify this device</h2><p>Compare this code with the code on your desktop. Confirm on both devices only if they match.</p><strong style={{ fontSize: 32, letterSpacing: 6 }}>{code}</strong><p><button type="button" onClick={confirm} disabled={confirmed}>Codes match</button></p>{confirmed && <p role="status">Waiting for desktop confirmation...</p>}<button type="button" onClick={disconnect}>Disconnect</button></section>}
    {phase === 'paired' && <div className={`remote-workspace ${showSessions ? 'show-sessions' : ''}`}>
      <aside className="remote-session-sidebar" id="remote-session-sidebar" aria-label="Conversations">
      <details><summary>New agent session</summary>
      <form onSubmit={event => { event.preventDefault(); void send({ type:'launch', agent:launchAgent, repo_path:launchRepo.trim(), model:launchModel.trim() || undefined }).catch(() => setError('Agent could not be launched. Check the desktop connection.')); }}>
        <label htmlFor="remote-launch-agent">Agent</label><select id="remote-launch-agent" value={launchAgent} onChange={event => setLaunchAgent(event.target.value)}><option value="claude">Claude Code</option><option value="codex">OpenAI Codex</option><option value="gemini">Gemini CLI</option></select>
        <label htmlFor="remote-launch-repo">Workspace directory on desktop</label><input id="remote-launch-repo" value={launchRepo} onChange={event => setLaunchRepo(event.target.value)} required />
        <button type="button" disabled={foldersLoading} onClick={() => void browseFolders()}>Browse desktop folders</button>
        {foldersLoading && <p role="status">Loading desktop folders...</p>}
        {folderBrowser && <section className="sp-folder-browser" aria-label="Desktop folders">
          <p>{folderBrowser.path}</p>
          <div>{folderBrowser.roots.map(root => <button type="button" key={root} disabled={foldersLoading} onClick={() => void browseFolders(root)}>{root}</button>)}</div>
          {folderBrowser.parent && <button type="button" disabled={foldersLoading} onClick={() => void browseFolders(folderBrowser.parent!)}>Up one folder</button>}
          <div className="sp-folder-list">{folderBrowser.folders.map(folder => <button type="button" key={folder.path} disabled={foldersLoading} onClick={() => void browseFolders(folder.path)}>{folder.name}</button>)}</div>
          {!folderBrowser.folders.length && <p>No subfolders.</p>}
          {folderBrowser.truncated && <p>More folders exist. Enter a full path if your folder is not listed.</p>}
          <button type="button" onClick={() => {setLaunchRepo(folderBrowser.path);setFolderBrowser(null);}}>Use this folder</button>
          <button type="button" onClick={() => setFolderBrowser(null)}>Cancel folder selection</button>
        </section>}
        <label htmlFor="remote-launch-model">Model (optional)</label><input id="remote-launch-model" value={launchModel} onChange={event => setLaunchModel(event.target.value)} maxLength={128} />
        <button type="submit" disabled={!launchRepo.trim()}>Launch Agent</button>
      </form></details>
        <div className="remote-list-heading"><h2>Conversations</h2><button type="button" onClick={() => void send({type:'sessions'}).catch(() => setError('Sessions could not be refreshed. Reconnect and retry.'))}>Refresh</button></div>
        <div className="remote-session-list">{sessions.map(session => <button type="button" key={session.id} aria-current={session.id === selected ? 'true' : undefined} onClick={() => selectSession(session.id)}><strong>{session.title || 'New conversation'}</strong><span>{session.agent} · {session.state.replaceAll('_',' ')}</span></button>)}</div>
        {!sessions.length && <p>No conversations yet. Start an agent here or on your desktop.</p>}
      </aside>
      <div className="remote-conversation">{selected ? <>
        {historyLoading && <p role="status">Loading messages...</p>}
        {hasEarlier && <button type="button" disabled={historyLoading} onClick={() => {
          if (chatContainer.current) historyAnchor.current = {top:chatContainer.current.scrollTop,height:chatContainer.current.scrollHeight};
          loadingHistory.current = true; setHistoryLoading(true); loadChat(selectedRef.current, events[0]?.event_id);
        }}>Load earlier messages</button>}
        <div className="sp-remote-chat" style={{display:'block'}} ref={chatContainer} role="log" aria-label="Conversation" aria-live="polite" />
        <form className="remote-composer" onSubmit={event => { event.preventDefault(); void submitPrompt(); }}>
          <label htmlFor="remote-prompt">Message the agent</label>
          <textarea placeholder="Message the agent..." onKeyDown={event => { if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) { event.preventDefault(); void submitPrompt(); } }} ref={promptInput} id="remote-prompt" value={prompt} onChange={event => changePrompt(event.target.value)} maxLength={4000} rows={1} />
          <div className="remote-composer-toolbar"><span>{active?.model || "Agent default"}</span><button type="submit" disabled={sending || !prompt.trim() || !active?.can_prompt}>{sending ? 'Sending...' : 'Send'}</button></div>
          {!active?.can_prompt && <p>{active?.state === 'waiting_for_history' ? 'Waiting for the agent to record its first turn. Complete login or trust prompts on the desktop.' : active?.state === 'idle' ? 'This session is read-only or cannot be uniquely matched to a Spawnt terminal.' : 'The agent is busy or waiting for approval. Prompting becomes available when it is idle.'}</p>}
        </form>
      </> : <div className="remote-empty"><h2>Your conversations</h2><p>Choose a conversation or start an agent to begin.</p><button type="button" onClick={() => setShowSessions(true)}>Show conversations</button></div>}</div>
    </div>}
  </section>;
}
