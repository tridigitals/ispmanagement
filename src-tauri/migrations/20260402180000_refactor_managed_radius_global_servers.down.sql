CREATE TABLE IF NOT EXISTS public.managed_radius_servers (
    id text PRIMARY KEY NOT NULL,
    tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
    name text NOT NULL,
    db_host text NOT NULL,
    db_port integer NOT NULL DEFAULT 5432,
    db_name text NOT NULL,
    db_user text NOT NULL,
    db_password_enc text NOT NULL,
    is_active boolean NOT NULL DEFAULT true,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT uq_managed_radius_servers_tenant_name UNIQUE (tenant_id, name)
);

CREATE INDEX IF NOT EXISTS idx_managed_radius_servers_tenant_active
    ON public.managed_radius_servers (tenant_id, is_active, updated_at DESC);

INSERT INTO public.managed_radius_servers (
    id,
    tenant_id,
    name,
    db_host,
    db_port,
    db_name,
    db_user,
    db_password_enc,
    is_active,
    created_at,
    updated_at
)
SELECT
    assignment.id,
    assignment.tenant_id,
    server.name,
    server.db_host,
    server.db_port,
    server.db_name,
    server.db_user,
    server.db_password_enc,
    assignment.is_active,
    assignment.created_at,
    assignment.updated_at
FROM public.tenant_radius_assignments assignment
INNER JOIN public.radius_servers server
  ON server.id = assignment.radius_server_id
ON CONFLICT (id) DO NOTHING;

ALTER TABLE public.managed_radius_nas
    DROP CONSTRAINT IF EXISTS managed_radius_nas_radius_server_id_fkey;

UPDATE public.managed_radius_nas nas
SET radius_server_id = assignment.id
FROM public.tenant_radius_assignments assignment
WHERE assignment.radius_server_id = nas.radius_server_id
  AND assignment.tenant_id = nas.tenant_id;

ALTER TABLE public.managed_radius_nas
    ADD CONSTRAINT managed_radius_nas_radius_server_id_fkey
    FOREIGN KEY (radius_server_id)
    REFERENCES public.managed_radius_servers(id)
    ON DELETE CASCADE;

DROP TABLE IF EXISTS public.tenant_radius_assignments;
DROP TABLE IF EXISTS public.radius_servers;
