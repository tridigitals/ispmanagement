ALTER TABLE public.customer_registration_invites
    ADD COLUMN IF NOT EXISTS token_enc TEXT;
