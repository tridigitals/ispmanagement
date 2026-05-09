ALTER TABLE public.isp_package_router_mappings
    ADD COLUMN IF NOT EXISTS isolation_pool text;
