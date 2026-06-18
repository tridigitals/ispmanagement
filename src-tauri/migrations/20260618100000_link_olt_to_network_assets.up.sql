-- Sprint B.1: Link OLT/ONU ke network_assets
-- Kolom baru: olt_id (FK ke olts) + pon_port (e.g. "0/1/1")
-- Backward-compatible: nullable + ON DELETE SET NULL

ALTER TABLE network_assets
  ADD COLUMN IF NOT EXISTS olt_id text REFERENCES public.olts(id) ON DELETE SET NULL,
  ADD COLUMN IF NOT EXISTS pon_port text;

CREATE INDEX IF NOT EXISTS idx_network_assets_olt_pon
  ON network_assets(olt_id, pon_port) WHERE olt_id IS NOT NULL;

-- Sprint B.2: Link ONU history ke customer
-- customer_id nullable — banyak ONU di OLT yang belum ter-assign ke customer

ALTER TABLE olt_onu_history
  ADD COLUMN IF NOT EXISTS customer_id text REFERENCES customers(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_olt_onu_history_customer
  ON olt_onu_history(customer_id, recorded_at DESC) WHERE customer_id IS NOT NULL;
