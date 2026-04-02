-- Managed RADIUS default server + plan gate foundation
-- 1. Add one global default flag to radius_servers
-- 2. Enforce only one default server at a time

ALTER TABLE public.radius_servers
    ADD COLUMN IF NOT EXISTS is_default boolean NOT NULL DEFAULT false;

UPDATE public.radius_servers
SET is_default = false
WHERE is_default IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_radius_servers_single_default
    ON public.radius_servers (is_default)
    WHERE is_default = true;
