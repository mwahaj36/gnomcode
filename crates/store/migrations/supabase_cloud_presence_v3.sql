-- Apply in the Supabase SQL editor before publishing cloud collectors.
-- Independent rows prevent desktop snapshots from replacing cloud sessions.
BEGIN;
CREATE TABLE IF NOT EXISTS public.fleethud_cloud_sessions (
    user_id uuid NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    session_id text NOT NULL CHECK (length(session_id) BETWEEN 1 AND 240),
    session jsonb NOT NULL CHECK (
        jsonb_typeof(session) = 'object'
        AND session ?& ARRAY['id', 'surface', 'usage_available']
        AND session->>'id' = session_id
        AND session->>'surface' = 'cloud'
        AND jsonb_typeof(session->'usage_available') = 'boolean'
    ),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, session_id)
);
ALTER TABLE public.fleethud_cloud_sessions ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS cloud_sessions_owner ON public.fleethud_cloud_sessions;
CREATE POLICY cloud_sessions_owner ON public.fleethud_cloud_sessions
    FOR ALL TO authenticated
    USING ((SELECT auth.uid()) = user_id)
    WITH CHECK ((SELECT auth.uid()) = user_id);
REVOKE ALL ON public.fleethud_cloud_sessions FROM PUBLIC, anon;
GRANT SELECT, INSERT, UPDATE, DELETE ON public.fleethud_cloud_sessions TO authenticated;

CREATE OR REPLACE FUNCTION public.fleethud_cloud_heartbeat_time()
RETURNS trigger LANGUAGE plpgsql SET search_path = '' AS $$
BEGIN
    NEW.updated_at := clock_timestamp();
    RETURN NEW;
END;
$$;
DROP TRIGGER IF EXISTS cloud_heartbeat_time ON public.fleethud_cloud_sessions;
CREATE TRIGGER cloud_heartbeat_time BEFORE INSERT OR UPDATE
    ON public.fleethud_cloud_sessions FOR EACH ROW
    EXECUTE FUNCTION public.fleethud_cloud_heartbeat_time();

-- Expire presence at read time even if the collector was killed without cleanup.
-- security_invoker keeps the underlying owner policy in effect (Postgres 15+).
CREATE OR REPLACE VIEW public.fleethud_cloud_sessions_live
WITH (security_invoker = true) AS
SELECT user_id, session_id, updated_at,
    jsonb_set(jsonb_set(jsonb_set(session,
        '{state}', to_jsonb(CASE WHEN updated_at > now() - interval '90 seconds'
            THEN 'connected'::text ELSE 'disconnected'::text END)),
        '{last_event_at}', to_jsonb(updated_at)), '{is_active}', 'false'::jsonb) AS session
FROM public.fleethud_cloud_sessions;
REVOKE ALL ON public.fleethud_cloud_sessions_live FROM PUBLIC, anon;
GRANT SELECT ON public.fleethud_cloud_sessions_live TO authenticated;
COMMIT;
