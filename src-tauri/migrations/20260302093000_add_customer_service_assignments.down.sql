DROP TRIGGER IF EXISTS trg_customer_service_assignments_set_updated_at
  ON public.customer_service_assignments;

DROP INDEX IF EXISTS public.idx_customer_service_assignments_node;
DROP INDEX IF EXISTS public.idx_customer_service_assignments_work_order;
DROP INDEX IF EXISTS public.idx_customer_service_assignments_subscription;

DROP TABLE IF EXISTS public.customer_service_assignments;
