-- MikroTik Interface Metrics (per-router, per-interface counters + rates)

CREATE TABLE IF NOT EXISTS public.mikrotik_interface_metrics (
    id text PRIMARY KEY NOT NULL,
    router_id text NOT NULL REFERENCES public.mikrotik_routers(id) ON DELETE CASCADE,
    interface_name text NOT NULL,
    ts timestamp with time zone NOT NULL,
    rx_byte bigint,
    tx_byte bigint,
    rx_bps bigint,
    tx_bps bigint,
    running boolean,
    disabled boolean,
    link_downs bigint
);

CREATE INDEX IF NOT EXISTS idx_mikrotik_interface_metrics_router_iface_ts
    ON public.mikrotik_interface_metrics (router_id, interface_name, ts DESC);

