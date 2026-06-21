ALTER TABLE public.support_tickets
    DROP COLUMN IF EXISTS started_at,
    DROP COLUMN IF EXISTS resolved_at,
    DROP COLUMN IF EXISTS completion_notes,
    DROP COLUMN IF EXISTS signature_url,
    DROP COLUMN IF EXISTS completion_photos;

DROP INDEX IF EXISTS idx_support_tickets_assigned_status;