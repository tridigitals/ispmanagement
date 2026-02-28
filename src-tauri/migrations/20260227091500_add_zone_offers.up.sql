-- Phase 2: commercial coverage offers by zone

CREATE TABLE IF NOT EXISTS public.zone_offers (
    id uuid PRIMARY KEY,
    tenant_id uuid NOT NULL,
    zone_id uuid NOT NULL REFERENCES public.service_zones(id) ON DELETE CASCADE,
    package_id text NOT NULL REFERENCES public.isp_packages(id) ON DELETE CASCADE,
    price_monthly numeric(12,2),
    price_yearly numeric(12,2),
    is_active boolean NOT NULL DEFAULT true,
    metadata jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT uq_zone_offers_zone_package UNIQUE (zone_id, package_id)
);

CREATE INDEX IF NOT EXISTS idx_zone_offers_tenant_zone
  ON public.zone_offers (tenant_id, zone_id);
CREATE INDEX IF NOT EXISTS idx_zone_offers_tenant_active
  ON public.zone_offers (tenant_id, is_active);
CREATE INDEX IF NOT EXISTS idx_zone_offers_package
  ON public.zone_offers (package_id);

DROP TRIGGER IF EXISTS trg_zone_offers_set_updated_at ON public.zone_offers;

-- Keep this migration self-contained: ensure updated_at trigger function exists
CREATE OR REPLACE FUNCTION public.tg_set_updated_at()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
  NEW.updated_at = now();
  RETURN NEW;
END;
$$;

CREATE TRIGGER trg_zone_offers_set_updated_at
BEFORE UPDATE ON public.zone_offers
FOR EACH ROW EXECUTE FUNCTION public.tg_set_updated_at();
