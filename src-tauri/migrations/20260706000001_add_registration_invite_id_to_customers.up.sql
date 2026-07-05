ALTER TABLE public.customers
    ADD COLUMN IF NOT EXISTS registration_invite_id TEXT REFERENCES public.customer_registration_invites(id) ON DELETE SET NULL;

COMMENT ON COLUMN public.customers.registration_invite_id IS 'Points to the invite link used to register this customer (nullable)';
