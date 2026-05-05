DROP INDEX IF EXISTS public.idx_isp_packages_provisioning_type;

ALTER TABLE public.isp_packages
    DROP COLUMN IF EXISTS provisioning_type;
