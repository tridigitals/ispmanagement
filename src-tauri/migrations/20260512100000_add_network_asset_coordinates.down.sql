DROP INDEX IF EXISTS idx_network_assets_tenant_coordinates;

ALTER TABLE public.network_assets
  DROP CONSTRAINT IF EXISTS chk_network_assets_coordinate_pair;
ALTER TABLE public.network_assets
  DROP CONSTRAINT IF EXISTS chk_network_assets_longitude;
ALTER TABLE public.network_assets
  DROP CONSTRAINT IF EXISTS chk_network_assets_latitude;

ALTER TABLE public.network_assets
  DROP COLUMN IF EXISTS longitude,
  DROP COLUMN IF EXISTS latitude;
