ALTER TABLE public.mikrotik_routers
  ADD COLUMN IF NOT EXISTS latitude double precision,
  ADD COLUMN IF NOT EXISTS longitude double precision;

ALTER TABLE public.mikrotik_routers
  DROP CONSTRAINT IF EXISTS chk_mikrotik_routers_latitude;
ALTER TABLE public.mikrotik_routers
  DROP CONSTRAINT IF EXISTS chk_mikrotik_routers_longitude;

ALTER TABLE public.mikrotik_routers
  ADD CONSTRAINT chk_mikrotik_routers_latitude
    CHECK (latitude IS NULL OR (latitude >= -90 AND latitude <= 90)),
  ADD CONSTRAINT chk_mikrotik_routers_longitude
    CHECK (longitude IS NULL OR (longitude >= -180 AND longitude <= 180));

CREATE INDEX IF NOT EXISTS idx_mikrotik_routers_tenant_coordinates
  ON public.mikrotik_routers (tenant_id, latitude, longitude);
