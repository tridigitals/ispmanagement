CREATE TABLE IF NOT EXISTS public.installation_work_orders (
    id text PRIMARY KEY NOT NULL,
    tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
    subscription_id text NOT NULL REFERENCES public.customer_subscriptions(id) ON DELETE CASCADE,
    invoice_id text REFERENCES public.invoices(id) ON DELETE SET NULL,
    customer_id text NOT NULL REFERENCES public.customers(id) ON DELETE CASCADE,
    location_id text NOT NULL REFERENCES public.customer_locations(id) ON DELETE CASCADE,
    router_id text REFERENCES public.mikrotik_routers(id) ON DELETE SET NULL,
    status text NOT NULL DEFAULT 'pending', -- pending | in_progress | completed | cancelled
    assigned_to text REFERENCES public.users(id) ON DELETE SET NULL,
    scheduled_at timestamp with time zone NULL,
    completed_at timestamp with time zone NULL,
    notes text,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_installation_work_orders_tenant_status
    ON public.installation_work_orders (tenant_id, status, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_installation_work_orders_subscription
    ON public.installation_work_orders (subscription_id, created_at DESC);

