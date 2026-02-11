-- MikroTik Alert Incidents (tenant-scoped)
--
-- Stores active/resolved incidents derived from router polling.
-- We keep exactly one ACTIVE incident per (tenant_id, router_id, alert_type)
-- using a partial unique index (PostgreSQL).

CREATE TABLE IF NOT EXISTS public.mikrotik_alerts (
    id text PRIMARY KEY NOT NULL,
    tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
    router_id text NOT NULL REFERENCES public.mikrotik_routers(id) ON DELETE CASCADE,
    alert_type text NOT NULL, -- offline | cpu | latency
    severity text NOT NULL DEFAULT 'warning', -- info | warning | critical
    status text NOT NULL DEFAULT 'open', -- open | ack | resolved
    title text NOT NULL,
    message text NOT NULL,
    value_num double precision,
    threshold_num double precision,
    triggered_at timestamp with time zone NOT NULL,
    last_seen_at timestamp with time zone NOT NULL,
    resolved_at timestamp with time zone,
    acked_at timestamp with time zone,
    acked_by text, -- user_id
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_mikrotik_alerts_tenant_status
    ON public.mikrotik_alerts (tenant_id, status, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_mikrotik_alerts_router_active
    ON public.mikrotik_alerts (router_id, resolved_at);

-- Enforce at most one active incident per router+type.
CREATE UNIQUE INDEX IF NOT EXISTS uq_mikrotik_alerts_active_per_type
    ON public.mikrotik_alerts (tenant_id, router_id, alert_type)
    WHERE resolved_at IS NULL;

