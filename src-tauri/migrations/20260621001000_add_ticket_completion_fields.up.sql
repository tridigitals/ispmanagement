-- Sprint 3: support_ticket completion fields
-- Allows field technicians to record progress and proof-of-work when
-- resolving tickets from the mobile-technician app.

ALTER TABLE public.support_tickets
    ADD COLUMN IF NOT EXISTS started_at           timestamptz,
    ADD COLUMN IF NOT EXISTS resolved_at          timestamptz,
    ADD COLUMN IF NOT EXISTS completion_notes     text,
    ADD COLUMN IF NOT EXISTS signature_url        text,
    ADD COLUMN IF NOT EXISTS completion_photos    jsonb NOT NULL DEFAULT '[]'::jsonb;
    -- completion_photos is an array of file_record IDs attached at resolve time.

-- Index for technician productivity queries
CREATE INDEX IF NOT EXISTS idx_support_tickets_assigned_status
    ON public.support_tickets (assigned_to, status);