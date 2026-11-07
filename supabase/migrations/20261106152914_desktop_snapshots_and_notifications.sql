begin;
-- Metadata snapshots belong to a desktop, so another machine cannot overwrite them.
alter table public.spawnt_desktops add column if not exists live_snapshot jsonb;
comment on column public.spawnt_desktops.live_snapshot is 'Opt-in usage metadata only. Never conversation content, filesystem paths or agent credentials.';

create table if not exists public.gnomcode_push_subscription (
  user_id uuid not null references auth.users(id) on delete cascade,
  endpoint text not null check (char_length(endpoint) between 1 and 2048),
  subscription jsonb not null check (jsonb_typeof(subscription)='object' and octet_length(subscription::text)<=4096),
  updated_at timestamptz not null default now(),
  primary key (user_id,endpoint)
);
create table if not exists public.gnomcode_push_delivery (
  user_id uuid not null references auth.users(id) on delete cascade,
  event_id text not null check (char_length(event_id) between 1 and 256),
  created_at timestamptz not null default now(),
  primary key (user_id,event_id)
);
alter table public.gnomcode_push_subscription enable row level security;
alter table public.gnomcode_push_delivery enable row level security;
revoke all on public.gnomcode_push_subscription,public.gnomcode_push_delivery from anon,authenticated;
grant select,insert,update,delete on public.gnomcode_push_subscription,public.gnomcode_push_delivery to authenticated;
drop policy if exists push_subscription_owner on public.gnomcode_push_subscription;
create policy push_subscription_owner on public.gnomcode_push_subscription for all to authenticated
  using ((select auth.uid())=user_id) with check ((select auth.uid())=user_id);
drop policy if exists push_delivery_owner on public.gnomcode_push_delivery;
create policy push_delivery_owner on public.gnomcode_push_delivery for all to authenticated
  using ((select auth.uid())=user_id) with check ((select auth.uid())=user_id);
comment on table public.gnomcode_push_subscription is 'Explicit opt-in notification destinations. Generic lifecycle notices only; revocable from the browser.';
notify pgrst,'reload schema';
commit;
