CREATE TABLE IF NOT EXISTS public.customer_registration_invites (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    created_by TEXT REFERENCES public.users(id) ON DELETE SET NULL,
    max_uses BIGINT NOT NULL DEFAULT 1 CHECK (max_uses > 0),
    used_count BIGINT NOT NULL DEFAULT 0 CHECK (used_count >= 0),
    expires_at TIMESTAMPTZ NOT NULL,
    is_revoked BOOLEAN NOT NULL DEFAULT FALSE,
    revoked_at TIMESTAMPTZ,
    last_used_at TIMESTAMPTZ,
    note TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_customer_registration_invites_tenant_created
    ON public.customer_registration_invites (tenant_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_customer_registration_invites_tenant_expires
    ON public.customer_registration_invites (tenant_id, expires_at DESC);

