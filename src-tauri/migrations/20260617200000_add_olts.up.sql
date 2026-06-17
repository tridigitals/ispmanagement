-- OLT Inventory + ONU History (tenant-scoped)
-- Multi-vendor OLT monitoring via driver abstraction

CREATE TABLE IF NOT EXISTS public.olts (
    id text PRIMARY KEY NOT NULL,
    tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
    name text NOT NULL,
    description text,
    olt_type text NOT NULL,
    host text NOT NULL,
    port integer NOT NULL DEFAULT 80,
    username text NOT NULL,
    password_enc text,
    last_stats jsonb,
    last_updated timestamp with time zone,
    is_online boolean NOT NULL DEFAULT false,
    last_polled_at timestamp with time zone,
    last_error text,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_olts_tenant
    ON public.olts (tenant_id);

CREATE INDEX IF NOT EXISTS idx_olts_type
    ON public.olts (olt_type);

CREATE TABLE IF NOT EXISTS public.olt_onu_history (
    id text PRIMARY KEY NOT NULL,
    olt_id text NOT NULL REFERENCES public.olts(id) ON DELETE CASCADE,
    tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
    onu_id text NOT NULL,
    pon text NOT NULL,
    mac text,
    name text,
    status text NOT NULL,
    rx_power real,
    tx_power real,
    distance real,
    temperature real,
    recorded_at timestamp with time zone NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_onu_history_olt
    ON public.olt_onu_history (olt_id, recorded_at DESC);

CREATE INDEX IF NOT EXISTS idx_onu_history_tenant
    ON public.olt_onu_history (tenant_id);

CREATE INDEX IF NOT EXISTS idx_onu_history_mac
    ON public.olt_onu_history (mac);
