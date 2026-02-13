-- MikroTik PPP Profiles + IP Pools (tenant scoped, per router)
-- Used for importing PPPoE accounts with full context (profile + pool),
-- and for keeping DB as a source-of-truth / inventory.

CREATE TABLE IF NOT EXISTS public.mikrotik_ppp_profiles (
    id text PRIMARY KEY NOT NULL,
    tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
    router_id text NOT NULL REFERENCES public.mikrotik_routers(id) ON DELETE CASCADE,
    name text NOT NULL,
    local_address text,
    remote_address text,
    rate_limit text,
    dns_server text,
    only_one boolean,
    change_tcp_mss boolean,
    use_compression boolean,
    use_encryption boolean,
    use_ipv6 boolean,
    bridge text,
    comment text,
    router_present boolean NOT NULL DEFAULT true,
    last_sync_at timestamp with time zone,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT mikrotik_ppp_profiles_unique UNIQUE (tenant_id, router_id, name)
);

CREATE INDEX IF NOT EXISTS idx_mikrotik_ppp_profiles_router ON public.mikrotik_ppp_profiles(router_id, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_mikrotik_ppp_profiles_tenant ON public.mikrotik_ppp_profiles(tenant_id, updated_at DESC);

CREATE TABLE IF NOT EXISTS public.mikrotik_ip_pools (
    id text PRIMARY KEY NOT NULL,
    tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
    router_id text NOT NULL REFERENCES public.mikrotik_routers(id) ON DELETE CASCADE,
    name text NOT NULL,
    ranges text,
    next_pool text,
    comment text,
    router_present boolean NOT NULL DEFAULT true,
    last_sync_at timestamp with time zone,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT mikrotik_ip_pools_unique UNIQUE (tenant_id, router_id, name)
);

CREATE INDEX IF NOT EXISTS idx_mikrotik_ip_pools_router ON public.mikrotik_ip_pools(router_id, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_mikrotik_ip_pools_tenant ON public.mikrotik_ip_pools(tenant_id, updated_at DESC);

