import type {Notification} from 'electron';

export function retainNotification(notification:Notification,retained:Set<Notification>,onClick:()=>void,platform:NodeJS.Platform):void {
  // A collected toast is removed from Action Center, even after its banner times out.
  if(retained.size>=128){const oldest=retained.values().next().value;if(oldest)retained.delete(oldest);}
  retained.add(notification);
  const release=()=>{retained.delete(notification);};
  notification.once('click',()=>{onClick();release();});
  notification.once('failed',release);
  notification.on('close',event=>{
    if(platform!=='win32' || event?.reason==='userCanceled' || event?.reason==='applicationHidden')release();
  });
}

export function repairNotificationShortcut(shortcut:string,executable:string,links:Pick<typeof import('electron').shell,'readShortcutLink'|'writeShortcutLink'>):boolean {
  try {
    const details=links.readShortcutLink(shortcut);
    if(details.target.toLowerCase()!==executable.toLowerCase())return false;
    if(details.appUserModelId==='app.gnomcode.desktop')return true;
    return links.writeShortcutLink(shortcut,'update',{target:details.target,appUserModelId:'app.gnomcode.desktop'});
  }catch{return false;}
}
