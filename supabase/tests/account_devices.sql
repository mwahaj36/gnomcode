-- Run against a disposable database after applying the device-directory migration.
begin;
insert into auth.users(id) values
  ('10000000-0000-0000-0000-000000000001'),
  ('20000000-0000-0000-0000-000000000002') on conflict do nothing;
set local role authenticated;
set local request.jwt.claim.sub = '10000000-0000-0000-0000-000000000001';
insert into public.spawnt_desktops(user_id,id) values
  ('10000000-0000-0000-0000-000000000001',repeat('a',32)),
  ('10000000-0000-0000-0000-000000000001',repeat('b',32));
insert into public.spawnt_device_requests(user_id,desktop_id,id,public_key,name) values
  ('10000000-0000-0000-0000-000000000001',repeat('a',32),repeat('c',32),repeat('d',64),'Test phone');
update public.spawnt_device_requests set revoked=true where id=repeat('c',32);
do $$ begin
  if (select count(*) from public.spawnt_desktops) <> 2 then raise exception 'multiple desktops overwritten'; end if;
  if not (select revoked from public.spawnt_device_requests where id=repeat('c',32)) then raise exception 'owner revocation failed'; end if;
  begin
    update public.spawnt_desktops set user_id='20000000-0000-0000-0000-000000000002';
    raise exception 'owner reassignment was allowed';
  exception when insufficient_privilege then null; end;
end $$;
set local request.jwt.claim.sub = '20000000-0000-0000-0000-000000000002';
do $$ begin
  if exists(select 1 from public.spawnt_desktops) then raise exception 'desktop leaked across accounts'; end if;
  if exists(select 1 from public.spawnt_device_requests) then raise exception 'device key leaked across accounts'; end if;
  begin
    insert into public.spawnt_device_requests(user_id,desktop_id,id,public_key,name) values
      ('10000000-0000-0000-0000-000000000001',repeat('a',32),repeat('e',32),repeat('f',64),'Other owner');
    raise exception 'cross-account request was allowed';
  exception when insufficient_privilege then null; end;
end $$;
update public.spawnt_device_requests set revoked=false;
delete from public.spawnt_desktops;
set local request.jwt.claim.sub = '10000000-0000-0000-0000-000000000001';
do $$ begin
  if (select count(*) from public.spawnt_desktops) <> 2 then raise exception 'cross-account delete succeeded'; end if;
  if not (select revoked from public.spawnt_device_requests) then raise exception 'cross-account update succeeded'; end if;
end $$;
delete from public.spawnt_desktops where id=repeat('a',32);
do $$ begin
  if exists(select 1 from public.spawnt_device_requests) then raise exception 'deleted desktop left requests behind'; end if;
end $$;
set local role anon;
do $$ begin
  begin perform * from public.spawnt_desktops; raise exception 'anonymous desktop read allowed';
  exception when insufficient_privilege then null; end;
  begin perform * from public.spawnt_device_requests; raise exception 'anonymous request read allowed';
  exception when insufficient_privilege then null; end;
end $$;
rollback;
