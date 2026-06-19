-- Rollback Sprint C migration: remove latitude/longitude/address_line from olts table

ALTER TABLE public.olts
  DROP COLUMN IF EXISTS latitude,
  DROP COLUMN IF EXISTS longitude,
  DROP COLUMN IF EXISTS address_line;