CREATE TABLE public.user_addresses (
    id text NOT NULL,
    user_id text NOT NULL,
    label text,
    recipient_name text,
    phone text,
    line1 text NOT NULL,
    line2 text,
    city text,
    state text,
    postal_code text,
    country_code text DEFAULT 'ID'::text NOT NULL,
    is_default_shipping boolean DEFAULT false NOT NULL,
    is_default_billing boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL
);

ALTER TABLE ONLY public.user_addresses
    ADD CONSTRAINT user_addresses_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.user_addresses
    ADD CONSTRAINT user_addresses_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;

CREATE INDEX user_addresses_user_id_idx ON public.user_addresses USING btree (user_id);

-- Enforce a single default shipping/billing address per user.
CREATE UNIQUE INDEX user_addresses_default_shipping_uniq
    ON public.user_addresses (user_id)
    WHERE (is_default_shipping = true);

CREATE UNIQUE INDEX user_addresses_default_billing_uniq
    ON public.user_addresses (user_id)
    WHERE (is_default_billing = true);

