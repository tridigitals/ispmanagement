# ISP Network Mapping Implementation Blueprint

## 1. Objective
Build a production-grade network mapping and service-coverage platform for ISP operations that supports:
- Mixed media topology (`fiber`, `lan`, `wireless`, `ptp_radio`).
- Real path geometry (not straight lines).
- Multi-path routing (primary/backup/tertiary).
- Area-based package availability and regional pricing.
- Scalable map rendering for tens of thousands of customers.

This blueprint is written to guide phased implementation across backend, database, frontend, and operations.

---

## 2. Guiding Principles
1. Separate **commercial product** from **technical provisioning mapping**.
2. Model network as a **graph** (`nodes`, `links`, `paths`), not a flat list.
3. Use **PostGIS** for coverage and spatial lookup.
4. Use **viewport-based fetch + clustering/vector tiles** for scale.
5. Keep critical operations auditable and role-protected.

---

## 3. Domain Model

### 3.1 Core Entities
- `network_nodes`: physical/logical points (core, POP, OLT, router, AP, splitter, customer endpoint).
- `network_links`: media connections between nodes (fiber/lan/wireless).
- `network_paths`: logical service paths between origin and destination nodes.
- `network_path_members`: ordered links composing a path.
- `service_zones`: coverage polygons/multipolygons.
- `zone_node_bindings`: which nodes serve which zones.
- `package_templates`: product definition (speed/features).
- `zone_offers`: per-zone commercial prices per package template.
- `router_tech_mappings`: per-router technical mapping for package (PPP profile/pool/vlan).
- `customer_service_assignments`: chosen zone + node + path for active customer service.
- `installation_work_orders`: field execution lifecycle.

### 3.2 Separation of Concerns
- **Commercial**: `package_templates` + `zone_offers`.
- **Network planning**: `service_zones`, `zone_node_bindings`, capacity.
- **Technical provisioning**: `router_tech_mappings`.
- **Operations**: alerts/incidents/work orders tied to node/link/path.

---

## 4. Database Design (PostgreSQL + PostGIS)

## 4.1 Recommended Extensions
- `postgis`
- `pg_trgm` (search)
- optional: `btree_gist`

### 4.2 Key Columns

#### `network_nodes`
- `id uuid pk`
- `tenant_id uuid not null`
- `name text not null`
- `node_type text not null` (`core`, `pop`, `olt`, `router`, `tower`, `ap`, `splitter`, `customer_endpoint`)
- `status text not null default 'active'`
- `geom geometry(Point, 4326) not null`
- `capacity_json jsonb` (ports, throughput, slots)
- `health_json jsonb` (cpu, mem, latency snapshot)
- `created_at`, `updated_at`

Indexes:
- `gist (geom)`
- `btree (tenant_id, node_type)`

#### `network_links`
- `id uuid pk`
- `tenant_id uuid not null`
- `from_node_id uuid not null`
- `to_node_id uuid not null`
- `link_type text not null` (`fiber`, `lan`, `wireless`, `ptp_radio`)
- `status text not null default 'up'`
- `priority int not null default 100`
- `capacity_mbps numeric(12,2)`
- `utilization_pct numeric(5,2)`
- `loss_db numeric(8,3)` (fiber/radio)
- `latency_ms numeric(8,3)`
- `geom geometry(MultiLineString, 4326) not null`
- `metadata jsonb`

Indexes:
- `gist (geom)`
- `btree (tenant_id, status, link_type)`
- `btree (from_node_id, to_node_id)`

#### `network_paths`
- `id uuid pk`
- `tenant_id uuid not null`
- `name text not null`
- `source_node_id uuid not null`
- `target_node_id uuid not null`
- `path_role text not null` (`primary`, `backup`, `tertiary`)
- `status text not null default 'active'`
- `cost_score numeric(10,3)`
- `latency_estimate_ms numeric(10,3)`
- `created_at`, `updated_at`

Unique suggestion:
- `(tenant_id, source_node_id, target_node_id, path_role)`

#### `network_path_members`
- `id uuid pk`
- `tenant_id uuid not null`
- `path_id uuid not null`
- `link_id uuid not null`
- `seq_no int not null`

Unique:
- `(path_id, seq_no)`
- `(path_id, link_id)`

#### `service_zones`
- `id uuid pk`
- `tenant_id uuid not null`
- `name text not null`
- `zone_type text not null` (`residential`, `business`, `wireless`, etc.)
- `priority int not null default 100`
- `status text not null default 'active'`
- `geom geometry(MultiPolygon, 4326) not null`

Indexes:
- `gist (geom)`
- `btree (tenant_id, status, priority)`

#### `zone_offers`
- `id uuid pk`
- `tenant_id uuid not null`
- `zone_id uuid not null`
- `package_template_id uuid not null`
- `price_monthly numeric(14,2) not null`
- `price_yearly numeric(14,2) not null default 0`
- `is_active bool not null default true`

Unique:
- `(zone_id, package_template_id)`

---

## 5. Spatial and Path Query Patterns

### 5.1 Coverage Lookup
Given customer location point:
- `SELECT zone WHERE ST_Contains(zone.geom, point) ORDER BY priority ASC LIMIT 1;`
- fallback to nearest zone if no contains (optional with `ST_DWithin`).

### 5.2 Viewport Fetch
Given map bbox:
- `ST_Intersects(geom, ST_MakeEnvelope(..., 4326))`
- Return simplified geometry by zoom (`ST_SimplifyPreserveTopology` threshold by zoom).

