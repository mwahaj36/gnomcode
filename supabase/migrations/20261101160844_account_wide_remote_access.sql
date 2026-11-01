-- Account-wide Remote access. Safe to apply directly in Supabase SQL Editor.
-- Standalone: supports fresh databases and the earlier device-directory migration.
-- Existing linked desktops stay disabled until the native user confirms the new scope.
begin;
create table if not exists public.spawnt_desktops (
    user_id uuid not null references auth.users(id) on delete cascade,
    id text not null check (id ~ '^[a-f0-9]{32}$'),
    name text not null default 'My desktop' check (char_length(name) between 1 and 80),
    last_seen_at timestamptz not null default now(),
    primary key (user_id, id)
);
create table if not exists public.spawnt_device_requests (
    user_id uuid not null,
    desktop_id text not null,
    id text not null check (id ~ '^[a-f0-9]{32}$'),
    public_key text not null check (public_key ~ '^[a-f0-9]{64}$'),
    name text not null check (char_length(name) between 1 and 80),
    revoked boolean not null default false,
    state text not null default 'pending' check (state in ('pending', 'approved')),
    offer jsonb,
    created_at timestamptz not null default now(),
    primary key (user_id, desktop_id, id),
    foreign key (user_id, desktop_id) references public.spawnt_desktops(user_id, id) on delete cascade,
    check (offer is null or (jsonb_typeof(offer) = 'object' and octet_length(offer::text) <= 2048))
);
alter table public.spawnt_desktops add column if not exists remote_enabled boolean not null default false;
alter table public.spawnt_desktops enable row level security;
alter table public.spawnt_device_requests enable row level security;
revoke all on public.spawnt_desktops, public.spawnt_device_requests from anon, authenticated;
grant select, insert, update, delete on public.spawnt_desktops, public.spawnt_device_requests to authenticated;
drop policy if exists desktop_owner on public.spawnt_desktops;
create policy desktop_owner on public.spawnt_desktops for all to authenticated
    using ((select auth.uid()) = user_id) with check ((select auth.uid()) = user_id);
drop policy if exists request_owner on public.spawnt_device_requests;
create policy request_owner on public.spawnt_device_requests for all to authenticated
    using ((select auth.uid()) = user_id) with check ((select auth.uid()) = user_id);

comment on column public.spawnt_desktops.remote_enabled is
  'Account-wide Remote availability. Native consent is also required locally; heartbeats never enable access.';
notify pgrst, 'reload schema';
commit;
