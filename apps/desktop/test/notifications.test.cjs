const test=require('node:test'),assert=require('node:assert/strict'),{EventEmitter}=require('node:events');
test('Windows toast timeout keeps a strong reference for Notification Center',()=>{
 const {retainNotification}=require('../dist/notifications');const retained=new Set(),toast=new EventEmitter();let clicks=0;
 retainNotification(toast,retained,()=>clicks++,'win32');
 assert.ok(retained.has(toast));toast.emit('close',{reason:'timedOut'});assert.ok(retained.has(toast));
 toast.emit('click');assert.equal(clicks,1);assert.equal(retained.size,0);
});
test('failed and user-dismissed notifications release their references',()=>{
 const {retainNotification}=require('../dist/notifications');
 for(const event of ['failed','close']){const retained=new Set(),toast=new EventEmitter();retainNotification(toast,retained,()=>{},'win32');toast.emit(event,{reason:'userCanceled'});assert.equal(retained.size,0);}
});

test('notification registration repairs only a shortcut targeting this app',()=>{
 const {repairNotificationShortcut}=require('../dist/notifications');let written;
 const links={readShortcutLink:()=>({target:'C:\\App\\Gnomcode.exe'}),writeShortcutLink:(...args)=>{written=args;return true}};
 assert.equal(repairNotificationShortcut('app.lnk','C:\\App\\Gnomcode.exe',links),true);
 assert.deepEqual(written,['app.lnk','update',{target:'C:\\App\\Gnomcode.exe',appUserModelId:'app.gnomcode.desktop'}]);
 written=null;links.readShortcutLink=()=>({target:'C:\\Other\\App.exe'});
 assert.equal(repairNotificationShortcut('app.lnk','C:\\App\\Gnomcode.exe',links),false);assert.equal(written,null);
});
