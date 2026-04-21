CREATE TABLE IF NOT EXISTS public.mikrotik_ppp_active_sessions (
    id text PRIMARY KEY NOT NULL,
    tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
    router_id text NOT NULL REFERENCES public.mikrotik_routers(id) ON DELETE CASCADE,
    username text NOT NULL,
    address text,
    caller_id text,
    uptime text,
    last_seen_at timestamp with time zone NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT uq_mikrotik_ppp_active_sessions_router_username UNIQUE (tenant_id, router_id, username)
);

CREATE INDEX IF NOT EXISTS idx_mikrotik_ppp_active_sessions_router_seen
    ON public.mikrotik_ppp_active_sessions (tenant_id, router_id, last_seen_at DESC);

CREATE INDEX IF NOT EXISTS idx_mikrotik_ppp_active_sessions_username
    ON public.mikrotik_ppp_active_sessions (tenant_id, username, last_seen_at DESC);
