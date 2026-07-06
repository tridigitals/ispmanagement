DROP INDEX IF EXISTS idx_customers_tenant_customer_number;
ALTER TABLE public.customers DROP COLUMN IF EXISTS customer_number;
