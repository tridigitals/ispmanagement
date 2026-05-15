CREATE TABLE IF NOT EXISTS public.network_assets (
    id text PRIMARY KEY,
    tenant_id text NOT NULL,
    asset_type text NOT NULL,
    name text NOT NULL,
    code text,
    vendor text,
    model text,
    serial_number text,
    status text NOT NULL DEFAULT 'available',
    customer_id text NULL REFERENCES public.customers(id) ON DELETE SET NULL,
    location_id text NULL REFERENCES public.customer_locations(id) ON DELETE SET NULL,
    work_order_id text NULL REFERENCES public.installation_work_orders(id) ON DELETE SET NULL,
    parent_asset_id text NULL REFERENCES public.network_assets(id) ON DELETE SET NULL,
    notes text,
    metadata jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT chk_network_assets_type CHECK (
        asset_type IN ('olt', 'odc', 'odp', 'splitter', 'ont', 'onu', 'fat', 'nap')
    ),
    CONSTRAINT chk_network_assets_status CHECK (
        status IN ('available', 'reserved', 'installed', 'faulty', 'retired')
    ),
    CONSTRAINT chk_network_assets_parent_self CHECK (
        parent_asset_id IS NULL OR parent_asset_id <> id
    )
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_network_assets_tenant_code
  ON public.network_assets (tenant_id, lower(code))
  WHERE code IS NOT NULL AND btrim(code) <> '';

CREATE UNIQUE INDEX IF NOT EXISTS uq_network_assets_tenant_serial
  ON public.network_assets (tenant_id, lower(serial_number))
  WHERE serial_number IS NOT NULL AND btrim(serial_number) <> '';

CREATE INDEX IF NOT EXISTS idx_network_assets_tenant_type
  ON public.network_assets (tenant_id, asset_type);
CREATE INDEX IF NOT EXISTS idx_network_assets_tenant_status
  ON public.network_assets (tenant_id, status);
CREATE INDEX IF NOT EXISTS idx_network_assets_tenant_customer
  ON public.network_assets (tenant_id, customer_id);
CREATE INDEX IF NOT EXISTS idx_network_assets_tenant_location
  ON public.network_assets (tenant_id, location_id);
CREATE INDEX IF NOT EXISTS idx_network_assets_tenant_parent
  ON public.network_assets (tenant_id, parent_asset_id);
CREATE INDEX IF NOT EXISTS idx_network_assets_tenant_updated
  ON public.network_assets (tenant_id, updated_at DESC);

DROP TRIGGER IF EXISTS trg_network_assets_set_updated_at ON public.network_assets;
CREATE TRIGGER trg_network_assets_set_updated_at
BEFORE UPDATE ON public.network_assets
FOR EACH ROW EXECUTE FUNCTION public.tg_set_updated_at();
