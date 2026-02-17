ALTER TABLE public.pppoe_accounts
    DROP COLUMN IF EXISTS package_id;

DROP TABLE IF EXISTS public.isp_package_router_mappings CASCADE;
DROP TABLE IF EXISTS public.isp_packages CASCADE;

