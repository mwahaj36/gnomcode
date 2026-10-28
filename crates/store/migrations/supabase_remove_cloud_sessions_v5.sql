-- Optional cleanup after upgrading all clients to v1.4.1 or newer.
-- Removes ONLY the retired hosted-session feature and its temporary pairing data.
-- Older desktop account sync (fleethud_sync_state) and Supabase Auth are untouched.
BEGIN;
DROP FUNCTION IF EXISTS public.fleethud_pair_approve(text,text,text,text);
DROP FUNCTION IF EXISTS public.fleethud_pair_consume(text,text);
DROP TABLE IF EXISTS public.fleethud_pairing_envelopes;
DROP VIEW IF EXISTS public.fleethud_cloud_sessions_live;
DROP TABLE IF EXISTS public.fleethud_cloud_sessions;
DROP FUNCTION IF EXISTS public.fleethud_cloud_heartbeat_time();
COMMIT;
