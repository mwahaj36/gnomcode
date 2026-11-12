export function relativeTime(iso:string, now?:number):string|null;
export function isOnline(lastSeen:string, now?:number):boolean;
export function deviceKind(name:string):'phone'|'browser';
export function groupDevices<T extends {id:string; revoked:boolean; state:string}>(requests:T[], mineIds?:string[]):{key:string; label:string; rows:T[]}[];
