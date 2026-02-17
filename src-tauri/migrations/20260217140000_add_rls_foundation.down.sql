-- Rollback RLS Foundation

DO $$
DECLARE
    r RECORD;
BEGIN
    -- Revert generic tenant_id-table policies.
    FOR r IN
        SELECT c.table_name
        FROM information_schema.columns c
        WHERE c.table_schema = 'public'
          AND c.column_name = 'tenant_id'
        GROUP BY c.table_name
    LOOP
        EXECUTE format('DROP POLICY IF EXISTS p_tenant_select ON public.%I', r.table_name);
        EXECUTE format('DROP POLICY IF EXISTS p_tenant_insert ON public.%I', r.table_name);
        EXECUTE format('DROP POLICY IF EXISTS p_tenant_update ON public.%I', r.table_name);
        EXECUTE format('DROP POLICY IF EXISTS p_tenant_delete ON public.%I', r.table_name);
        EXECUTE format('ALTER TABLE public.%I DISABLE ROW LEVEL SECURITY', r.table_name);
    END LOOP;
END $$;

-- tenants table
DROP POLICY IF EXISTS p_tenants_select ON public.tenants;
DROP POLICY IF EXISTS p_tenants_insert ON public.tenants;
DROP POLICY IF EXISTS p_tenants_update ON public.tenants;
DROP POLICY IF EXISTS p_tenants_delete ON public.tenants;
ALTER TABLE public.tenants DISABLE ROW LEVEL SECURITY;

-- support_ticket_messages table
DROP POLICY IF EXISTS p_support_ticket_messages_select ON public.support_ticket_messages;
DROP POLICY IF EXISTS p_support_ticket_messages_insert ON public.support_ticket_messages;
DROP POLICY IF EXISTS p_support_ticket_messages_update ON public.support_ticket_messages;
DROP POLICY IF EXISTS p_support_ticket_messages_delete ON public.support_ticket_messages;
ALTER TABLE public.support_ticket_messages DISABLE ROW LEVEL SECURITY;

-- support_ticket_attachments table
DROP POLICY IF EXISTS p_support_ticket_attachments_select ON public.support_ticket_attachments;
DROP POLICY IF EXISTS p_support_ticket_attachments_insert ON public.support_ticket_attachments;
DROP POLICY IF EXISTS p_support_ticket_attachments_update ON public.support_ticket_attachments;
DROP POLICY IF EXISTS p_support_ticket_attachments_delete ON public.support_ticket_attachments;
ALTER TABLE public.support_ticket_attachments DISABLE ROW LEVEL SECURITY;
