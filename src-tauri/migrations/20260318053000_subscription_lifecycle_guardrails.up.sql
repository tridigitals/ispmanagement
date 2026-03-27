ALTER TABLE public.customer_subscriptions
    DROP CONSTRAINT IF EXISTS customer_subscriptions_status_check;

ALTER TABLE public.customer_subscriptions
    ADD CONSTRAINT customer_subscriptions_status_check
    CHECK (
        status IN (
            'active',
            'pending_installation',
            'installation_done_awaiting_payment',
            'suspended',
            'cancelled'
        )
    );

CREATE UNIQUE INDEX IF NOT EXISTS idx_installation_work_orders_one_active_per_subscription
    ON public.installation_work_orders (tenant_id, subscription_id)
    WHERE status IN ('pending', 'in_progress');
