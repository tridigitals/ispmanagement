DROP INDEX IF EXISTS public.idx_mikrotik_routers_tenant_coordinates;

ALTER TABLE public.mikrotik_routers
  DROP CONSTRAINT IF EXISTS chk_mikrotik_routers_latitude;
ALTER TABLE public.mikrotik_routers
  DROP CONSTRAINT IF EXISTS chk_mikrotik_routers_longitude;

ALTER TABLE public.mikrotik_routers
  DROP COLUMN IF EXISTS latitude,
  DROP COLUMN IF EXISTS longitude;
