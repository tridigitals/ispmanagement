CREATE TABLE IF NOT EXISTS public.radius_accounting_sessions (
    id text PRIMARY KEY NOT NULL,
    tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
    router_id text NOT NULL REFERENCES public.mikrotik_routers(id) ON DELETE CASCADE,
    nas_ip_address text,
    nas_ip_or_cidr text,
    username text NOT NULL,
    radius_identity text,
    acct_session_id text NOT NULL,
    status_type text NOT NULL,
    framed_ip_address text,
    calling_station_id text,
    session_time_seconds bigint,
    input_octets bigint,
    output_octets bigint,
    terminate_cause text,
    started_at timestamp with time zone,
    last_update_at timestamp with time zone,
    ended_at timestamp with time zone,
    raw_attributes_json text,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT radius_accounting_sessions_status_type_check
        CHECK (status_type IN ('start', 'stop', 'interim_update', 'accounting_on', 'accounting_off')),
    CONSTRAINT uq_radius_accounting_sessions_tenant_router_session
        UNIQUE (tenant_id, router_id, acct_session_id)
);

CREATE INDEX IF NOT EXISTS idx_radius_accounting_sessions_tenant_router_updated
    ON public.radius_accounting_sessions (tenant_id, router_id, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_radius_accounting_sessions_tenant_username
    ON public.radius_accounting_sessions (tenant_id, username, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_radius_accounting_sessions_active_lookup
    ON public.radius_accounting_sessions (tenant_id, router_id, ended_at, updated_at DESC);

CREATE TABLE IF NOT EXISTS public.radius_auth_log (
    id text PRIMARY KEY NOT NULL,
    tenant_id text REFERENCES public.tenants(id) ON DELETE SET NULL,
    router_id text REFERENCES public.mikrotik_routers(id) ON DELETE SET NULL,
    source_ip text NOT NULL,
    username text,
    radius_identity text,
    outcome text NOT NULL,
    reason text,
    auth_type text,
    latency_ms bigint,
    created_at timestamp with time zone NOT NULL,
    CONSTRAINT radius_auth_log_outcome_check
        CHECK (outcome IN ('accept', 'reject', 'error', 'invalid_nas'))
);

CREATE INDEX IF NOT EXISTS idx_radius_auth_log_tenant_router_created
    ON public.radius_auth_log (tenant_id, router_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_radius_auth_log_source_created
    ON public.radius_auth_log (source_ip, created_at DESC);
