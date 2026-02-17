-- ISP Packages (tenant-scoped) + per-router mapping to MikroTik profile / pool.

CREATE TABLE IF NOT EXISTS public.isp_packages (
    id text PRIMARY KEY NOT NULL,
    tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
    name text NOT NULL,
    description text,
    is_active boolean NOT NULL DEFAULT true,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT isp_packages_tenant_name_unique UNIQUE (tenant_id, name)
);

CREATE INDEX IF NOT EXISTS idx_isp_packages_tenant ON public.isp_packages(tenant_id);

CREATE TABLE IF NOT EXISTS public.isp_package_router_mappings (
    id text PRIMARY KEY NOT NULL,
    tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
    router_id text NOT NULL REFERENCES public.mikrotik_routers(id) ON DELETE CASCADE,
    package_id text NOT NULL REFERENCES public.isp_packages(id) ON DELETE CASCADE,
    -- RouterOS PPP profile name to apply when this package is selected for a PPPoE account
    router_profile_name text NOT NULL,
    -- Optional: default pool name to use for remote-address assignment
    address_pool text,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT isp_pkg_router_unique UNIQUE (tenant_id, router_id, package_id)
);

CREATE INDEX IF NOT EXISTS idx_isp_pkg_map_tenant ON public.isp_package_router_mappings(tenant_id);
CREATE INDEX IF NOT EXISTS idx_isp_pkg_map_router ON public.isp_package_router_mappings(router_id, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_isp_pkg_map_package ON public.isp_package_router_mappings(package_id, updated_at DESC);

ALTER TABLE public.pppoe_accounts
    ADD COLUMN IF NOT EXISTS package_id text REFERENCES public.isp_packages(id) ON DELETE SET NULL;