### 5.3 Path Selection
Input: zone + package + customer endpoint
1. Candidate nodes from `zone_node_bindings`.
2. Filter by capacity and health.
3. Compute best path (Dijkstra/A* on `network_links` weighted by latency/loss/utilization).
4. Store selected primary and fallback path.

---

## 6. API Design (HTTP)

Base prefix: `/api/admin/network`

### 6.1 Nodes
- `GET /nodes?bbox=&type=&status=&page=`
- `POST /nodes`
- `PATCH /nodes/:id`
- `DELETE /nodes/:id`

### 6.2 Links
- `GET /links?bbox=&type=&status=`
- `POST /links`
- `PATCH /links/:id`
- `DELETE /links/:id`

### 6.3 Paths
- `GET /paths?source_node_id=&target_node_id=`
- `POST /paths`
- `POST /paths/compute` (auto compute candidate path)
- `PATCH /paths/:id`
- `DELETE /paths/:id`

### 6.4 Zones
- `GET /zones?bbox=`
- `POST /zones`
- `PATCH /zones/:id`
- `DELETE /zones/:id`
- `POST /zones/resolve` (point -> zone)

### 6.5 Offers and Mapping
- `GET /zone-offers?zone_id=`
- `POST /zone-offers`
- `PATCH /zone-offers/:id`
- `GET /router-tech-mappings?package_id=`
- `POST /router-tech-mappings/upsert`

### 6.6 Customer Coverage
- `POST /coverage/check` with lat/lng/address
- response: `zone`, `available_packages`, `candidate_nodes`

---

## 7. Frontend Map Architecture (50k+ customers)

### 7.1 Rendering Strategy
- Use `MapLibre GL` (WebGL).
- Layers:
  - `zones` polygon layer
  - `links` line layer (color by media/type/status)
  - `nodes` symbol/circle layer
  - `customers` clustered source

### 7.2 Data Loading Strategy
- Never load all customers at once.
- Use viewport bbox fetch and zoom-based detail.
- At low zoom: cluster only.
- At high zoom: individual points.

### 7.3 Detail UX
- Click marker => side panel details (lazy API fetch).
- Keep map lightweight; avoid huge popup payloads.
- Support toggles for layer visibility.

### 7.4 Performance Rules
- debounce map move events (150-250ms).
- cancel in-flight fetch on next move.
- cache tile/bbox responses.
- avoid DOM marker lists; rely on GL layers.

---

## 8. Provisioning and Work Order Flow

### 8.1 Order-to-Activation
1. Customer location validated.
2. Zone resolved.
3. Show zone-eligible package offers.
4. Payment verified.
5. Create `installation_work_order`.
6. Assign technician + schedule.
7. After completed checklist, apply PPPoE/router mapping.
8. Activate service + emit notifications.

### 8.2 Technician Checklist (minimum)
- cable installed
- CPE/ONT installed
- PPPoE configured
- speed test pass
- photo evidence + geo timestamp

---

## 9. RBAC Matrix (Minimum)

- `network_topology:read/manage`
- `service_zones:read/manage`
- `zone_offers:read/manage`
- `router_tech_mapping:read/manage`
- `work_orders:read/manage`
- `coverage:read`

Roles:
- `owner/admin`: full manage
- `planner`: topology + zones + offers
- `noc`: read topology + alerts/incidents
- `technician`: work orders + assigned node details
- `customer`: coverage-check + own services only

---

## 10. Observability and Reliability

### 10.1 Metrics
- API latency p95/p99 for map endpoints
- count of nodes/links/zones fetched per request
- path compute time
- work order SLA

### 10.2 Logs
- audit trails for topology and pricing changes
- old/new value snapshots
- actor + tenant + timestamp

### 10.3 Alerts
- high node utilization
- link down with impacted customer count
- zone without capacity

---

## 11. Phased Implementation Plan

### Phase 1 - Foundation (mandatory first)
- Add schema for nodes, links, zones, mappings.
- CRUD APIs for nodes/links/zones.
- Basic map page with nodes+links+zones layers.
- Capacity fields and basic status badges.

### Phase 2 - Commercial Integration
- Add `zone_offers`.
- Coverage check endpoint.
- Customer package listing filtered by zone.

### Phase 3 - Provisioning Automation
- Path compute service.
- Auto node assignment from zone+capacity.
- Integrate with installation work orders.

### Phase 4 - Scale and Intelligence
- Clustering/vector tile endpoints.
- impact analysis and simulation.
- recommendation engine for reroute/failover.

---

## 12. Non-Functional Requirements
- Multi-tenant isolation at query level (`tenant_id` mandatory).
- All spatial and graph endpoints must support pagination/cursor.
- No endpoint should return unbounded result sets in production.
- Every write action auditable.
- All policy checks enforced on API (not only UI).

---

## 13. Suggested Initial Backlog (Actionable)
1. Migration: `network_nodes`, `network_links`, `service_zones`.
2. PostGIS enable + spatial indexes.
3. API CRUD for nodes/links/zones.
4. Admin page: Network Map (read-only first).
5. Coverage check endpoint + test cases.
6. Add `zone_offers` and package filtering by zone.
7. Integrate with existing package router mapping.
8. Add work-order creation trigger after payment verification.

---

## 14. Success Criteria
- Map remains responsive at 50k customer points (clustered).
- Coverage check < 300ms p95.
- Zone-based package filtering works correctly.
- Provisioning chooses valid router/path with capacity checks.
- Full audit trail for topology/pricing changes.

