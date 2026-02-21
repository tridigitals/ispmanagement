DROP INDEX IF EXISTS public.idx_mikrotik_incidents_tenant_owner;

ALTER TABLE public.mikrotik_incidents
    DROP COLUMN IF EXISTS notes,
    DROP COLUMN IF EXISTS owner_user_id;
