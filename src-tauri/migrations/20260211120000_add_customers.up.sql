-- Customers + Locations (tenant-scoped) for ISP Management
-- Purpose: allow admin to manage customers with multiple service locations,
-- and optionally map portal users (login to /dashboard) to a customer.

CREATE TABLE IF NOT EXISTS public.customers (
    id text NOT NULL,
    tenant_id text NOT NULL,
    name text NOT NULL,
    email text,
    phone text,
    notes text,
    is_active boolean DEFAULT true NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT customers_pkey PRIMARY KEY (id),
    CONSTRAINT customers_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS customers_tenant_id_idx ON public.customers(tenant_id);
CREATE INDEX IF NOT EXISTS customers_tenant_name_idx ON public.customers(tenant_id, name);

CREATE TABLE IF NOT EXISTS public.customer_locations (
    id text NOT NULL,
    tenant_id text NOT NULL,
    customer_id text NOT NULL,
    label text NOT NULL,
    address_line1 text,
    address_line2 text,
    city text,
    state text,
    postal_code text,
    country text,
    latitude numeric(10,6),
    longitude numeric(10,6),
    notes text,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT customer_locations_pkey PRIMARY KEY (id),
    CONSTRAINT customer_locations_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE,
    CONSTRAINT customer_locations_customer_id_fkey FOREIGN KEY (customer_id) REFERENCES public.customers(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS customer_locations_tenant_id_idx ON public.customer_locations(tenant_id);
CREATE INDEX IF NOT EXISTS customer_locations_customer_id_idx ON public.customer_locations(customer_id);

-- Maps application users to a customer so they can access the customer portal (/dashboard).
CREATE TABLE IF NOT EXISTS public.customer_users (
    id text NOT NULL,
    tenant_id text NOT NULL,
    customer_id text NOT NULL,
    user_id text NOT NULL,
    created_at timestamp with time zone NOT NULL,
    CONSTRAINT customer_users_pkey PRIMARY KEY (id),
    CONSTRAINT customer_users_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE,
    CONSTRAINT customer_users_customer_id_fkey FOREIGN KEY (customer_id) REFERENCES public.customers(id) ON DELETE CASCADE,
    CONSTRAINT customer_users_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE,
    CONSTRAINT customer_users_tenant_user_unique UNIQUE (tenant_id, user_id),
    CONSTRAINT customer_users_customer_user_unique UNIQUE (customer_id, user_id)
);

CREATE INDEX IF NOT EXISTS customer_users_tenant_id_idx ON public.customer_users(tenant_id);
CREATE INDEX IF NOT EXISTS customer_users_customer_id_idx ON public.customer_users(customer_id);
CREATE INDEX IF NOT EXISTS customer_users_user_id_idx ON public.customer_users(user_id);

