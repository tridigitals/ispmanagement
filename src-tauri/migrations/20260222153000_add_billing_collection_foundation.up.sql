-- Billing collection foundation:
-- 1) reminder delivery log per invoice (email/sms/wa later)
-- 2) suspension/resume/action trail for collection workflow

CREATE TABLE IF NOT EXISTS public.invoice_reminder_logs (
    id text PRIMARY KEY NOT NULL,
    tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
    invoice_id text NOT NULL REFERENCES public.invoices(id) ON DELETE CASCADE,
    reminder_code text NOT NULL, -- e.g. H-3, H-1, H+1, H+3
    channel text NOT NULL DEFAULT 'email',
    recipient text,
    status text NOT NULL DEFAULT 'sent', -- sent | failed | skipped
    detail text,
    created_at timestamp with time zone NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_invoice_reminder_logs_tenant_created
    ON public.invoice_reminder_logs (tenant_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_invoice_reminder_logs_invoice_created
    ON public.invoice_reminder_logs (invoice_id, created_at DESC);

CREATE TABLE IF NOT EXISTS public.billing_collection_logs (
    id text PRIMARY KEY NOT NULL,
    tenant_id text NOT NULL REFERENCES public.tenants(id) ON DELETE CASCADE,
    invoice_id text NOT NULL REFERENCES public.invoices(id) ON DELETE CASCADE,
    subscription_id text,
    action text NOT NULL, -- evaluate | reminder | suspend | resume | skip
    result text NOT NULL, -- success | failed | skipped
    reason text,
    actor_type text NOT NULL DEFAULT 'system', -- system | manual
    actor_id text,
    created_at timestamp with time zone NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_billing_collection_logs_tenant_created
    ON public.billing_collection_logs (tenant_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_billing_collection_logs_invoice_created
    ON public.billing_collection_logs (invoice_id, created_at DESC);
