CREATE TABLE IF NOT EXISTS public.message_templates (
    id text PRIMARY KEY,
    tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
    key text NOT NULL,
    name text NOT NULL,
    description text,
    use_case text NOT NULL DEFAULT 'custom',
    target text NOT NULL DEFAULT 'customer',
    trigger_mode text NOT NULL DEFAULT 'manual',
    event_key text,
    channel text NOT NULL DEFAULT 'whatsapp',
    locale text NOT NULL DEFAULT 'id-ID',
    status text NOT NULL DEFAULT 'draft',
    whatsapp_body text,
    email_subject text,
    email_body text,
    variables text NOT NULL DEFAULT '[]',
    version bigint NOT NULL DEFAULT 1,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT uq_message_templates_tenant_key UNIQUE (tenant_id, key)
);

CREATE INDEX IF NOT EXISTS idx_message_templates_tenant_updated
    ON public.message_templates (tenant_id, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_message_templates_tenant_filters
    ON public.message_templates (tenant_id, status, channel, target, trigger_mode);

INSERT INTO public.permissions (id, resource, action, description)
VALUES
    ('perm_communication_templates_read', 'communication_templates', 'read', 'View communication message templates'),
    ('perm_communication_templates_manage', 'communication_templates', 'manage', 'Manage communication message templates')
ON CONFLICT (resource, action) DO UPDATE
SET description = EXCLUDED.description;

INSERT INTO public.role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM public.roles r
JOIN public.permissions p ON p.resource = 'communication_templates'
WHERE r.name IN ('Owner', 'Admin', 'Customer Service')
  AND p.action IN ('read', 'manage')
ON CONFLICT DO NOTHING;
