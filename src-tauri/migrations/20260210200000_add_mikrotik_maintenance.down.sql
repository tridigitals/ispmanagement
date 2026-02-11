DROP INDEX IF EXISTS public.idx_mikrotik_routers_maintenance_until;

ALTER TABLE public.mikrotik_routers
  DROP COLUMN IF EXISTS maintenance_until,
  DROP COLUMN IF EXISTS maintenance_reason;

