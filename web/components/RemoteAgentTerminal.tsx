'use client';

import { useEffect, useRef } from 'react';
import type { Terminal } from '@xterm/xterm';
import '@xterm/xterm/css/xterm.css';

export type TerminalOutput = { data: number[]; cursor: number; reset: boolean };

export default function RemoteAgentTerminal({ output, onInput, onResize }: {
  output: TerminalOutput | null;
  onInput: (text: string) => void;
  onResize: (cols: number, rows: number) => void;
}) {
  const host = useRef<HTMLDivElement>(null);
  const terminal = useRef<Terminal | null>(null);
  const callbacks = useRef({ onInput, onResize });
  callbacks.current = { onInput, onResize };
  const backlog = useRef<TerminalOutput[]>([]);
  const delivered = useRef<number | null>(null);
  const consume = (part: TerminalOutput) => {
    const instance = terminal.current;
    if (!instance) { backlog.current.push(part); return; }
    if (delivered.current === part.cursor && !part.reset) return;
    if (part.reset) instance.reset();
    instance.write(new Uint8Array(part.data)); delivered.current = part.cursor;
  };
  useEffect(() => { if (output) consume(output); }, [output]);
  useEffect(() => {
    let disposed = false;
    let cleanup = () => {};
    void (async () => {
      const [{ Terminal }, { FitAddon }] = await Promise.all([import('@xterm/xterm'), import('@xterm/addon-fit')]);
      if (disposed || !host.current) return;
      const colors = getComputedStyle(host.current);
      const instance = new Terminal({ cursorBlink: true, scrollback: 5000, fontSize: 13, fontFamily: 'Consolas, monospace', allowProposedApi: false, linkHandler:{activate:(_event,text)=>{try{const url=new URL(text);if(url.protocol==='https:'||url.protocol==='http:')window.open(url.href,'_blank','noopener,noreferrer');}catch{}}},
        theme: { background: colors.getPropertyValue('--sp-bg').trim(), foreground: colors.getPropertyValue('--sp-text').trim() } });
      const fit = new FitAddon(); instance.loadAddon(fit); instance.open(host.current); terminal.current = instance;
      const input = instance.onData(text => callbacks.current.onInput(text));
      const resized = instance.onResize(({cols,rows}) => callbacks.current.onResize(cols,rows));
      const links = instance.registerLinkProvider({ provideLinks(y, callback) {
        let start=y-1,end=y-1;
        while(start>0 && instance.buffer.active.getLine(start)?.isWrapped)start--;
        while(instance.buffer.active.getLine(end+1)?.isWrapped)end++;
        const line=Array.from({length:end-start+1},(_,i)=>instance.buffer.active.getLine(start+i)?.translateToString(start+i===end)||'').join('');
        callback([...line.matchAll(/https?:\/\/[^\s<>"']+/g)].map(match => ({ text: match[0], range: {start:{x:match.index!%instance.cols+1,y:start+Math.floor(match.index!/instance.cols)+1},end:{x:(match.index!+match[0].length-1)%instance.cols+1,y:start+Math.floor((match.index!+match[0].length-1)/instance.cols)+1}}, activate: () => { window.open(match[0], '_blank', 'noopener,noreferrer'); } })));
      } });
      const observer = new ResizeObserver(() => fit.fit()); observer.observe(host.current); fit.fit();
      for (const part of backlog.current.splice(0)) consume(part);
      instance.focus();
      cleanup = () => { observer.disconnect(); input.dispose(); resized.dispose(); links.dispose(); terminal.current = null; instance.dispose(); };
    })();
    return () => { disposed = true; cleanup(); };
  }, []);
  return <><div style={{display:'flex',gap:8,justifyContent:'flex-end',marginBottom:8}}><button type="button" onClick={()=>terminal.current?.scrollToTop()}>Earlier output</button><button type="button" onClick={()=>terminal.current?.scrollToBottom()}>Latest output</button></div><div ref={host} role="region" aria-label="Live agent terminal with scrollback" style={{flex:'1 1 0',height:'auto',minHeight:120,width:'100%',minWidth:0,background:'var(--sp-bg)',padding:8,boxSizing:'border-box'}} /></>;
}
