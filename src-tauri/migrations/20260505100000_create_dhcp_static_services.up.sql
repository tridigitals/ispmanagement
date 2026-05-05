CREATE TABLE IF NOT EXISTS public.dhcp_static_services (
    id text PRIMARY KEY NOT NULL,
    tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
    subscription_id text NOT NULL REFERENCES public.customer_subscriptions(id) ON DELETE CASCADE,
    router_id text NOT NULL REFERENCES public.mikrotik_routers(id) ON DELETE CASCADE,
    customer_id text NOT NULL REFERENCES public.customers(id) ON DELETE CASCADE,
    location_id text NOT NULL REFERENCES public.customer_locations(id) ON DELETE CASCADE,
    package_id text NOT NULL REFERENCES public.isp_packages(id) ON DELETE RESTRICT,
    dhcp_server_name text NOT NULL,
    mac_address text NOT NULL,
    ip_address text NOT NULL,
    comment text,
    disabled boolean NOT NULL DEFAULT false,
    lease_present boolean NOT NULL DEFAULT false,
    lease_router_ref text,
    lease_last_sync_at timestamp with time zone,
    lease_last_error text,
    queue_mode text NOT NULL DEFAULT 'none',
    queue_name text,
    queue_target text,
    queue_rate_limit text,
    queue_present boolean NOT NULL DEFAULT false,
    queue_last_sync_at timestamp with time zone,
    queue_last_error text,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT dhcp_static_services_queue_mode_check CHECK (queue_mode IN ('none', 'simple_queue')),
    CONSTRAINT uq_dhcp_static_subscription UNIQUE (tenant_id, subscription_id),
    CONSTRAINT uq_dhcp_static_router_mac UNIQUE (tenant_id, router_id, mac_address),
    CONSTRAINT uq_dhcp_static_router_ip UNIQUE (tenant_id, router_id, ip_address)
);

CREATE INDEX IF NOT EXISTS idx_dhcp_static_services_router
    ON public.dhcp_static_services (tenant_id, router_id, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_dhcp_static_services_customer
    ON public.dhcp_static_services (tenant_id, customer_id, updated_at DESC);
