-- Add category and subscription_id to support tickets

ALTER TABLE public.support_tickets
    ADD COLUMN IF NOT EXISTS category text;

ALTER TABLE public.support_tickets
    ADD COLUMN IF NOT EXISTS subscription_id text
    REFERENCES public.customer_subscriptions(id) ON DELETE SET NULL;

-- CHECK constraint for category
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'support_tickets_category_check'
    ) THEN
        ALTER TABLE public.support_tickets
            ADD CONSTRAINT support_tickets_category_check
            CHECK (category IN ('general', 'billing', 'technical', 'installation'));
    END IF;
END $$;

-- Index for filtering by category
CREATE INDEX IF NOT EXISTS idx_support_tickets_tenant_category
    ON public.support_tickets (tenant_id, category, updated_at DESC);

-- Index for finding tickets by subscription
CREATE INDEX IF NOT EXISTS idx_support_tickets_subscription
    ON public.support_tickets (subscription_id, updated_at DESC);
