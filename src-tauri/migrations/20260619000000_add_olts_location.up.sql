-- Sprint C: add latitude/longitude/address_line to olts table
-- Source of truth for OLT location. Also denormalized to network_assets.latitude/longitude
-- for the network_mapping_service sync (see 20260618100000_link_olt_to_network_assets).
-- Both columns must be filled together OR both NULL (validated in NetworkAssetService).

ALTER TABLE public.olts
  ADD COLUMN IF NOT EXISTS latitude double precision,
  ADD COLUMN IF NOT EXISTS longitude double precision,
  ADD COLUMN IF NOT EXISTS address_line text;