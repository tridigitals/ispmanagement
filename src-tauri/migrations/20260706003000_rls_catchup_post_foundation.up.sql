-- RLS Catch-Up: tables created after foundation migration (20260217140000)
-- Re-runs the same pattern on any tenant_id-bearing table that lacks RLS.
-- Idempotent: only enables RLS + creates policies if they don't already exist.

DO $$
DECLARE
    r RECORD;
    pol_count int;
BEGIN
    FOR r IN
        SELECT c.table_name
        FROM information_schema.columns c
        JOIN pg_class pc ON pc.relname = c.table_name AND pc.relnamespace = 'public'::regnamespace
        WHERE c.table_schema = 'public'
          AND c.column_name = 'tenant_id'
          AND pc.relrowsecurity = false
        GROUP BY c.table_name
    LOOP
        EXECUTE format('ALTER TABLE public.%I ENABLE ROW LEVEL SECURITY', r.table_name);

        -- Drop policies first (idempotent) then recreate
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

