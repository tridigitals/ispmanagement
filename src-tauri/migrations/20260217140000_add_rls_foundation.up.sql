-- RLS Foundation (PostgreSQL)
-- Staged rollout:
-- 1) Enable RLS + create policies on tenant-scoped tables.
-- 2) Policies are intentionally backward-compatible when app context is not set:
--    if app.current_tenant_id is NULL/empty, access is not restricted yet.
-- 3) Full enforcement can be enabled later after request-scoped DB context is implemented.

-- Helper expression semantics used in policies:
-- - superadmin context:
--     current_setting('app.current_is_superadmin', true) = 'true'
-- - tenant context:
--     nullif(current_setting('app.current_tenant_id', true), '')
-- - staged fallback:
--     if tenant context is NULL, allow (backward-compatible mode)

DO $$
DECLARE
    r RECORD;
BEGIN
    -- Generic RLS for all tables that have tenant_id column.
    FOR r IN
        SELECT c.table_name
        FROM information_schema.columns c
        WHERE c.table_schema = 'public'
          AND c.column_name = 'tenant_id'
        GROUP BY c.table_name
    LOOP
        EXECUTE format('ALTER TABLE public.%I ENABLE ROW LEVEL SECURITY', r.table_name);

        EXECUTE format('DROP POLICY IF EXISTS p_tenant_select ON public.%I', r.table_name);
        EXECUTE format('DROP POLICY IF EXISTS p_tenant_insert ON public.%I', r.table_name);
        EXECUTE format('DROP POLICY IF EXISTS p_tenant_update ON public.%I', r.table_name);
        EXECUTE format('DROP POLICY IF EXISTS p_tenant_delete ON public.%I', r.table_name);

        EXECUTE format($sql$
            CREATE POLICY p_tenant_select ON public.%I
            FOR SELECT
            USING (
                current_setting('app.current_is_superadmin', true) = 'true'
                OR nullif(current_setting('app.current_tenant_id', true), '') IS NULL
                OR tenant_id::text = nullif(current_setting('app.current_tenant_id', true), '')
                OR tenant_id IS NULL
            )
        $sql$, r.table_name);

        EXECUTE format($sql$
            CREATE POLICY p_tenant_insert ON public.%I
            FOR INSERT
            WITH CHECK (
                current_setting('app.current_is_superadmin', true) = 'true'
                OR nullif(current_setting('app.current_tenant_id', true), '') IS NULL
                OR tenant_id::text = nullif(current_setting('app.current_tenant_id', true), '')
                OR tenant_id IS NULL
            )
        $sql$, r.table_name);

        EXECUTE format($sql$
            CREATE POLICY p_tenant_update ON public.%I
            FOR UPDATE
            USING (
                current_setting('app.current_is_superadmin', true) = 'true'
                OR nullif(current_setting('app.current_tenant_id', true), '') IS NULL
                OR tenant_id::text = nullif(current_setting('app.current_tenant_id', true), '')
                OR tenant_id IS NULL
            )
            WITH CHECK (
                current_setting('app.current_is_superadmin', true) = 'true'
                OR nullif(current_setting('app.current_tenant_id', true), '') IS NULL
                OR tenant_id::text = nullif(current_setting('app.current_tenant_id', true), '')
                OR tenant_id IS NULL
            )
        $sql$, r.table_name);

        EXECUTE format($sql$
            CREATE POLICY p_tenant_delete ON public.%I
            FOR DELETE
            USING (
                current_setting('app.current_is_superadmin', true) = 'true'
                OR nullif(current_setting('app.current_tenant_id', true), '') IS NULL
                OR tenant_id::text = nullif(current_setting('app.current_tenant_id', true), '')
                OR tenant_id IS NULL
            )
        $sql$, r.table_name);
    END LOOP;
END $$;

-- Tenants table does not have tenant_id; isolate by row id.
ALTER TABLE public.tenants ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS p_tenants_select ON public.tenants;
DROP POLICY IF EXISTS p_tenants_insert ON public.tenants;
DROP POLICY IF EXISTS p_tenants_update ON public.tenants;
DROP POLICY IF EXISTS p_tenants_delete ON public.tenants;

