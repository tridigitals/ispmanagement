-- MikroTik Router Maintenance/Snooze (tenant-scoped)

ALTER TABLE public.mikrotik_routers
  ADD COLUMN IF NOT EXISTS maintenance_until timestamp with time zone,
  ADD COLUMN IF NOT EXISTS maintenance_reason text;

CREATE INDEX IF NOT EXISTS idx_mikrotik_routers_maintenance_until
  ON public.mikrotik_routers (maintenance_until);

