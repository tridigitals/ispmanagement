-- Managed RADIUS global-server refactor
-- Required outcomes:
-- 1. create global radius_servers
-- 2. create tenant_radius_assignments
-- 3. migrate and deduplicate legacy tenant-scoped managed_radius_servers
-- 4. repoint managed_radius_nas.radius_server_id to new global IDs
-- 5. preserve existing data while removing the legacy server table

CREATE TABLE IF NOT EXISTS public.radius_servers (
    id text PRIMARY KEY NOT NULL,
    name text NOT NULL,
    db_host text NOT NULL,
    db_port integer NOT NULL DEFAULT 5432,
    db_name text NOT NULL,
    db_user text NOT NULL,
    db_password_enc text NOT NULL,
    is_active boolean NOT NULL DEFAULT true,
    notes text,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT uq_radius_servers_name UNIQUE (name)
);

CREATE TABLE IF NOT EXISTS public.tenant_radius_assignments (
    id text PRIMARY KEY NOT NULL,
    tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
    radius_server_id text NOT NULL REFERENCES public.radius_servers(id) ON DELETE CASCADE,
    is_active boolean NOT NULL DEFAULT true,
    assigned_at timestamp with time zone NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_tenant_radius_assignments_tenant_updated
    ON public.tenant_radius_assignments (tenant_id, updated_at DESC);

CREATE UNIQUE INDEX IF NOT EXISTS uq_tenant_radius_assignments_active_tenant
    ON public.tenant_radius_assignments (tenant_id)
    WHERE is_active = true;

WITH legacy_servers AS (
    SELECT
        s.id AS legacy_server_id,
        COALESCE(existing.id, gen_random_uuid()::text) AS global_server_id,
        s.tenant_id,
        s.name,
        s.db_host,
        s.db_port,
        s.db_name,
        s.db_user,
        s.db_password_enc,
        s.is_active,
        s.created_at,
        s.updated_at
    FROM public.managed_radius_servers s
    LEFT JOIN public.radius_servers existing
      ON existing.name = s.name
),
inserted_servers AS (
    INSERT INTO public.radius_servers (
        id,
        name,
        db_host,
        db_port,
        db_name,
        db_user,
        db_password_enc,
        is_active,
        notes,
        created_at,
        updated_at
    )
    SELECT DISTINCT ON (global_server_id)
        global_server_id,
        name,
        db_host,
        db_port,
        db_name,
        db_user,
        db_password_enc,
        is_active,
        NULL::text,
        created_at,
        updated_at
    FROM legacy_servers
    ON CONFLICT (id) DO NOTHING
    RETURNING id
)
INSERT INTO public.tenant_radius_assignments (
    id,
    tenant_id,
    radius_server_id,
    is_active,
    assigned_at,
    created_at,
    updated_at
)
SELECT
    gen_random_uuid()::text,
    legacy.tenant_id,
    legacy.global_server_id,
    legacy.is_active,
    legacy.updated_at,
    legacy.created_at,
    legacy.updated_at
FROM legacy_servers legacy
ON CONFLICT DO NOTHING;

ALTER TABLE public.managed_radius_nas
    DROP CONSTRAINT IF EXISTS managed_radius_nas_radius_server_id_fkey;

WITH legacy_servers AS (
    SELECT
        s.id AS legacy_server_id,
        replacement.id AS global_server_id
    FROM public.managed_radius_servers s
    INNER JOIN public.radius_servers replacement
      ON replacement.name = s.name
)
UPDATE public.managed_radius_nas n
SET radius_server_id = legacy.global_server_id
FROM legacy_servers legacy
WHERE n.radius_server_id = legacy.legacy_server_id;

ALTER TABLE public.managed_radius_nas
    ADD CONSTRAINT managed_radius_nas_radius_server_id_fkey
    FOREIGN KEY (radius_server_id)
    REFERENCES public.radius_servers(id)
    ON DELETE CASCADE;

DROP INDEX IF EXISTS public.idx_managed_radius_servers_tenant_active;
DROP TABLE IF EXISTS public.managed_radius_servers;
