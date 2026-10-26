-- Run in a disposable Postgres database after the v3 migration, with auth.uid()
-- reading request.jwt.claim.sub and test users seeded as below by the harness.
SET ROLE authenticated;
SELECT set_config('request.jwt.claim.sub', '00000000-0000-0000-0000-000000000001', false);
INSERT INTO public.fleethud_cloud_sessions(user_id, session_id, session, updated_at)
VALUES ('00000000-0000-0000-0000-000000000001', 'codex:cloud-one',
'{"id":"codex:cloud-one","surface":"cloud","usage_available":false,"is_active":false}',
'2000-01-01');
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM public.fleethud_cloud_sessions_live
        WHERE session->>'state' = 'connected' AND updated_at > now() - interval '5 seconds') THEN
        RAISE EXCEPTION 'server did not override caller timestamp';
    END IF;
    BEGIN
        INSERT INTO public.fleethud_cloud_sessions(user_id, session_id, session)
        VALUES ('00000000-0000-0000-0000-000000000002', 'codex:cloud-two',
        '{"id":"codex:cloud-two","surface":"cloud","usage_available":false}');
        RAISE EXCEPTION 'cross-account write succeeded';
    EXCEPTION WHEN insufficient_privilege THEN NULL;
    END;
    BEGIN
        INSERT INTO public.fleethud_cloud_sessions(user_id, session_id, session)
        VALUES ('00000000-0000-0000-0000-000000000001', 'codex:cloud-invalid', '{}');
        RAISE EXCEPTION 'missing metadata accepted';
    EXCEPTION WHEN check_violation THEN NULL;
    END;
END $$;
UPDATE public.fleethud_cloud_sessions SET session = session || '{"usage_available":true,"input_tokens":100,"total_cost_usd":0.1,"cost_basis":"estimated"}'::jsonb;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM public.fleethud_cloud_sessions_live
        WHERE session->'usage_available' = 'true'::jsonb AND session->>'cost_basis' = 'estimated') THEN
        RAISE EXCEPTION 'measured usage was not retained';
    END IF;
END $$;
SELECT set_config('request.jwt.claim.sub', '00000000-0000-0000-0000-000000000002', false);
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM public.fleethud_cloud_sessions_live) THEN
        RAISE EXCEPTION 'cross-account read succeeded';
    END IF;
END $$;
RESET ROLE;
-- Simulate a dead collector without waiting 90 seconds. Disable only the server
-- clock trigger in this disposable database; callers cannot disable it under RLS.
ALTER TABLE public.fleethud_cloud_sessions DISABLE TRIGGER cloud_heartbeat_time;
UPDATE public.fleethud_cloud_sessions SET updated_at = now() - interval '120 seconds';
ALTER TABLE public.fleethud_cloud_sessions ENABLE TRIGGER cloud_heartbeat_time;
SET ROLE authenticated;
SELECT set_config('request.jwt.claim.sub', '00000000-0000-0000-0000-000000000001', false);
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM public.fleethud_cloud_sessions_live
        WHERE session->>'state' = 'disconnected' AND session->>'is_active' = 'false') THEN
        RAISE EXCEPTION 'dead collector did not expire';
    END IF;
END $$;
RESET ROLE;
