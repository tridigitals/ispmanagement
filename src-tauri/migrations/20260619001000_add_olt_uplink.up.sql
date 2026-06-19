-- Sprint D: add uplink_router_id + uplink_port to public.olts
-- Links OLT to its upstream MikroTik router for topology chain OLT→Router→Customer→ONU

ALTER TABLE public.olts
  ADD COLUMN IF NOT EXISTS uplink_router_id uuid,
  ADD COLUMN IF NOT EXISTS uplink_port text;

-- Foreign key to mikrotik_routers
DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint WHERE conname = 'fk_olts_uplink_router'
  ) THEN
    ALTER TABLE public.olts
      ADD CONSTRAINT fk_olts_uplink_router
      FOREIGN KEY (uplink_router_id)
      REFERENCES public.mikrotik_routers(id)
      ON DELETE SET NULL;
  END IF;
END $$;
