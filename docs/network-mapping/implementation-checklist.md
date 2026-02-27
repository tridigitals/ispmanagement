# Network Mapping Implementation Checklist

## Status Key
- `[ ]` pending
- `[~]` in progress
- `[x]` done

## Progress Notes
- [x] Blueprint and supporting docs created in `docs/network-mapping/`
- [x] Phase 1 foundation migration files created:
  - `src-tauri/migrations/20260225153000_add_network_mapping_foundation.up.sql`
  - `src-tauri/migrations/20260225153000_add_network_mapping_foundation.down.sql`

## Phase 1 - Data Foundation

## DB / Migrations
- [x] Add migration `enable_postgis`
- [x] Create table `network_nodes`
- [x] Create table `network_links`
- [x] Create table `service_zones`
- [x] Create table `zone_node_bindings`
- [x] Add spatial indexes (`gist`) for geometry columns
- [x] Add composite tenant indexes
- [x] Add `updated_at` triggers
- [x] Add down migrations for all above

## Backend Models
- [x] Add `src-tauri/src/models/network_mapping.rs` (nodes, links, zones, bindings, resolve DTOs)
- [x] Register models/mod exports

## Backend Services
- [x] Add `src-tauri/src/services/network_mapping_service.rs` (nodes + links + zones + bindings + resolve)
- [x] Add validation helpers for geo payload
- [x] Add tenant scoping in every query

## HTTP Layer
- [x] Add `src-tauri/src/http/network_mapping.rs`
- [x] Register routes in `src-tauri/src/http/mod.rs`
- [x] Wire service in app startup (`lib.rs`, `http::AppState`, `src/bin/server.rs`)
- [x] Add auth guards and permission checks
- [x] Standardize error mapping for geometry errors

## API Client
- [x] Add client methods in `src/lib/api/client.ts`:
- [x] `network.nodes.list/create/update/delete`
- [x] `network.links.list/create/update/delete`
- [x] `network.zones.list/create/update/delete/resolve`
- [x] `network.zoneBindings.list/create/delete`

## Phase 1 - Frontend

## Admin UI (Read-first)
- [x] Add page `src/routes/[tenant]/(app)/admin/network/map/+page.svelte`
- [x] Render base map (MapLibre)
- [x] Show nodes layer
- [x] Show links layer
- [x] Show zones layer
- [x] Add layer toggles
- [x] Add map bbox-based loading

## CRUD UI
- [x] Node create/edit modal
- [x] Link create/edit modal (polyline input)
- [x] Zone create/edit modal (polygon)
- [x] Zone-node binding management UI

## Performance
- [x] Debounce map move requests
- [x] Cancel in-flight requests on viewport change
- [x] Client-side cache by bbox+zoom signature
- [x] Cluster customer points (if layer enabled)

## Phase 2 - Commercial Coverage

## DB / API
- [ ] Create table `zone_offers`
- [ ] Create table `package_templates` (if not existing equivalent)
- [ ] Add endpoints for zone offers
- [ ] Add endpoint `/coverage/check`

## Frontend
- [ ] Customer flow: address/coordinate coverage check
- [ ] Filter visible packages by zone
- [ ] Display zone-specific price

## Phase 3 - Provisioning Integration

## Backend
- [ ] Add path compute service (shortest path with constraints)
- [ ] Add candidate node ranking by health+capacity
- [ ] Store assignment result on order/payment verification
- [ ] Trigger installation work order creation from successful payment

## Frontend
- [ ] Work order detail includes selected node/path context
- [ ] NOC impact view: node/link issue -> affected customers

## Quality / Security / Ops

## RBAC
- [ ] Add new permissions to seeder:
- [ ] `network_topology:read/manage`
- [ ] `service_zones:read/manage`
- [ ] `coverage:read`
- [ ] Bind permissions to owner/admin/planner/noc/technician roles

## Testing
- [ ] Migration integration test (up/down)
- [ ] API test nodes CRUD
- [ ] API test links CRUD + geometry validation
- [ ] API test zones resolve by point
- [ ] API test tenant isolation
- [ ] UI smoke test map load and layer toggles

## Observability
- [ ] Add structured logs for network mapping endpoints
- [ ] Add metrics: request count/latency/bbox size
- [ ] Add alert for slow spatial query (p95 threshold)

## Deployment Readiness
- [ ] Add PostGIS dependency note to deployment docs
- [ ] Add migration execution SOP
- [ ] Add rollback SOP
- [ ] Staging load test with synthetic 50k points

## Milestone Exit Criteria
- [ ] Phase 1: topology + zones fully manageable from admin
- [ ] Phase 2: package visibility filtered by resolved zone
- [ ] Phase 3: paid order produces assignable installation flow
