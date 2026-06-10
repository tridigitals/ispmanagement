-- Reverse: remove category and subscription_id

DROP INDEX IF EXISTS idx_support_tickets_tenant_category;
DROP INDEX IF EXISTS idx_support_tickets_subscription;

ALTER TABLE public.support_tickets
    DROP CONSTRAINT IF EXISTS support_tickets_category_check;

ALTER TABLE public.support_tickets
    DROP COLUMN IF EXISTS category;

ALTER TABLE public.support_tickets
    DROP COLUMN IF EXISTS subscription_id;
