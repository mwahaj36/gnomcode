-- Run inside a transaction and roll back; creates only isolated fixture identities.
select set_config('gnomcode.test_owner',gen_random_uuid()::text,true);
select set_config('gnomcode.test_other',gen_random_uuid()::text,true);
insert into auth.users(id) values(current_setting('gnomcode.test_owner')::uuid),(current_setting('gnomcode.test_other')::uuid);
insert into public.gnomcode_push_subscription(user_id,endpoint,subscription) values
(current_setting('gnomcode.test_owner')::uuid,'https://fcm.googleapis.com/test-owner','{}'),
(current_setting('gnomcode.test_other')::uuid,'https://fcm.googleapis.com/test-other','{}');
select set_config('request.jwt.claim.sub',current_setting('gnomcode.test_owner'),true);
set local role authenticated;
do $$
begin
  if (select count(*) from public.gnomcode_push_subscription) != 1 then raise exception 'Push subscriptions leaked across accounts'; end if;
  begin
    update public.gnomcode_push_subscription set user_id=current_setting('gnomcode.test_other')::uuid;
    raise exception 'Owner reassignment was allowed';
  exception when insufficient_privilege then null;
  end;
  begin
    insert into public.gnomcode_push_delivery(user_id,event_id) values(current_setting('gnomcode.test_other')::uuid,'cross-account');
    raise exception 'Cross-account event insertion was allowed';
  exception when insufficient_privilege then null;
  end;
end $$;
reset role;
