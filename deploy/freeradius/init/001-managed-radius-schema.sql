CREATE TABLE IF NOT EXISTS managed_radius_nas (
    id text PRIMARY KEY,
    tenant_id text NOT NULL,
    router_id text NOT NULL,
    nas_name text NOT NULL,
    nas_ip_or_cidr text NOT NULL,
    shared_secret text NOT NULL,
    shortname text,
    is_active boolean NOT NULL DEFAULT true,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT uq_managed_radius_nas_router UNIQUE (tenant_id, router_id),
    CONSTRAINT uq_managed_radius_nas_ip UNIQUE (nas_ip_or_cidr)
);

CREATE TABLE IF NOT EXISTS managed_radius_accounts (
    id text PRIMARY KEY,
    tenant_id text NOT NULL,
    router_id text NOT NULL,
    username text NOT NULL,
    radius_identity text NOT NULL,
    cleartext_password text NOT NULL,
    profile_name text,
    remote_address text,
    address_pool text,
    disabled boolean NOT NULL DEFAULT false,
    comment text,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT uq_managed_radius_accounts_username UNIQUE (tenant_id, username),
    CONSTRAINT uq_managed_radius_accounts_identity UNIQUE (tenant_id, radius_identity)
);

CREATE INDEX IF NOT EXISTS idx_managed_radius_accounts_tenant_router
    ON managed_radius_accounts (tenant_id, router_id, username);
