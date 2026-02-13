CREATE TABLE IF NOT EXISTS public.mikrotik_logs (
    id text PRIMARY KEY,
    tenant_id text NOT NULL,
    router_id text NOT NULL REFERENCES public.mikrotik_routers(id) ON DELETE CASCADE,
    router_log_id text,
    logged_at timestamptz NOT NULL,
    router_time text,
    topics text,
    level text,
    message text NOT NULL,
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_mikrotik_logs_tenant_logged_at
    ON public.mikrotik_logs (tenant_id, logged_at DESC);

CREATE INDEX IF NOT EXISTS idx_mikrotik_logs_router_logged_at
    ON public.mikrotik_logs (router_id, logged_at DESC);

CREATE UNIQUE INDEX IF NOT EXISTS uq_mikrotik_logs_router_log_id
    ON public.mikrotik_logs (router_id, router_log_id)
    WHERE router_log_id IS NOT NULL;
