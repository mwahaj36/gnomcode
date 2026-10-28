-- Spawnt Live Cloud Sync Table (Supabase Postgres + RLS)
-- Stores per-user subscription plans, custom model pricing overrides,
-- and the latest 2-second live mission-control snapshot pushed from the Native App.

CREATE TABLE IF NOT EXISTS public.spawnt_sync_state (
    user_id uuid PRIMARY KEY REFERENCES auth.users(id) ON DELETE CASCADE,
    email text,
    display_name text NOT NULL DEFAULT 'Developer',
    openai_plan_usd double precision NOT NULL DEFAULT 20.0,
    claude_plan_usd double precision NOT NULL DEFAULT 20.0,
    google_plan_usd double precision NOT NULL DEFAULT 20.0,
    other_plan_usd double precision NOT NULL DEFAULT 0.0,
    custom_pricing jsonb NOT NULL DEFAULT '[]'::jsonb,
    live_snapshot jsonb NOT NULL DEFAULT '{}'::jsonb,
    updated_at timestamptz NOT NULL DEFAULT now()
);

ALTER TABLE public.spawnt_sync_state ENABLE ROW LEVEL SECURITY;

GRANT SELECT, INSERT, UPDATE ON public.spawnt_sync_state TO authenticated;

DROP POLICY IF EXISTS "Users can read own sync state" ON public.spawnt_sync_state;
CREATE POLICY "Users can read own sync state"
    ON public.spawnt_sync_state
    FOR SELECT
    TO authenticated
    USING ((SELECT auth.uid()) = user_id);

DROP POLICY IF EXISTS "Users can insert own sync state" ON public.spawnt_sync_state;
CREATE POLICY "Users can insert own sync state"
    ON public.spawnt_sync_state
    FOR INSERT
    TO authenticated
    WITH CHECK ((SELECT auth.uid()) = user_id);

DROP POLICY IF EXISTS "Users can update own sync state" ON public.spawnt_sync_state;
CREATE POLICY "Users can update own sync state"
    ON public.spawnt_sync_state
    FOR UPDATE
    TO authenticated
    USING ((SELECT auth.uid()) = user_id)
    WITH CHECK ((SELECT auth.uid()) = user_id);
