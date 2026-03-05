ALTER TABLE public.isp_packages
    ADD COLUMN IF NOT EXISTS service_type text;

UPDATE public.isp_packages
SET service_type = 'internet_pppoe'
WHERE service_type IS NULL OR btrim(service_type) = '';

ALTER TABLE public.isp_packages
    ALTER COLUMN service_type SET DEFAULT 'internet_pppoe';

ALTER TABLE public.isp_packages
    ALTER COLUMN service_type SET NOT NULL;

CREATE INDEX IF NOT EXISTS idx_isp_packages_service_type
    ON public.isp_packages(tenant_id, service_type);
