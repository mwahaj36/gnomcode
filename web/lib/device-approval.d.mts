export function approvedRequest<T extends {desktop_id:string; id:string; revoked:boolean; state:string; offer:unknown}>(requests:T[], waiting:{desktop:string; request:string}):T | undefined;
