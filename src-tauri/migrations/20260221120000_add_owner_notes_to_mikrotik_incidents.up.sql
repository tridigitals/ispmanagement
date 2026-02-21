ALTER TABLE public.mikrotik_incidents
    ADD COLUMN IF NOT EXISTS owner_user_id text,
    ADD COLUMN IF NOT EXISTS notes text;

CREATE INDEX IF NOT EXISTS idx_mikrotik_incidents_tenant_owner
    ON public.mikrotik_incidents (tenant_id, owner_user_id, updated_at DESC);
