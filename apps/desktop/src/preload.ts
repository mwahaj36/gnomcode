import { contextBridge, ipcRenderer } from 'electron';

export interface DesktopApi {
  setAppearance(theme: 'light' | 'dark'): Promise<void>;
  getVersion(): Promise<string>;
  bringToFront(): Promise<void>;
  chooseWorkspace(): Promise<string | null>;
  launchSession(agent: string, repoPath: string, model?: string): Promise<unknown>;
  stopSession(sessionId: string): Promise<unknown>;
}

const desktop: DesktopApi = Object.freeze({
  setAppearance: (theme: 'light' | 'dark'): Promise<void> => ipcRenderer.invoke('desktop:set-appearance', theme),
  getVersion: (): Promise<string> => ipcRenderer.invoke('desktop:get-version'),
  bringToFront: (): Promise<void> => ipcRenderer.invoke('desktop:bring-to-front'),
  chooseWorkspace: (): Promise<string | null> => ipcRenderer.invoke('desktop:choose-workspace'),
  launchSession: (agent: string, repoPath: string, model?: string): Promise<unknown> =>
    ipcRenderer.invoke('desktop:launch-session', agent, repoPath, model),
  stopSession: (sessionId: string): Promise<unknown> =>
    ipcRenderer.invoke('desktop:stop-session', sessionId),
});
contextBridge.exposeInMainWorld('spawntDesktop', desktop);

window.addEventListener('DOMContentLoaded', () => {
  if (!document.body.classList.contains('sp-native-console')) return;
  document.body.classList.add('sp-desktop-window');
  const header=document.createElement('div');header.className='desktop-titlebar';header.textContent='Spawnt';
  document.body.prepend(header);
  const style=document.createElement('style');style.textContent=`
    .desktop-titlebar{position:fixed;inset:0 0 auto;height:32px;z-index:100;background:var(--sp-surface,#0d0d0f);color:var(--sp-text-2,#aaa);font:500 12px/32px var(--sp-font-sans,sans-serif);padding:0 150px 0 16px;user-select:none;-webkit-app-region:drag;border-bottom:1px solid var(--sp-border,#222)}
    body.sp-desktop-window{padding-top:32px}
    body.sp-desktop-window .sp-console-workspace{min-height:calc(100dvh - 32px)}
    body.sp-desktop-window .sp-console-workspace>.sp-console-nav{top:32px;height:calc(100dvh - 32px)}
    body.sp-desktop-window[data-console-view=sessions] .sp-console-content{height:calc(100dvh - 32px)}
    body.sp-desktop-window .ide-layout{height:calc(100dvh - 97px)}
    @media(max-width:600px){body.sp-desktop-window .ide-layout{height:calc(100dvh - 32px)}body.sp-desktop-window #chat-list-toggle{top:44px!important}}
    @media(max-width:760px){body.sp-desktop-window .ide-sidebar{top:104px}}
  `;
  if (process.platform === 'darwin') style.textContent += '.desktop-titlebar{padding-left:80px}';
  document.head.append(style);
  const media=matchMedia('(prefers-color-scheme: dark)');
  const sync=(): void => {const theme=document.documentElement.dataset.theme;void desktop.setAppearance(theme === 'dark' || (theme !== 'light' && media.matches) ? 'dark' : 'light').catch(()=>{});};
  new MutationObserver(sync).observe(document.documentElement,{attributes:true,attributeFilter:['data-theme']});
  media.addEventListener('change',sync);sync();
});
