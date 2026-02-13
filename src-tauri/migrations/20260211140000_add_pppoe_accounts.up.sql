-- PPPoE Accounts (tenant-scoped) tied to customer locations + routers
-- DB is the source-of-truth. Router state is reconciled periodically.

CREATE TABLE IF NOT EXISTS public.pppoe_profiles (
    id text PRIMARY KEY NOT NULL,
    tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
    name text NOT NULL,
    -- Optional: store common profile settings (we may apply to routers later)
    rate_limit text,
    session_timeout_seconds integer,
    is_active boolean NOT NULL DEFAULT true,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT pppoe_profiles_tenant_name_unique UNIQUE (tenant_id, name)
);

CREATE INDEX IF NOT EXISTS idx_pppoe_profiles_tenant ON public.pppoe_profiles(tenant_id);

CREATE TABLE IF NOT EXISTS public.pppoe_accounts (
    id text PRIMARY KEY NOT NULL,
    tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
    router_id text NOT NULL REFERENCES public.mikrotik_routers(id) ON DELETE CASCADE,
    customer_id text NOT NULL REFERENCES public.customers(id) ON DELETE CASCADE,
    location_id text NOT NULL REFERENCES public.customer_locations(id) ON DELETE CASCADE,
    username text NOT NULL,
    password_enc text NOT NULL,
    profile_id text REFERENCES public.pppoe_profiles(id) ON DELETE SET NULL,
    -- Optional router-side profile name override (if set, used instead of profile_id.name)
    router_profile_name text,
    -- IP assignment: use either a static remote address or a pool name
    remote_address text,
    address_pool text,
    disabled boolean NOT NULL DEFAULT false,
    comment text,
    -- Sync state
    router_present boolean NOT NULL DEFAULT false,
    router_secret_id text,
    last_sync_at timestamp with time zone,
    last_error text,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT pppoe_accounts_tenant_router_username_unique UNIQUE (tenant_id, router_id, username)
);

CREATE INDEX IF NOT EXISTS idx_pppoe_accounts_tenant ON public.pppoe_accounts(tenant_id);
CREATE INDEX IF NOT EXISTS idx_pppoe_accounts_router ON public.pppoe_accounts(router_id, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_pppoe_accounts_customer ON public.pppoe_accounts(customer_id, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_pppoe_accounts_location ON public.pppoe_accounts(location_id, updated_at DESC);

