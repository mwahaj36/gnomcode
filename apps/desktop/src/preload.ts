import { contextBridge, ipcRenderer } from 'electron';

export interface DesktopApi {
  getVersion(): Promise<string>;
  bringToFront(): Promise<void>;
  chooseWorkspace(): Promise<string | null>;
  launchSession(agent: string, repoPath: string, model?: string): Promise<unknown>;
  stopSession(sessionId: string): Promise<unknown>;
}

const desktop: DesktopApi = Object.freeze({
  getVersion: (): Promise<string> => ipcRenderer.invoke('desktop:get-version'),
  bringToFront: (): Promise<void> => ipcRenderer.invoke('desktop:bring-to-front'),
  chooseWorkspace: (): Promise<string | null> => ipcRenderer.invoke('desktop:choose-workspace'),
  launchSession: (agent: string, repoPath: string, model?: string): Promise<unknown> =>
    ipcRenderer.invoke('desktop:launch-session', agent, repoPath, model),
  stopSession: (sessionId: string): Promise<unknown> =>
    ipcRenderer.invoke('desktop:stop-session', sessionId),
});
contextBridge.exposeInMainWorld('spawntDesktop', desktop);
