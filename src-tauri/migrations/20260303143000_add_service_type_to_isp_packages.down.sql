DROP INDEX IF EXISTS public.idx_isp_packages_service_type;

ALTER TABLE public.isp_packages
    DROP COLUMN IF EXISTS service_type;
