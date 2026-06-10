-- Add satisfaction rating to support tickets

ALTER TABLE public.support_tickets
    ADD COLUMN IF NOT EXISTS satisfaction_rating integer;

ALTER TABLE public.support_tickets
    ADD COLUMN IF NOT EXISTS satisfaction_comment text;

-- CHECK constraint for rating (1-5)
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'support_tickets_satisfaction_rating_check'
    ) THEN
        ALTER TABLE public.support_tickets
            ADD CONSTRAINT support_tickets_satisfaction_rating_check
            CHECK (satisfaction_rating IS NULL OR (satisfaction_rating >= 1 AND satisfaction_rating <= 5));
    END IF;
END $$;
