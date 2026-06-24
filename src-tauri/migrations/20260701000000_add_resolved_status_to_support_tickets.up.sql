-- Add 'resolved' status to support_tickets check constraint

ALTER TABLE public.support_tickets
    DROP CONSTRAINT IF EXISTS support_tickets_status_check;

ALTER TABLE public.support_tickets
    ADD CONSTRAINT support_tickets_status_check
    CHECK (status IN ('open', 'pending', 'closed', 'resolved'));
