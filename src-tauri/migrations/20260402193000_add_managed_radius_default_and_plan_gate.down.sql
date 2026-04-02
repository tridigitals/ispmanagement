DROP INDEX IF EXISTS public.idx_radius_servers_single_default;

ALTER TABLE public.radius_servers
    DROP COLUMN IF EXISTS is_default;
