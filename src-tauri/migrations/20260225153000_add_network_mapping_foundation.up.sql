-- Network mapping foundation (Phase 1)
-- Requires PostgreSQL + PostGIS

CREATE EXTENSION IF NOT EXISTS postgis;

CREATE TABLE IF NOT EXISTS public.network_nodes (
    id uuid PRIMARY KEY,
    tenant_id uuid NOT NULL,
    name text NOT NULL,
    node_type text NOT NULL,
    status text NOT NULL DEFAULT 'active',
    geom geometry(Point, 4326) NOT NULL,
    capacity_json jsonb NOT NULL DEFAULT '{}'::jsonb,
    health_json jsonb NOT NULL DEFAULT '{}'::jsonb,
    metadata jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT chk_network_nodes_type
      CHECK (node_type IN ('core', 'pop', 'olt', 'router', 'tower', 'ap', 'splitter', 'customer_endpoint')),
    CONSTRAINT chk_network_nodes_status
      CHECK (status IN ('active', 'inactive', 'maintenance'))
);

CREATE TABLE IF NOT EXISTS public.network_links (
    id uuid PRIMARY KEY,
    tenant_id uuid NOT NULL,
    from_node_id uuid NOT NULL REFERENCES public.network_nodes(id) ON DELETE CASCADE,
    to_node_id uuid NOT NULL REFERENCES public.network_nodes(id) ON DELETE CASCADE,
    name text NOT NULL,
    link_type text NOT NULL,
    status text NOT NULL DEFAULT 'up',
    priority int NOT NULL DEFAULT 100,
    capacity_mbps numeric(12,2),
    utilization_pct numeric(5,2),
    loss_db numeric(8,3),
    latency_ms numeric(8,3),
    geom geometry(MultiLineString, 4326) NOT NULL,
    metadata jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT chk_network_links_type
      CHECK (link_type IN ('fiber', 'lan', 'wireless', 'ptp_radio')),
    CONSTRAINT chk_network_links_status
      CHECK (status IN ('up', 'down', 'degraded', 'maintenance')),
    CONSTRAINT chk_network_links_nodes
      CHECK (from_node_id <> to_node_id)
);

CREATE TABLE IF NOT EXISTS public.service_zones (
    id uuid PRIMARY KEY,
    tenant_id uuid NOT NULL,
    name text NOT NULL,
    zone_type text NOT NULL,
    priority int NOT NULL DEFAULT 100,
    status text NOT NULL DEFAULT 'active',
    geom geometry(MultiPolygon, 4326) NOT NULL,
    metadata jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT chk_service_zones_status
      CHECK (status IN ('active', 'inactive'))
);

CREATE TABLE IF NOT EXISTS public.zone_node_bindings (
    id uuid PRIMARY KEY,
    tenant_id uuid NOT NULL,
    zone_id uuid NOT NULL REFERENCES public.service_zones(id) ON DELETE CASCADE,
    node_id uuid NOT NULL REFERENCES public.network_nodes(id) ON DELETE CASCADE,
    is_primary boolean NOT NULL DEFAULT false,
    weight int NOT NULL DEFAULT 100,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT uq_zone_node_bindings_zone_node UNIQUE (zone_id, node_id)
);

-- B-Tree indexes
CREATE INDEX IF NOT EXISTS idx_network_nodes_tenant_type
  ON public.network_nodes (tenant_id, node_type);
CREATE INDEX IF NOT EXISTS idx_network_nodes_tenant_status
  ON public.network_nodes (tenant_id, status);
CREATE INDEX IF NOT EXISTS idx_network_nodes_tenant_updated
  ON public.network_nodes (tenant_id, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_network_links_tenant_type
  ON public.network_links (tenant_id, link_type);
CREATE INDEX IF NOT EXISTS idx_network_links_tenant_status
  ON public.network_links (tenant_id, status);
CREATE INDEX IF NOT EXISTS idx_network_links_nodes
  ON public.network_links (from_node_id, to_node_id);
CREATE INDEX IF NOT EXISTS idx_network_links_tenant_updated
  ON public.network_links (tenant_id, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_service_zones_tenant_status_priority
  ON public.service_zones (tenant_id, status, priority);
CREATE INDEX IF NOT EXISTS idx_service_zones_tenant_updated
  ON public.service_zones (tenant_id, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_zone_node_bindings_tenant
  ON public.zone_node_bindings (tenant_id);
CREATE INDEX IF NOT EXISTS idx_zone_node_bindings_zone_weight
  ON public.zone_node_bindings (zone_id, is_primary DESC, weight ASC);

-- Spatial indexes
CREATE INDEX IF NOT EXISTS idx_network_nodes_geom
  ON public.network_nodes USING gist (geom);
CREATE INDEX IF NOT EXISTS idx_network_links_geom
  ON public.network_links USING gist (geom);
CREATE INDEX IF NOT EXISTS idx_service_zones_geom
  ON public.service_zones USING gist (geom);

-- updated_at trigger helper
CREATE OR REPLACE FUNCTION public.tg_set_updated_at()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
  NEW.updated_at = now();
  RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS trg_network_nodes_set_updated_at ON public.network_nodes;
CREATE TRIGGER trg_network_nodes_set_updated_at
BEFORE UPDATE ON public.network_nodes
FOR EACH ROW EXECUTE FUNCTION public.tg_set_updated_at();

DROP TRIGGER IF EXISTS trg_network_links_set_updated_at ON public.network_links;
CREATE TRIGGER trg_network_links_set_updated_at
BEFORE UPDATE ON public.network_links
FOR EACH ROW EXECUTE FUNCTION public.tg_set_updated_at();

DROP TRIGGER IF EXISTS trg_service_zones_set_updated_at ON public.service_zones;
CREATE TRIGGER trg_service_zones_set_updated_at
BEFORE UPDATE ON public.service_zones
FOR EACH ROW EXECUTE FUNCTION public.tg_set_updated_at();
