DROP INDEX IF EXISTS idx_users_registration_status;

ALTER TABLE public.users DROP CONSTRAINT IF EXISTS fk_users_approved_by;
ALTER TABLE public.users DROP CONSTRAINT IF EXISTS fk_users_rejected_by;

ALTER TABLE public.users DROP COLUMN IF EXISTS registration_status;
ALTER TABLE public.users DROP COLUMN IF EXISTS pending_review_message;
ALTER TABLE public.users DROP COLUMN IF EXISTS approved_at;
ALTER TABLE public.users DROP COLUMN IF EXISTS approved_by_user_id;
ALTER TABLE public.users DROP COLUMN IF EXISTS rejected_at;
ALTER TABLE public.users DROP COLUMN IF EXISTS rejected_reason;
ALTER TABLE public.users DROP COLUMN IF EXISTS rejected_by_user_id;
