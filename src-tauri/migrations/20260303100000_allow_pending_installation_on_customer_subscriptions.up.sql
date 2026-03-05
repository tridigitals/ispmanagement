ALTER TABLE public.customer_subscriptions
    DROP CONSTRAINT IF EXISTS customer_subscriptions_status_check;

ALTER TABLE public.customer_subscriptions
    ADD CONSTRAINT customer_subscriptions_status_check
    CHECK (status IN ('active', 'pending_installation', 'suspended', 'cancelled'));
