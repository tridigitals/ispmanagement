ALTER TABLE public.customer_registration_invites
    ADD COLUMN IF NOT EXISTS invite_url TEXT;

UPDATE public.customer_registration_invites
SET invite_url = ''
WHERE invite_url IS NULL;

COMMENT ON COLUMN public.customer_registration_invites.invite_url IS 'Full invite URL (e.g. https://domain/register?invite=...)';
