ALTER TABLE network_assets DROP CONSTRAINT IF EXISTS chk_network_assets_group;
ALTER TABLE network_assets ADD CONSTRAINT chk_network_assets_group CHECK (asset_group = ANY (ARRAY['access_fiber'::text, 'infrastructure'::text]));
