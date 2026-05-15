ALTER TABLE public.network_assets
  ADD COLUMN IF NOT EXISTS asset_group text;

ALTER TABLE public.network_assets
  ALTER COLUMN asset_group SET DEFAULT 'access_fiber';

UPDATE public.network_assets
SET asset_group = CASE
  WHEN asset_type IN ('switch', 'router', 'media_converter', 'odf', 'ups')
    THEN 'infrastructure'
  ELSE 'access_fiber'
END
WHERE asset_group IS NULL OR btrim(asset_group) = '';

ALTER TABLE public.network_assets
  ALTER COLUMN asset_group SET NOT NULL;

ALTER TABLE public.network_assets
  DROP CONSTRAINT IF EXISTS chk_network_assets_type;

ALTER TABLE public.network_assets
  ADD CONSTRAINT chk_network_assets_type CHECK (
    asset_type IN (
      'olt', 'odc', 'odp', 'splitter', 'ont', 'onu', 'fat', 'nap',
      'switch', 'router', 'media_converter', 'odf', 'ups'
    )
  );

ALTER TABLE public.network_assets
  DROP CONSTRAINT IF EXISTS chk_network_assets_group;

ALTER TABLE public.network_assets
  ADD CONSTRAINT chk_network_assets_group CHECK (
    asset_group IN ('access_fiber', 'infrastructure')
  );

CREATE INDEX IF NOT EXISTS idx_network_assets_tenant_group
  ON public.network_assets (tenant_id, asset_group);
