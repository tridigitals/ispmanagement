-- Add registration_status and audit columns to users table.
-- Existing users default to 'active' so no data migration needed.

ALTER TABLE public.users
    ADD COLUMN IF NOT EXISTS registration_status VARCHAR(20) NOT NULL DEFAULT 'active'
        CHECK (registration_status IN ('active', 'pending', 'rejected'));

ALTER TABLE public.users
    ADD COLUMN IF NOT EXISTS pending_review_message TEXT;

ALTER TABLE public.users
    ADD COLUMN IF NOT EXISTS approved_at TIMESTAMPTZ;

ALTER TABLE public.users
    ADD COLUMN IF NOT EXISTS approved_by_user_id TEXT;

ALTER TABLE public.users
    ADD COLUMN IF NOT EXISTS rejected_at TIMESTAMPTZ;

ALTER TABLE public.users
    ADD COLUMN IF NOT EXISTS rejected_reason TEXT;

ALTER TABLE public.users
    ADD COLUMN IF NOT EXISTS rejected_by_user_id TEXT;

-- Index for fast filtering of pending users (superadmin list query)
CREATE INDEX IF NOT EXISTS idx_users_registration_status ON public.users (registration_status)
    WHERE registration_status != 'active';

-- FK constraints (text-based, matching existing id pattern)
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'fk_users_approved_by'
    ) THEN
        ALTER TABLE public.users
            ADD CONSTRAINT fk_users_approved_by
            FOREIGN KEY (approved_by_user_id) REFERENCES public.users(id) ON DELETE SET NULL;
    END IF;
END $$;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'fk_users_rejected_by'
    ) THEN
        ALTER TABLE public.users
            ADD CONSTRAINT fk_users_rejected_by
            FOREIGN KEY (rejected_by_user_id) REFERENCES public.users(id) ON DELETE SET NULL;
    END IF;
END $$;
