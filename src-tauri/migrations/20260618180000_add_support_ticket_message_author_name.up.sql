-- Add author_name to support ticket messages so the UI can show the
-- actual sender's display name (instead of hardcoding "Customer"/"Staff"
-- or "Anda"/"Admin"). Backfilled from the users table where possible.

ALTER TABLE public.support_ticket_messages
    ADD COLUMN IF NOT EXISTS author_name text;

-- Backfill existing messages — look up the user's name at the time of
-- the message. We use COALESCE so messages whose author has been deleted
-- (SET NULL) fall back to NULL and the UI shows a generic placeholder.
UPDATE public.support_ticket_messages m
SET author_name = u.name
FROM public.users u
WHERE m.author_id = u.id
  AND m.author_name IS NULL;

-- Helpful index for any future "messages by author" queries.
CREATE INDEX IF NOT EXISTS idx_support_ticket_messages_author
    ON public.support_ticket_messages (author_id, created_at DESC);