CREATE POLICY p_tenants_select ON public.tenants
FOR SELECT
USING (
    current_setting('app.current_is_superadmin', true) = 'true'
    OR nullif(current_setting('app.current_tenant_id', true), '') IS NULL
    OR id::text = nullif(current_setting('app.current_tenant_id', true), '')
);

CREATE POLICY p_tenants_insert ON public.tenants
FOR INSERT
WITH CHECK (
    current_setting('app.current_is_superadmin', true) = 'true'
    OR nullif(current_setting('app.current_tenant_id', true), '') IS NULL
    OR id::text = nullif(current_setting('app.current_tenant_id', true), '')
);

CREATE POLICY p_tenants_update ON public.tenants
FOR UPDATE
USING (
    current_setting('app.current_is_superadmin', true) = 'true'
    OR nullif(current_setting('app.current_tenant_id', true), '') IS NULL
    OR id::text = nullif(current_setting('app.current_tenant_id', true), '')
)
WITH CHECK (
    current_setting('app.current_is_superadmin', true) = 'true'
    OR nullif(current_setting('app.current_tenant_id', true), '') IS NULL
    OR id::text = nullif(current_setting('app.current_tenant_id', true), '')
);

CREATE POLICY p_tenants_delete ON public.tenants
FOR DELETE
USING (
    current_setting('app.current_is_superadmin', true) = 'true'
    OR nullif(current_setting('app.current_tenant_id', true), '') IS NULL
    OR id::text = nullif(current_setting('app.current_tenant_id', true), '')
);

-- Support messages are scoped through support_tickets (messages table has no tenant_id).
ALTER TABLE public.support_ticket_messages ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS p_support_ticket_messages_select ON public.support_ticket_messages;
DROP POLICY IF EXISTS p_support_ticket_messages_insert ON public.support_ticket_messages;
DROP POLICY IF EXISTS p_support_ticket_messages_update ON public.support_ticket_messages;
DROP POLICY IF EXISTS p_support_ticket_messages_delete ON public.support_ticket_messages;

CREATE POLICY p_support_ticket_messages_select ON public.support_ticket_messages
FOR SELECT
USING (
    EXISTS (
        SELECT 1
        FROM public.support_tickets t
        WHERE t.id = support_ticket_messages.ticket_id
          AND (
              current_setting('app.current_is_superadmin', true) = 'true'
              OR nullif(current_setting('app.current_tenant_id', true), '') IS NULL
              OR t.tenant_id::text = nullif(current_setting('app.current_tenant_id', true), '')
          )
    )
);

CREATE POLICY p_support_ticket_messages_insert ON public.support_ticket_messages
FOR INSERT
WITH CHECK (
    EXISTS (
        SELECT 1
        FROM public.support_tickets t
        WHERE t.id = support_ticket_messages.ticket_id
          AND (
              current_setting('app.current_is_superadmin', true) = 'true'
              OR nullif(current_setting('app.current_tenant_id', true), '') IS NULL
              OR t.tenant_id::text = nullif(current_setting('app.current_tenant_id', true), '')
          )
    )
);

CREATE POLICY p_support_ticket_messages_update ON public.support_ticket_messages
FOR UPDATE
USING (
    EXISTS (
        SELECT 1
        FROM public.support_tickets t
        WHERE t.id = support_ticket_messages.ticket_id
          AND (
              current_setting('app.current_is_superadmin', true) = 'true'
              OR nullif(current_setting('app.current_tenant_id', true), '') IS NULL
              OR t.tenant_id::text = nullif(current_setting('app.current_tenant_id', true), '')
          )
    )
)
WITH CHECK (
    EXISTS (
        SELECT 1
        FROM public.support_tickets t
        WHERE t.id = support_ticket_messages.ticket_id
          AND (
              current_setting('app.current_is_superadmin', true) = 'true'
              OR nullif(current_setting('app.current_tenant_id', true), '') IS NULL
              OR t.tenant_id::text = nullif(current_setting('app.current_tenant_id', true), '')
          )
    )
);

