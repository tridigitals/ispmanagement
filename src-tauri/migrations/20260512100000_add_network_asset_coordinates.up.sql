ALTER TABLE public.network_assets
  ADD COLUMN IF NOT EXISTS latitude double precision,
  ADD COLUMN IF NOT EXISTS longitude double precision;

ALTER TABLE public.network_assets
  DROP CONSTRAINT IF EXISTS chk_network_assets_latitude;
ALTER TABLE public.network_assets
  DROP CONSTRAINT IF EXISTS chk_network_assets_longitude;
ALTER TABLE public.network_assets
  DROP CONSTRAINT IF EXISTS chk_network_assets_coordinate_pair;

ALTER TABLE public.network_assets
  ADD CONSTRAINT chk_network_assets_latitude
    CHECK (latitude IS NULL OR (latitude >= -90 AND latitude <= 90)),
  ADD CONSTRAINT chk_network_assets_longitude
    CHECK (longitude IS NULL OR (longitude >= -180 AND longitude <= 180)),
  ADD CONSTRAINT chk_network_assets_coordinate_pair
    CHECK ((latitude IS NULL AND longitude IS NULL) OR (latitude IS NOT NULL AND longitude IS NOT NULL));

CREATE INDEX IF NOT EXISTS idx_network_assets_tenant_coordinates
  ON public.network_assets (tenant_id, latitude, longitude);
