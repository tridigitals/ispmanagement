-- Down: disable RLS on tables that were enabled by this migration.
-- Safe: only touches tables that were created post-foundation.
DO $$
DECLARE
    r RECORD;
BEGIN
    FOR r IN
        SELECT c.table_name
        FROM information_schema.columns c
        WHERE c.table_schema = 'public'
          AND c.column_name = 'tenant_id'
          AND c.table_name IN (
              'billing_collection_logs','customer_registration_invites','customer_service_assignments',
              'dhcp_static_services','installation_work_orders','invoice_reminder_logs',
              'managed_radius_nas','managed_radius_servers','message_templates','mikrotik_incidents',
              'mikrotik_ppp_active_sessions','mixradius_import_batches','mixradius_import_conflicts',
              'mixradius_import_external_refs','mixradius_staging_customer_locations','mixradius_staging_customers',
              'mixradius_staging_nas','mixradius_staging_plans','mixradius_staging_transactions',
              'mixradius_staging_usage','network_assets','network_links','network_nodes',
              'olt_onu_history','olts','radius_accounting_sessions','radius_auth_log',
              'radius_servers','service_zones','technician_locations','tenant_radius_assignments',
              'zone_node_bindings','zone_offers'
          )
    LOOP
        EXECUTE format('ALTER TABLE public.%I DISABLE ROW LEVEL SECURITY', r.table_name);
        EXECUTE format('DROP POLICY IF EXISTS p_tenant_select ON public.%I', r.table_name);
        EXECUTE format('DROP POLICY IF EXISTS p_tenant_insert ON public.%I', r.table_name);
        EXECUTE format('DROP POLICY IF EXISTS p_tenant_update ON public.%I', r.table_name);
        EXECUTE format('DROP POLICY IF EXISTS p_tenant_delete ON public.%I', r.table_name);
    END LOOP;
END $$;
