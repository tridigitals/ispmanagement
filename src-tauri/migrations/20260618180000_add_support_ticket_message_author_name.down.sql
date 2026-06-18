-- Reverse: remove author_name from support ticket messages
DROP INDEX IF EXISTS idx_support_ticket_messages_author;
ALTER TABLE public.support_ticket_messages
    DROP COLUMN IF EXISTS author_name;
