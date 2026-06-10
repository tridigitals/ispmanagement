-- Remove satisfaction rating from support tickets

ALTER TABLE public.support_tickets
    DROP CONSTRAINT IF EXISTS support_tickets_satisfaction_rating_check;

ALTER TABLE public.support_tickets
    DROP COLUMN IF EXISTS satisfaction_rating;

ALTER TABLE public.support_tickets
    DROP COLUMN IF EXISTS satisfaction_comment;
