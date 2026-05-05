ALTER TABLE public.isp_packages
    ADD COLUMN IF NOT EXISTS provisioning_type text;

UPDATE public.isp_packages
SET provisioning_type = 'pppoe'
WHERE provisioning_type IS NULL OR btrim(provisioning_type) = '';

ALTER TABLE public.isp_packages
    ALTER COLUMN provisioning_type SET DEFAULT 'pppoe';

ALTER TABLE public.isp_packages
    ALTER COLUMN provisioning_type SET NOT NULL;

CREATE INDEX IF NOT EXISTS idx_isp_packages_provisioning_type
    ON public.isp_packages(tenant_id, provisioning_type);
