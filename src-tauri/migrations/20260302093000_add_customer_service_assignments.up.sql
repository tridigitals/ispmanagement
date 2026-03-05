DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1
    FROM pg_proc p
    JOIN pg_namespace n ON n.oid = p.pronamespace
    WHERE p.proname = 'set_updated_at'
      AND n.nspname = 'public'
  ) THEN
    CREATE FUNCTION public.set_updated_at()
    RETURNS trigger
    LANGUAGE plpgsql
    AS $fn$
    BEGIN
      NEW.updated_at = now();
      RETURN NEW;
    END;
    $fn$;
  END IF;
END $$;

CREATE TABLE IF NOT EXISTS public.customer_service_assignments (
    id text PRIMARY KEY,
    tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
    invoice_id text NOT NULL REFERENCES public.invoices(id) ON DELETE CASCADE,
    subscription_id text NOT NULL REFERENCES public.customer_subscriptions(id) ON DELETE CASCADE,
    work_order_id text REFERENCES public.installation_work_orders(id) ON DELETE SET NULL,
    customer_id text NOT NULL REFERENCES public.customers(id) ON DELETE CASCADE,
    location_id text NOT NULL REFERENCES public.customer_locations(id) ON DELETE CASCADE,
    selected_zone_id text,
    selected_node_id text,
    selected_node_score numeric(7,2),
    candidate_snapshot jsonb NOT NULL DEFAULT '[]'::jsonb,
    path_node_ids jsonb NOT NULL DEFAULT '[]'::jsonb,
    path_link_ids jsonb NOT NULL DEFAULT '[]'::jsonb,
    status text NOT NULL DEFAULT 'pending_installation',
    resolution_notes text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT uq_customer_service_assignments_invoice UNIQUE (tenant_id, invoice_id),
    CONSTRAINT chk_customer_service_assignments_status
      CHECK (status IN ('pending_installation', 'assigned', 'in_progress', 'completed', 'failed'))
);

CREATE INDEX IF NOT EXISTS idx_customer_service_assignments_subscription
  ON public.customer_service_assignments (tenant_id, subscription_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_customer_service_assignments_work_order
  ON public.customer_service_assignments (tenant_id, work_order_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_customer_service_assignments_node
  ON public.customer_service_assignments (tenant_id, selected_node_id);

DROP TRIGGER IF EXISTS trg_customer_service_assignments_set_updated_at
  ON public.customer_service_assignments;
CREATE TRIGGER trg_customer_service_assignments_set_updated_at
BEFORE UPDATE ON public.customer_service_assignments
FOR EACH ROW EXECUTE FUNCTION public.set_updated_at();
