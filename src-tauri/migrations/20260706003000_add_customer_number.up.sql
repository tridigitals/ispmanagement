-- Add customer_number column to customers table
ALTER TABLE public.customers ADD COLUMN IF NOT EXISTS customer_number TEXT;

-- Unique constraint (tenant-scoped: same number can't repeat within a tenant)
CREATE UNIQUE INDEX IF NOT EXISTS idx_customers_tenant_customer_number ON public.customers (tenant_id, customer_number) WHERE customer_number IS NOT NULL;
