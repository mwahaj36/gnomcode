import { contextBridge, ipcRenderer } from 'electron';

export interface DesktopApi {
  getVersion(): Promise<string>;
  bringToFront(): Promise<void>;
}

const desktop: DesktopApi = Object.freeze({
  getVersion: (): Promise<string> => ipcRenderer.invoke('desktop:get-version'),
  bringToFront: (): Promise<void> => ipcRenderer.invoke('desktop:bring-to-front'),
});
contextBridge.exposeInMainWorld('spawntDesktop', desktop);
