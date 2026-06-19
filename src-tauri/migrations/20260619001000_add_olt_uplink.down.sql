-- Sprint D down: drop uplink_router_id + uplink_port from public.olts

ALTER TABLE public.olts
  DROP CONSTRAINT IF EXISTS fk_olts_uplink_router;

ALTER TABLE public.olts
  DROP COLUMN IF EXISTS uplink_port,
  DROP COLUMN IF EXISTS uplink_router_id;
