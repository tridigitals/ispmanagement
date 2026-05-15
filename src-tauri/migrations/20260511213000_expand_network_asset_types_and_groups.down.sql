DROP INDEX IF EXISTS idx_network_assets_tenant_group;

ALTER TABLE public.network_assets
  DROP CONSTRAINT IF EXISTS chk_network_assets_group;

ALTER TABLE public.network_assets
  DROP CONSTRAINT IF EXISTS chk_network_assets_type;

ALTER TABLE public.network_assets
  ADD CONSTRAINT chk_network_assets_type CHECK (
    asset_type IN ('olt', 'odc', 'odp', 'splitter', 'ont', 'onu', 'fat', 'nap')
  );

ALTER TABLE public.network_assets
  DROP COLUMN IF EXISTS asset_group;
