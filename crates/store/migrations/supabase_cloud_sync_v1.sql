-- Gnomcode Supabase Schema & Row-Level Security (RLS) Migrations (SPEC §3.15)
-- Multi-tenant sync for Pro personal multi-device mirroring and Team organization rollups.

-- 1. Organizations table
CREATE TABLE IF NOT EXISTS public.organizations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    kind TEXT NOT NULL CHECK (kind IN ('personal', 'team')),
    plan TEXT NOT NULL CHECK (plan IN ('pro', 'team')),
    share_project_names BOOLEAN NOT NULL DEFAULT FALSE,
    max_seats INTEGER NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
ALTER TABLE public.organizations ENABLE ROW LEVEL SECURITY;

-- 2. Organization Members table
CREATE TABLE IF NOT EXISTS public.org_members (
    org_id UUID NOT NULL REFERENCES public.organizations(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('owner', 'admin', 'member', 'billing')),
    status TEXT NOT NULL CHECK (status IN ('invited', 'active', 'suspended')),
    joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (org_id, user_id)
);
CREATE INDEX IF NOT EXISTS idx_org_members_user ON public.org_members(user_id);
ALTER TABLE public.org_members ENABLE ROW LEVEL SECURITY;

-- 3. Paired Devices table
CREATE TABLE IF NOT EXISTS public.devices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES public.organizations(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    device_name TEXT NOT NULL,
    os TEXT NOT NULL CHECK (os IN ('windows', 'macos', 'linux')),
    token_hash TEXT NOT NULL UNIQUE,
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_devices_org_user ON public.devices(org_id, user_id);
ALTER TABLE public.devices ENABLE ROW LEVEL SECURITY;

-- 4. Hourly Usage Sync table (Idempotent rollup)
CREATE TABLE IF NOT EXISTS public.usage_hourly_sync (
    org_id UUID NOT NULL REFERENCES public.organizations(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    device_id UUID NOT NULL REFERENCES public.devices(id) ON DELETE CASCADE,
    hour_bucket TIMESTAMPTZ NOT NULL,
    project_key TEXT NOT NULL,
    project_label TEXT,
    agent TEXT NOT NULL,
    model TEXT NOT NULL,
    input_tokens BIGINT NOT NULL DEFAULT 0,
    output_tokens BIGINT NOT NULL DEFAULT 0,
    cache_read_tokens BIGINT NOT NULL DEFAULT 0,
    cache_write_tokens BIGINT NOT NULL DEFAULT 0,
    cost_usd NUMERIC(12, 4) NOT NULL DEFAULT 0.0000,
    cost_basis TEXT NOT NULL CHECK (cost_basis IN ('exact', 'estimated')),
    turn_count INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (org_id, user_id, device_id, hour_bucket, project_key, agent, model)
);
CREATE INDEX IF NOT EXISTS idx_usage_sync_lookup ON public.usage_hourly_sync(org_id, hour_bucket);
ALTER TABLE public.usage_hourly_sync ENABLE ROW LEVEL SECURITY;

-- 5. Cross-Device Quota Pool Sync table (Keyed by user, provider, limit_id)
CREATE TABLE IF NOT EXISTS public.quota_pool_sync (
    org_id UUID NOT NULL REFERENCES public.organizations(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    provider TEXT NOT NULL,
    limit_id TEXT NOT NULL,
    window_kind TEXT NOT NULL CHECK (window_kind IN ('5h', 'weekly', 'monthly')),
    used_pct REAL NOT NULL,
    resets_at TIMESTAMPTZ,
    observed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, provider, limit_id, window_kind)
);
CREATE INDEX IF NOT EXISTS idx_quota_pool_sync_user ON public.quota_pool_sync(user_id, provider);
ALTER TABLE public.quota_pool_sync ENABLE ROW LEVEL SECURITY;

-- 6. Team Budget & Webhook Alert Rules
CREATE TABLE IF NOT EXISTS public.team_budget_rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES public.organizations(id) ON DELETE CASCADE,
    scope TEXT NOT NULL CHECK (scope IN ('org', 'member', 'project')),
    limit_usd NUMERIC(10, 2) NOT NULL,
    window_kind TEXT NOT NULL CHECK (window_kind IN ('daily', 'weekly', 'monthly')),
    webhook_url TEXT NOT NULL,
    hard_stop_via_hooks BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_budget_rules_org ON public.team_budget_rules(org_id);
ALTER TABLE public.team_budget_rules ENABLE ROW LEVEL SECURITY;

-- ============================================================
-- Row-Level Security Policies (Tenant isolation)
-- ============================================================

-- helper function: check if user is active member of org
CREATE OR REPLACE FUNCTION public.is_org_member(target_org_id UUID)
RETURNS BOOLEAN
LANGUAGE sql
SECURITY DEFINER
SET search_path = public
STABLE
AS $$
    SELECT EXISTS (
        SELECT 1 FROM public.org_members
        WHERE org_id = target_org_id
          AND user_id = auth.uid()
          AND status = 'active'
    );
$$;

-- helper function: check if user is admin or owner of org
CREATE OR REPLACE FUNCTION public.is_org_admin(target_org_id UUID)
RETURNS BOOLEAN
LANGUAGE sql
SECURITY DEFINER
SET search_path = public
STABLE
AS $$
    SELECT EXISTS (
        SELECT 1 FROM public.org_members
        WHERE org_id = target_org_id
          AND user_id = auth.uid()
          AND status = 'active'
          AND role IN ('owner', 'admin')
    );
$$;

-- Policy: organizations
CREATE POLICY org_members_read ON public.organizations
    FOR SELECT TO authenticated
    USING (public.is_org_member(id));

CREATE POLICY org_admin_update ON public.organizations
    FOR UPDATE TO authenticated
    USING (public.is_org_admin(id))
    WITH CHECK (public.is_org_admin(id));

-- Policy: devices
CREATE POLICY devices_user_manage ON public.devices
    FOR ALL TO authenticated
    USING (user_id = auth.uid())
    WITH CHECK (user_id = auth.uid() AND public.is_org_member(org_id));

-- Policy: quota_pool_sync
CREATE POLICY quota_pool_user_sync ON public.quota_pool_sync
    FOR ALL TO authenticated
    USING (user_id = auth.uid())
    WITH CHECK (user_id = auth.uid() AND public.is_org_member(org_id));

-- Policy: usage_hourly_sync
CREATE POLICY usage_sync_push ON public.usage_hourly_sync
    FOR INSERT TO authenticated
    WITH CHECK (user_id = auth.uid() AND public.is_org_member(org_id));

CREATE POLICY usage_sync_read ON public.usage_hourly_sync
    FOR SELECT TO authenticated
    USING (
        (user_id = auth.uid() AND public.is_org_member(org_id))
        OR public.is_org_admin(org_id)
    );
