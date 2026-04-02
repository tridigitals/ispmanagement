ALTER TABLE public.pppoe_accounts
    ADD COLUMN IF NOT EXISTS account_source text NOT NULL DEFAULT 'router',
    ADD COLUMN IF NOT EXISTS radius_present boolean NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS radius_identity text,
    ADD COLUMN IF NOT EXISTS radius_last_sync_at timestamp with time zone,
    ADD COLUMN IF NOT EXISTS radius_last_error text;

UPDATE public.pppoe_accounts
SET account_source = 'router'
WHERE account_source IS NULL OR btrim(account_source) = '';

ALTER TABLE public.pppoe_accounts
    DROP CONSTRAINT IF EXISTS chk_pppoe_accounts_account_source;

ALTER TABLE public.pppoe_accounts
    ADD CONSTRAINT chk_pppoe_accounts_account_source
    CHECK (account_source IN ('router', 'managed_radius'));

CREATE INDEX IF NOT EXISTS idx_pppoe_accounts_tenant_source
    ON public.pppoe_accounts (tenant_id, account_source, updated_at DESC);

CREATE TABLE IF NOT EXISTS public.managed_radius_servers (
    id text PRIMARY KEY NOT NULL,
    tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
    name text NOT NULL,
    db_host text NOT NULL,
    db_port integer NOT NULL DEFAULT 5432,
    db_name text NOT NULL,
    db_user text NOT NULL,
    db_password_enc text NOT NULL,
    is_active boolean NOT NULL DEFAULT true,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT uq_managed_radius_servers_tenant_name UNIQUE (tenant_id, name)
);

CREATE INDEX IF NOT EXISTS idx_managed_radius_servers_tenant_active
    ON public.managed_radius_servers (tenant_id, is_active, updated_at DESC);

CREATE TABLE IF NOT EXISTS public.managed_radius_nas (
    id text PRIMARY KEY NOT NULL,
    tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
    router_id text NOT NULL REFERENCES public.mikrotik_routers(id) ON DELETE CASCADE,
    radius_server_id text NOT NULL REFERENCES public.managed_radius_servers(id) ON DELETE CASCADE,
    nas_name text NOT NULL,
    nas_ip_or_cidr text NOT NULL,
    shared_secret_enc text NOT NULL,
    shortname text,
    is_active boolean NOT NULL DEFAULT true,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT uq_managed_radius_nas_router UNIQUE (tenant_id, router_id),
    CONSTRAINT uq_managed_radius_nas_identity UNIQUE (radius_server_id, nas_ip_or_cidr)
);

CREATE INDEX IF NOT EXISTS idx_managed_radius_nas_tenant_active
    ON public.managed_radius_nas (tenant_id, is_active, updated_at DESC);
