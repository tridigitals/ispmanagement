ALTER TABLE public.invoices
ADD COLUMN IF NOT EXISTS rejection_reason text;
