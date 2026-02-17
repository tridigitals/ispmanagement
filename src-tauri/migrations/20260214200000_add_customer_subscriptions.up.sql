CREATE TABLE IF NOT EXISTS public.customer_subscriptions (
    id text PRIMARY KEY NOT NULL,
    tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
    customer_id text NOT NULL REFERENCES public.customers(id) ON DELETE CASCADE,
    location_id text NOT NULL REFERENCES public.customer_locations(id) ON DELETE CASCADE,
    package_id text NOT NULL REFERENCES public.isp_packages(id) ON DELETE RESTRICT,
    router_id text REFERENCES public.mikrotik_routers(id) ON DELETE SET NULL,
    billing_cycle text NOT NULL DEFAULT 'monthly',
    price numeric(12,2) NOT NULL,
    currency_code text NOT NULL DEFAULT 'IDR',
    status text NOT NULL DEFAULT 'active',
    starts_at timestamp with time zone,
    ends_at timestamp with time zone,
    notes text,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT customer_subscriptions_billing_cycle_check CHECK (billing_cycle IN ('monthly', 'yearly')),
    CONSTRAINT customer_subscriptions_status_check CHECK (status IN ('active', 'suspended', 'cancelled')),
    CONSTRAINT customer_subscriptions_price_nonneg CHECK (price >= 0)
);

CREATE INDEX IF NOT EXISTS idx_customer_subscriptions_tenant_customer
    ON public.customer_subscriptions(tenant_id, customer_id, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_customer_subscriptions_location
    ON public.customer_subscriptions(location_id, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_customer_subscriptions_package
    ON public.customer_subscriptions(package_id, updated_at DESC);

