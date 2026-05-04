CREATE INDEX IF NOT EXISTS idx_customer_subscriptions_tenant_status_customer
    ON public.customer_subscriptions(tenant_id, status, customer_id);

