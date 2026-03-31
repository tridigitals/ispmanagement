CREATE INDEX IF NOT EXISTS idx_mikrotik_logs_tenant_router_logged_updated
    ON public.mikrotik_logs (tenant_id, router_id, logged_at DESC, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_mikrotik_logs_tenant_router_logged_at
    ON public.mikrotik_logs (tenant_id, router_id, logged_at);
