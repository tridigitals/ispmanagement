CREATE TABLE IF NOT EXISTS public.mixradius_import_batches (
    id text PRIMARY KEY NOT NULL,
    tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
    source_filename text NOT NULL,
    source_sha256 text NOT NULL,
    source_size_bytes bigint NOT NULL,
    parse_status text NOT NULL DEFAULT 'pending',
    execution_status text NOT NULL DEFAULT 'pending',
    execution_mode text NOT NULL DEFAULT 'preview_only',
    started_at timestamp with time zone,
    completed_at timestamp with time zone,
    progress_json jsonb NOT NULL DEFAULT '{}'::jsonb,
    summary_json jsonb NOT NULL DEFAULT '{}'::jsonb,
    error_json jsonb NOT NULL DEFAULT '[]'::jsonb,
    created_by text REFERENCES public.users(id) ON DELETE SET NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT chk_mixradius_import_batches_source_filename
        CHECK (btrim(source_filename) <> ''),
    CONSTRAINT chk_mixradius_import_batches_source_sha256
        CHECK (btrim(source_sha256) <> ''),
    CONSTRAINT chk_mixradius_import_batches_source_size_bytes
        CHECK (source_size_bytes > 0),
    CONSTRAINT chk_mixradius_import_batches_parse_status
        CHECK (parse_status IN ('pending', 'running', 'ready', 'failed')),
    CONSTRAINT chk_mixradius_import_batches_execution_status
        CHECK (execution_status IN ('pending', 'running', 'completed', 'failed', 'cancelled')),
    CONSTRAINT chk_mixradius_import_batches_execution_mode
        CHECK (execution_mode IN ('preview_only', 'safe_import', 'force_sync'))
);

