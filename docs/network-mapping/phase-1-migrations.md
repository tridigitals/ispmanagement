# Phase 1 Migrations Plan (Network Mapping)

## Goal
Deliver Phase 1 foundation schema for ISP network mapping with safe rollout and rollback.

## Scope
- Enable PostGIS
- Create topology entities: nodes, links
- Create coverage entities: service zones
- Create relation entity: zone-node bindings
- Add audit metadata and indexes

## Migration Order
1. `20260226090000_enable_postgis`
2. `20260226091000_create_network_nodes`
3. `20260226092000_create_network_links`
4. `20260226093000_create_service_zones`
5. `20260226094000_create_zone_node_bindings`
6. `20260226095000_add_network_audit_indexes`

## Migration Details

## 1) Enable PostGIS
### Up
- `CREATE EXTENSION IF NOT EXISTS postgis;`
- Optional: `CREATE EXTENSION IF NOT EXISTS btree_gist;`

### Down
- Keep extension by default in rollback (recommended).
- Do not drop extension automatically unless environment is dedicated.

## 2) Create `network_nodes`
### Columns
- `id uuid primary key`
- `tenant_id uuid not null`
- `name text not null`
- `node_type text not null`
- `status text not null default 'active'`
- `geom geometry(Point, 4326) not null`
- `capacity_json jsonb not null default '{}'::jsonb`
- `health_json jsonb not null default '{}'::jsonb`
- `metadata jsonb not null default '{}'::jsonb`
- `created_at timestamptz not null default now()`
- `updated_at timestamptz not null default now()`

### Constraints
- `CHECK (node_type in ('core','pop','olt','router','tower','ap','splitter','customer_endpoint'))`
- `CHECK (status in ('active','inactive','maintenance'))`

### Indexes
- `CREATE INDEX idx_network_nodes_tenant_type ON network_nodes(tenant_id, node_type);`
- `CREATE INDEX idx_network_nodes_tenant_status ON network_nodes(tenant_id, status);`
- `CREATE INDEX idx_network_nodes_geom ON network_nodes USING gist(geom);`

## 3) Create `network_links`
### Columns
- `id uuid primary key`
- `tenant_id uuid not null`
- `from_node_id uuid not null references network_nodes(id) on delete cascade`
- `to_node_id uuid not null references network_nodes(id) on delete cascade`
- `name text not null`
- `link_type text not null`
- `status text not null default 'up'`
- `priority int not null default 100`
- `capacity_mbps numeric(12,2)`
- `utilization_pct numeric(5,2)`
- `loss_db numeric(8,3)`
- `latency_ms numeric(8,3)`
- `geom geometry(MultiLineString, 4326) not null`
- `metadata jsonb not null default '{}'::jsonb`
- `created_at timestamptz not null default now()`
- `updated_at timestamptz not null default now()`

### Constraints
- `CHECK (link_type in ('fiber','lan','wireless','ptp_radio'))`
- `CHECK (status in ('up','down','degraded','maintenance'))`
- `CHECK (from_node_id <> to_node_id)`

### Indexes
- `CREATE INDEX idx_network_links_tenant_type ON network_links(tenant_id, link_type);`
- `CREATE INDEX idx_network_links_tenant_status ON network_links(tenant_id, status);`
- `CREATE INDEX idx_network_links_nodes ON network_links(from_node_id, to_node_id);`
- `CREATE INDEX idx_network_links_geom ON network_links USING gist(geom);`

## 4) Create `service_zones`
### Columns
- `id uuid primary key`
- `tenant_id uuid not null`
- `name text not null`
- `zone_type text not null`
- `priority int not null default 100`
- `status text not null default 'active'`
- `geom geometry(MultiPolygon, 4326) not null`
- `metadata jsonb not null default '{}'::jsonb`
- `created_at timestamptz not null default now()`
- `updated_at timestamptz not null default now()`

### Constraints
- `CHECK (status in ('active','inactive'))`

### Indexes
- `CREATE INDEX idx_service_zones_tenant_status ON service_zones(tenant_id, status, priority);`
- `CREATE INDEX idx_service_zones_geom ON service_zones USING gist(geom);`

## 5) Create `zone_node_bindings`
### Columns
- `id uuid primary key`
- `tenant_id uuid not null`
- `zone_id uuid not null references service_zones(id) on delete cascade`
- `node_id uuid not null references network_nodes(id) on delete cascade`
- `is_primary bool not null default false`
- `weight int not null default 100`
- `created_at timestamptz not null default now()`

### Constraints
- `UNIQUE (zone_id, node_id)`

### Indexes
- `CREATE INDEX idx_zone_node_bindings_tenant ON zone_node_bindings(tenant_id);`
- `CREATE INDEX idx_zone_node_bindings_zone_weight ON zone_node_bindings(zone_id, is_primary desc, weight asc);`

## 6) Audit/Utility Indexes
- Add composite indexes to speed admin queries by tenant + updated_at desc.
- Add trigger function for `updated_at` auto-update for all 4 tables.

## Rollback Strategy
- Rollback strictly reverse order:
  1. drop utility indexes/triggers
  2. drop `zone_node_bindings`
  3. drop `service_zones`
  4. drop `network_links`
  5. drop `network_nodes`
- Keep PostGIS extension unless full reset is intentional.

## Data Backfill (Optional)
- Insert node records from existing routers (`mikrotik_routers`) as `node_type='router'`.
- Create initial default service zone per tenant if none exists.

## Validation Checklist
- [ ] `sqlx migrate run` succeeds
- [ ] geometry inserts valid SRID 4326
- [ ] spatial index usage verified with `EXPLAIN ANALYZE`
- [ ] cascade delete behavior verified in staging
- [ ] multi-tenant filter tested on every query path
