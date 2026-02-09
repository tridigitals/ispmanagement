-- MikroTik Router Inventory + Monitoring (tenant-scoped)

CREATE TABLE IF NOT EXISTS public.mikrotik_routers (
    id text PRIMARY KEY NOT NULL,
    tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
    name text NOT NULL,
    host text NOT NULL,
    port integer NOT NULL DEFAULT 8728,
    username text NOT NULL,
    password text NOT NULL,
    use_tls boolean NOT NULL DEFAULT false,
    enabled boolean NOT NULL DEFAULT true,
    identity text,
    ros_version text,
    is_online boolean NOT NULL DEFAULT false,
    last_seen_at timestamp with time zone,
    latency_ms integer,
    last_error text,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_mikrotik_routers_tenant_enabled
    ON public.mikrotik_routers (tenant_id, enabled, updated_at DESC);

CREATE TABLE IF NOT EXISTS public.mikrotik_router_metrics (
    id text PRIMARY KEY NOT NULL,
    router_id text NOT NULL REFERENCES public.mikrotik_routers(id) ON DELETE CASCADE,
    ts timestamp with time zone NOT NULL,
    cpu_load integer,
    total_memory_bytes bigint,
    free_memory_bytes bigint,
    total_hdd_bytes bigint,
    free_hdd_bytes bigint,
    uptime_seconds bigint,
    rx_bps bigint,
    tx_bps bigint
);

CREATE INDEX IF NOT EXISTS idx_mikrotik_router_metrics_router_ts
    ON public.mikrotik_router_metrics (router_id, ts DESC);

