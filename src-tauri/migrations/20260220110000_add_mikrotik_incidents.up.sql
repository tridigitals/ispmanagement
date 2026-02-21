-- NOC incident timeline derived from MikroTik alerts.
-- Keeps historical entries while still allowing one active incident per dedup key.

CREATE TABLE IF NOT EXISTS public.mikrotik_incidents (
    id text PRIMARY KEY NOT NULL,
    tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
    router_id text NOT NULL REFERENCES public.mikrotik_routers(id) ON DELETE CASCADE,
    interface_name text,
    incident_type text NOT NULL, -- offline | cpu | latency | interface_*
    dedup_key text NOT NULL,     -- app-generated key to group recurring signals
    severity text NOT NULL DEFAULT 'warning', -- info | warning | critical
    status text NOT NULL DEFAULT 'open',      -- open | ack | in_progress | resolved
    title text NOT NULL,
    message text NOT NULL,
    value_num double precision,
    threshold_num double precision,
    first_seen_at timestamp with time zone NOT NULL,
    last_seen_at timestamp with time zone NOT NULL,
    resolved_at timestamp with time zone,
    acked_at timestamp with time zone,
    acked_by text,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_mikrotik_incidents_tenant_status
    ON public.mikrotik_incidents (tenant_id, status, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_mikrotik_incidents_router_active
    ON public.mikrotik_incidents (router_id, resolved_at, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_mikrotik_incidents_tenant_type
    ON public.mikrotik_incidents (tenant_id, incident_type, updated_at DESC);

CREATE UNIQUE INDEX IF NOT EXISTS uq_mikrotik_incidents_active_dedup
    ON public.mikrotik_incidents (tenant_id, dedup_key)
    WHERE resolved_at IS NULL;
