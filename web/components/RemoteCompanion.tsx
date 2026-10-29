'use client';

import { useEffect, useRef, useState } from 'react';
import { createChannel, generateKeyPair } from '../lib/remote-crypto.mjs';

type Channel = Awaited<ReturnType<typeof createChannel>>;
type Session = { id: string; title?: string | null; agent: string; state: string; can_prompt: boolean };
type ChatEvent = { event_id: string; turn_index: number; created_at: string; role: string; content: string };
type Reply = { type: string; message?: string; sessions?: Session[]; session_id?: string; new_session_id?: string; events?: ChatEvent[] };

export default function RemoteCompanion({ embedded = false }: { embedded?: boolean }) {
  const [link, setLink] = useState({ room: '', key: '', relay: '' });
  const [pairingInput, setPairingInput] = useState('');
  const [phase, setPhase] = useState<'ready' | 'connecting' | 'verify' | 'paired'>('ready');
  const [code, setCode] = useState('');
  const [error, setError] = useState('');
  const [sessions, setSessions] = useState<Session[]>([]);
  const [selected, setSelected] = useState('');
  const [events, setEvents] = useState<ChatEvent[]>([]);
  const [prompt, setPrompt] = useState('');
  const [targetModel, setTargetModel] = useState('');
  const [launchAgent, setLaunchAgent] = useState('claude');
  const [launchRepo, setLaunchRepo] = useState('');
  const [launchModel, setLaunchModel] = useState('');
  const [notice, setNotice] = useState('');
  const [confirmed, setConfirmed] = useState(false);
  const [historyLoading, setHistoryLoading] = useState(false);
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
    const params = new URLSearchParams(location.hash.slice(1));
    setLink({ room: params.get('room') || '', key: params.get('key') || '', relay: params.get('relay') || '' });
    return () => { if (poll.current) clearInterval(poll.current); socket.current?.close(); };
  }, []);

  function usePairingLink() {
    try {
      const url = new URL(pairingInput);
      if (url.origin !== location.origin || url.pathname !== '/remote') throw new Error('Invalid pairing link');
      const params = new URLSearchParams(url.hash.slice(1));
      setLink({ room: params.get('room') || '', key: params.get('key') || '', relay: params.get('relay') || '' });
      setError('');
    } catch { setError('Paste the full pairing link shown on your desktop.'); }
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

  async function handle(reply: Reply) {
    if (reply.type === 'confirm') {
      desktopConfirmed.current = true;
      if (phoneConfirmed.current) { setPhase('paired'); await send({ type: 'sessions' }); }
    } else if (reply.type === 'sessions') {
      setSessions(reply.sessions || []);
    } else if (reply.type === 'chat' && reply.session_id === selectedRef.current) {
      const page = reply.events || [];
      setEvents(current => {
        const all = new Map(current.map(event => [event.event_id, event]));
        page.forEach(event => all.set(event.event_id, event));
        return [...all.values()].sort((a, b) => a.turn_index - b.turn_index || a.created_at.localeCompare(b.created_at) || a.event_id.localeCompare(b.event_id));
      });
      if (loadingHistory.current && page.length === 500 && page[0]) loadChat(reply.session_id, page[0].event_id);
      else if (loadingHistory.current) {
        loadingHistory.current = false;
        setHistoryLoading(false);
        if (poll.current) clearInterval(poll.current);
        poll.current = setInterval(() => { loadChat(selectedRef.current); void send({ type: 'sessions' }).catch(() => setError('Session refresh failed.')); }, 2000);
      }
    } else if (reply.type === 'launch_ack') { setNotice('Agent launched. Refresh sessions after the CLI starts.'); await send({ type: 'sessions' }); }
    else if (reply.type === 'prompt_ack') setPrompt('');
    else if (reply.type === 'switch_ack') { setNotice(`New session ${reply.new_session_id || ''} launched.`); await send({ type: 'sessions' }); if (reply.new_session_id) selectSession(reply.new_session_id); }
    else if (reply.type === 'error') setError(reply.message || 'Remote request failed.');
  }

  async function connect() {
    setError('');
    setNotice('');
    if (!/^[a-zA-Z0-9_-]{16,128}$/.test(link.room) || !/^[0-9a-f]{64}$/.test(link.key)
      || !(/^wss:\/\//.test(link.relay) || /^ws:\/\/(127\.0\.0\.1|localhost):/.test(link.relay))) {
      setError('Use a valid pairing link from your desktop.'); return;
    }
    try {
      setPhase('connecting');
      setConfirmed(false);
      phoneConfirmed.current = false;
      desktopConfirmed.current = false;
      chunks.current = '';
      outgoing.current = Promise.resolve();
      const keys = await generateKeyPair();
      channel.current = await createChannel(keys, link.key, link.room, 'phone');
      setCode(channel.current.verifyCode);
      const ws = new WebSocket(`${link.relay.replace(/\/$/, '')}/${link.room}?role=phone`);
      ws.binaryType = 'arraybuffer'; socket.current = ws;
      ws.onopen = () => { ws.send(new TextEncoder().encode(JSON.stringify({ type: 'hello', public_key: keys.publicHex }))); setPhase('verify'); };
      ws.onmessage = event => {
        incoming.current = incoming.current.then(async () => {
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
      ws.onclose = () => { if (poll.current) clearInterval(poll.current); setPhase('ready'); setError('Connection closed. Pair again from the desktop.'); };
    } catch { setPhase('ready'); setError('Could not start encrypted pairing.'); }
  }

  async function confirm() {
    try {
      await send({ type: 'confirm' }); phoneConfirmed.current = true; setConfirmed(true);
      if (desktopConfirmed.current) { setPhase('paired'); await send({ type: 'sessions' }); }
    } catch { setError('Could not confirm pairing.'); }
  }

  function selectSession(id: string) {
    selectedRef.current = id; setSelected(id); setEvents([]); loadingHistory.current = true; setHistoryLoading(true);
    if (poll.current) clearInterval(poll.current);
    loadChat(id);
  }

  function disconnect() {
    socket.current?.close(); if (poll.current) clearInterval(poll.current);
    setPhase('ready'); setCode(''); setSessions([]); setEvents([]); setConfirmed(false); setHistoryLoading(false);
  }

  const active = sessions.find(session => session.id === selected);
  return <section className="sp-remote" aria-label="Encrypted Remote conversation">
    <h1>{embedded ? 'Encrypted chat' : 'Spawnt Remote'}</h1>
    {error && <p role="alert">{error}</p>}
    {notice && <p role="status">{notice}</p>}
    {phase === 'ready' && <section><p>On the desktop, open Settings and choose Start pairing. Open the generated link here, compare the six-digit codes, then confirm on both devices.</p>{embedded && <div><label htmlFor="remote-pairing-link">Desktop pairing link</label><input id="remote-pairing-link" type="url" value={pairingInput} onChange={event => setPairingInput(event.target.value)} /><button type="button" onClick={usePairingLink}>Use link</button></div>}{link.room ? <button type="button" onClick={connect}>Connect securely</button> : <p role="status">Waiting for a desktop pairing link.</p>}</section>}
    {phase === 'connecting' && <p role="status">Connecting to the relay...</p>}
    {phase === 'verify' && <section><h2>Verify this device</h2><p>Compare this code with the code on your desktop. Confirm on both devices only if they match.</p><strong style={{ fontSize: 32, letterSpacing: 6 }}>{code}</strong><p><button type="button" onClick={confirm} disabled={confirmed}>Codes match</button></p>{confirmed && <p role="status">Waiting for desktop confirmation...</p>}<button type="button" onClick={disconnect}>Disconnect</button></section>}
    {phase === 'paired' && <section>
      <p role="status">Encrypted connection active.</p><button type="button" onClick={disconnect}>Disconnect</button>
      <h2>New agent session</h2>
      <form onSubmit={event => { event.preventDefault(); void send({ type:'launch', agent:launchAgent, repo_path:launchRepo.trim(), model:launchModel.trim() || undefined }).catch(() => setError('Agent could not be launched. Check the desktop connection.')); }}>
        <label htmlFor="remote-launch-agent">Agent</label><select id="remote-launch-agent" value={launchAgent} onChange={event => setLaunchAgent(event.target.value)}><option value="claude">Claude Code</option><option value="codex">OpenAI Codex</option><option value="gemini">Gemini CLI</option></select>
        <label htmlFor="remote-launch-repo">Workspace directory on desktop</label><input id="remote-launch-repo" value={launchRepo} onChange={event => setLaunchRepo(event.target.value)} required />
        <label htmlFor="remote-launch-model">Model (optional)</label><input id="remote-launch-model" value={launchModel} onChange={event => setLaunchModel(event.target.value)} maxLength={128} />
        <button type="submit" disabled={!launchRepo.trim()}>Launch Agent</button>
      </form>
      <h2>Sessions</h2><button type="button" onClick={() => void send({ type: 'sessions' }).catch(() => setError('Sessions could not be refreshed. Reconnect and retry.'))}>Refresh sessions</button>
      <select aria-label="Agent session" value={selected} onChange={event => selectSession(event.target.value)}><option value="">Choose a session</option>{sessions.map(session => <option key={session.id} value={session.id}>{session.title || session.id} — {session.agent} ({session.state})</option>)}</select>
      {selected && <><h2>Conversation</h2>{historyLoading && <p role="status">Loading available history...</p>}<div className="sp-remote-chat" aria-live="polite">{events.filter(event => event.role === 'user' || event.role === 'assistant').map(event => <article className={`sp-chat-${event.role}`} key={event.event_id}><strong>{event.role === 'user' ? 'You' : 'Agent'}</strong><p>{event.content}</p></article>)}</div><form onSubmit={event => { event.preventDefault(); void send({ type: 'prompt', session_id: selected, text: prompt }).catch(() => setError('Prompt could not be sent.')); }}><label htmlFor="remote-prompt">Prompt this agent</label><textarea id="remote-prompt" value={prompt} onChange={event => setPrompt(event.target.value)} maxLength={4000} rows={3} /><button type="submit" disabled={!prompt.trim() || !active?.can_prompt}>Send prompt</button>{!active?.can_prompt && <p>Remote prompting requires an idle session launched by Spawnt.</p>}</form><form onSubmit={event => { event.preventDefault(); void send({ type: 'switch', session_id: selected, model: targetModel }).catch(() => setError('Model switch could not be sent.')); }}><label htmlFor="remote-model">Switch to model</label><input id="remote-model" value={targetModel} onChange={event => setTargetModel(event.target.value)} maxLength={128} /><button type="submit" disabled={!targetModel.trim()}>Handoff and switch</button></form></>}
    </section>}
  </section>;
}