CREATE POLICY p_support_ticket_messages_delete ON public.support_ticket_messages
FOR DELETE
USING (
    EXISTS (
        SELECT 1
        FROM public.support_tickets t
        WHERE t.id = support_ticket_messages.ticket_id
          AND (
              current_setting('app.current_is_superadmin', true) = 'true'
              OR nullif(current_setting('app.current_tenant_id', true), '') IS NULL
              OR t.tenant_id::text = nullif(current_setting('app.current_tenant_id', true), '')
          )
    )
);

-- Support attachments are scoped through messages -> tickets.
ALTER TABLE public.support_ticket_attachments ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS p_support_ticket_attachments_select ON public.support_ticket_attachments;
DROP POLICY IF EXISTS p_support_ticket_attachments_insert ON public.support_ticket_attachments;
DROP POLICY IF EXISTS p_support_ticket_attachments_update ON public.support_ticket_attachments;
DROP POLICY IF EXISTS p_support_ticket_attachments_delete ON public.support_ticket_attachments;

CREATE POLICY p_support_ticket_attachments_select ON public.support_ticket_attachments
FOR SELECT
USING (
    EXISTS (
        SELECT 1
        FROM public.support_ticket_messages m
        JOIN public.support_tickets t ON t.id = m.ticket_id
        WHERE m.id = support_ticket_attachments.message_id
          AND (
              current_setting('app.current_is_superadmin', true) = 'true'
              OR nullif(current_setting('app.current_tenant_id', true), '') IS NULL
              OR t.tenant_id::text = nullif(current_setting('app.current_tenant_id', true), '')
          )
    )
);

CREATE POLICY p_support_ticket_attachments_insert ON public.support_ticket_attachments
FOR INSERT
WITH CHECK (
    EXISTS (
        SELECT 1
        FROM public.support_ticket_messages m
        JOIN public.support_tickets t ON t.id = m.ticket_id
        WHERE m.id = support_ticket_attachments.message_id
          AND (
              current_setting('app.current_is_superadmin', true) = 'true'
              OR nullif(current_setting('app.current_tenant_id', true), '') IS NULL
              OR t.tenant_id::text = nullif(current_setting('app.current_tenant_id', true), '')
          )
    )
);

CREATE POLICY p_support_ticket_attachments_update ON public.support_ticket_attachments
FOR UPDATE
USING (
    EXISTS (
        SELECT 1
        FROM public.support_ticket_messages m
        JOIN public.support_tickets t ON t.id = m.ticket_id
        WHERE m.id = support_ticket_attachments.message_id
          AND (
              current_setting('app.current_is_superadmin', true) = 'true'
              OR nullif(current_setting('app.current_tenant_id', true), '') IS NULL
              OR t.tenant_id::text = nullif(current_setting('app.current_tenant_id', true), '')
          )
    )
)
WITH CHECK (
    EXISTS (
        SELECT 1
        FROM public.support_ticket_messages m
        JOIN public.support_tickets t ON t.id = m.ticket_id
        WHERE m.id = support_ticket_attachments.message_id
          AND (
              current_setting('app.current_is_superadmin', true) = 'true'
              OR nullif(current_setting('app.current_tenant_id', true), '') IS NULL
              OR t.tenant_id::text = nullif(current_setting('app.current_tenant_id', true), '')
          )
    )
);

CREATE POLICY p_support_ticket_attachments_delete ON public.support_ticket_attachments
FOR DELETE
USING (
    EXISTS (
        SELECT 1
        FROM public.support_ticket_messages m
        JOIN public.support_tickets t ON t.id = m.ticket_id
        WHERE m.id = support_ticket_attachments.message_id
          AND (
              current_setting('app.current_is_superadmin', true) = 'true'
              OR nullif(current_setting('app.current_tenant_id', true), '') IS NULL
              OR t.tenant_id::text = nullif(current_setting('app.current_tenant_id', true), '')
          )
    )
);
