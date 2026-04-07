ALTER TABLE public.customer_subscriptions
    DROP CONSTRAINT IF EXISTS customer_subscriptions_status_check;

UPDATE public.customer_subscriptions
SET status = 'installation_done_awaiting_payment'
WHERE status = 'grace_active';

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

ALTER TABLE public.customer_subscriptions
    DROP COLUMN IF EXISTS grace_until,
    DROP COLUMN IF EXISTS grace_started_at;
