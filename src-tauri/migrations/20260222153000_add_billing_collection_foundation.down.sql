DROP INDEX IF EXISTS public.idx_billing_collection_logs_invoice_created;
DROP INDEX IF EXISTS public.idx_billing_collection_logs_tenant_created;
DROP TABLE IF EXISTS public.billing_collection_logs;

DROP INDEX IF EXISTS public.idx_invoice_reminder_logs_invoice_created;
DROP INDEX IF EXISTS public.idx_invoice_reminder_logs_tenant_created;
DROP TABLE IF EXISTS public.invoice_reminder_logs;