CREATE INDEX IF NOT EXISTS idx_mixradius_import_batches_tenant_status
    ON public.mixradius_import_batches (tenant_id, execution_status, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_mixradius_import_batches_tenant_created
    ON public.mixradius_import_batches (tenant_id, created_at DESC);

CREATE TABLE IF NOT EXISTS public.mixradius_import_external_refs (
    id text PRIMARY KEY NOT NULL,
    tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
    import_batch_id text NOT NULL REFERENCES public.mixradius_import_batches(id) ON DELETE CASCADE,
    entity_type text NOT NULL,
    entity_id text NOT NULL,
    source_system text NOT NULL DEFAULT 'mixradius',
    source_ref text NOT NULL,
    last_seen_at timestamp with time zone,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT uq_mixradius_import_external_refs_source
        UNIQUE (tenant_id, source_system, entity_type, source_ref),
    CONSTRAINT uq_mixradius_import_external_refs_entity
        UNIQUE (tenant_id, entity_type, entity_id)
);

CREATE INDEX IF NOT EXISTS idx_mixradius_import_external_refs_batch
    ON public.mixradius_import_external_refs (tenant_id, import_batch_id, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_mixradius_import_external_refs_entity_lookup
    ON public.mixradius_import_external_refs (tenant_id, entity_type, entity_id);

CREATE TABLE IF NOT EXISTS public.mixradius_staging_nas (
    id text PRIMARY KEY NOT NULL,
    tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
    import_batch_id text NOT NULL REFERENCES public.mixradius_import_batches(id) ON DELETE CASCADE,
    source_ref text NOT NULL,
    nas_name text NOT NULL,
    nas_ip_or_cidr text NOT NULL,
    shortname text,
    source_json jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT uq_mixradius_staging_nas_source UNIQUE (tenant_id, import_batch_id, source_ref)
);

CREATE INDEX IF NOT EXISTS idx_mixradius_staging_nas_tenant_batch
    ON public.mixradius_staging_nas (tenant_id, import_batch_id, updated_at DESC);

CREATE TABLE IF NOT EXISTS public.mixradius_staging_plans (
    id text PRIMARY KEY NOT NULL,
    tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
    import_batch_id text NOT NULL REFERENCES public.mixradius_import_batches(id) ON DELETE CASCADE,
    source_ref text NOT NULL,
    plan_name text NOT NULL,
    bandwidth_name text,
    price numeric(18,2),
    validity text,
    shared_users integer,
    source_json jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT uq_mixradius_staging_plans_source UNIQUE (tenant_id, import_batch_id, source_ref)
);

CREATE INDEX IF NOT EXISTS idx_mixradius_staging_plans_tenant_batch
    ON public.mixradius_staging_plans (tenant_id, import_batch_id, updated_at DESC);

CREATE TABLE IF NOT EXISTS public.mixradius_staging_customers (
    id text PRIMARY KEY NOT NULL,
    tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
    import_batch_id text NOT NULL REFERENCES public.mixradius_import_batches(id) ON DELETE CASCADE,
    source_ref text NOT NULL,
    member_id text NOT NULL,
    username text,
    password text,
    fullname text,
    email text,
    phonenumber text,
    identity_number text,
    address text,
    plan_name text,
    price numeric(18,2),
    total numeric(18,2),
    renewed_on timestamp with time zone,
    expired_on timestamp with time zone,
    trx_invoice text,
    trx_status text,
    payment_type text,
    auth_status text,
    bind_mac text,
    mac_address text,
    latitude numeric(10,6),
    longitude numeric(10,6),
    odp_id text,
    odp_name text,
    source_json jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT uq_mixradius_staging_customers_source UNIQUE (tenant_id, import_batch_id, source_ref)
);

CREATE INDEX IF NOT EXISTS idx_mixradius_staging_customers_tenant_batch
    ON public.mixradius_staging_customers (tenant_id, import_batch_id, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_mixradius_staging_customers_member_id
    ON public.mixradius_staging_customers (tenant_id, member_id);

CREATE TABLE IF NOT EXISTS public.mixradius_staging_customer_locations (
    id text PRIMARY KEY NOT NULL,
    tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
    import_batch_id text NOT NULL REFERENCES public.mixradius_import_batches(id) ON DELETE CASCADE,
    source_ref text NOT NULL,
    member_id text NOT NULL,
    location_label text,
    address_line1 text,
    address_line2 text,
    city text,
    state text,
    postal_code text,
    country text,
    latitude numeric(10,6),
    longitude numeric(10,6),
    source_json jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT uq_mixradius_staging_customer_locations_source
        UNIQUE (tenant_id, import_batch_id, source_ref)
);

CREATE INDEX IF NOT EXISTS idx_mixradius_staging_customer_locations_tenant_batch
    ON public.mixradius_staging_customer_locations (tenant_id, import_batch_id, updated_at DESC);

CREATE TABLE IF NOT EXISTS public.mixradius_staging_transactions (
    id text PRIMARY KEY NOT NULL,
    tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
    import_batch_id text NOT NULL REFERENCES public.mixradius_import_batches(id) ON DELETE CASCADE,
    source_ref text NOT NULL,
    invoice_no text,
    member_id text,
    username text,
    transaction_status text,
    payment_type text,
    amount numeric(18,2),
    paid_at timestamp with time zone,
    source_json jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT uq_mixradius_staging_transactions_source UNIQUE (tenant_id, import_batch_id, source_ref)
);

CREATE INDEX IF NOT EXISTS idx_mixradius_staging_transactions_tenant_batch
    ON public.mixradius_staging_transactions (tenant_id, import_batch_id, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_mixradius_staging_transactions_invoice
    ON public.mixradius_staging_transactions (tenant_id, invoice_no);

CREATE TABLE IF NOT EXISTS public.mixradius_staging_usage (
    id text PRIMARY KEY NOT NULL,
    tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
    import_batch_id text NOT NULL REFERENCES public.mixradius_import_batches(id) ON DELETE CASCADE,
    source_ref text NOT NULL,
    member_id text,
    username text,
    usage_date date,
    session_count integer,
    download_bytes bigint,
    upload_bytes bigint,
    source_json jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT uq_mixradius_staging_usage_source UNIQUE (tenant_id, import_batch_id, source_ref)
);

CREATE INDEX IF NOT EXISTS idx_mixradius_staging_usage_tenant_batch
    ON public.mixradius_staging_usage (tenant_id, import_batch_id, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_mixradius_staging_usage_member_date
    ON public.mixradius_staging_usage (tenant_id, member_id, usage_date DESC);

CREATE TABLE IF NOT EXISTS public.mixradius_import_conflicts (
    id text PRIMARY KEY NOT NULL,
    tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
    import_batch_id text NOT NULL REFERENCES public.mixradius_import_batches(id) ON DELETE CASCADE,
    source_table text NOT NULL,
    source_ref text NOT NULL,
    conflict_type text NOT NULL,
    severity text NOT NULL DEFAULT 'warning',
    conflict_message text NOT NULL,
    resolution_status text NOT NULL DEFAULT 'open',
    details_json jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT chk_mixradius_import_conflicts_severity
        CHECK (severity IN ('info', 'warning', 'error')),
    CONSTRAINT chk_mixradius_import_conflicts_resolution_status
        CHECK (resolution_status IN ('open', 'resolved', 'ignored'))
);

CREATE INDEX IF NOT EXISTS idx_mixradius_import_conflicts_tenant_batch
    ON public.mixradius_import_conflicts (tenant_id, import_batch_id, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_mixradius_import_conflicts_source
    ON public.mixradius_import_conflicts (tenant_id, source_table, source_ref);
