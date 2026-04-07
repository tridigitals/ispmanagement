ALTER TABLE public.customer_subscriptions
    ADD COLUMN IF NOT EXISTS grace_started_at timestamp with time zone,
    ADD COLUMN IF NOT EXISTS grace_until timestamp with time zone;

UPDATE public.customer_subscriptions
SET status = 'grace_active',
    grace_started_at = COALESCE(grace_started_at, updated_at),
    grace_until = COALESCE(grace_until, updated_at + INTERVAL '3 day')
WHERE status = 'installation_done_awaiting_payment';

ALTER TABLE public.customer_subscriptions
    DROP CONSTRAINT IF EXISTS customer_subscriptions_status_check;

ALTER TABLE public.customer_subscriptions
    ADD CONSTRAINT customer_subscriptions_status_check
    CHECK (
        status IN (
            'active',
            'grace_active',
            'pending_installation',
            'installation_done_awaiting_payment',
            'suspended',
            'cancelled'
        )
    );
