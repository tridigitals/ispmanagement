# API Contract - Network Mapping (Phase 1/2)

## Conventions
- Base URL: `/api/admin/network`
- Auth: Bearer JWT
- Multi-tenant scope: inferred from token `tenant_id`
- Response envelope errors:
```json
{ "error": "message" }
```

## 1) Nodes

## GET `/nodes`
Query:
- `bbox=minLng,minLat,maxLng,maxLat` (optional)
- `node_type` (optional)
- `status` (optional)
- `page`, `per_page`

Response 200:
```json
{
  "data": [{
    "id": "uuid",
    "name": "POP-CBD",
    "node_type": "pop",
    "status": "active",
    "lat": -6.2,
    "lng": 106.8,
    "capacity_json": {},
    "health_json": {},
    "metadata": {},
    "updated_at": "2026-02-25T08:00:00Z"
  }],
  "total": 1,
  "page": 1,
  "per_page": 50
}
```

## POST `/nodes`
Request:
```json
{
  "name": "POP-CBD",
  "node_type": "pop",
  "status": "active",
  "lat": -6.2,
  "lng": 106.8,
  "capacity_json": {"ports_total": 24},
  "health_json": {},
  "metadata": {}
}
```

Response 201: node object

## PATCH `/nodes/:id`
Partial update allowed.

## DELETE `/nodes/:id`
Response 204

---

## 2) Links

## GET `/links`
Query:
- `bbox`
- `link_type`
- `status`
- `page`, `per_page`

Response 200:
```json
{
  "data": [{
    "id": "uuid",
    "name": "POP-CBD to ODP-12",
    "from_node_id": "uuid",
    "to_node_id": "uuid",
    "link_type": "fiber",
    "status": "up",
    "capacity_mbps": 1000,
    "utilization_pct": 37.5,
    "latency_ms": 1.2,
    "loss_db": 0.8,
    "geometry": {
      "type": "MultiLineString",
      "coordinates": [[[106.8,-6.2],[106.81,-6.21]]]
    }
  }],
  "total": 1
}
```

## POST `/links`
Request:
```json
{
  "name": "POP-CBD to ODP-12",
  "from_node_id": "uuid",
  "to_node_id": "uuid",
  "link_type": "fiber",
  "status": "up",
  "priority": 100,
  "capacity_mbps": 1000,
  "geometry": {
    "type": "MultiLineString",
    "coordinates": [[[106.8,-6.2],[106.81,-6.21]]]
  },
  "metadata": {}
}
```

Response 201: link object

## PATCH `/links/:id`
Partial update allowed.

## DELETE `/links/:id`
Response 204

---

## 3) Service Zones

## GET `/zones`
Query:
- `bbox`
- `status`
- `page`, `per_page`

## POST `/zones`
Request:
```json
{
  "name": "Zone A",
  "zone_type": "residential",
  "priority": 100,
  "status": "active",
  "geometry": {
    "type": "MultiPolygon",
    "coordinates": [[[[106.8,-6.2],[106.81,-6.2],[106.81,-6.21],[106.8,-6.21],[106.8,-6.2]]]]
  },
  "metadata": {}
}
```

## PATCH `/zones/:id`
Partial update.

## DELETE `/zones/:id`
Response 204

## POST `/zones/resolve`
Request:
```json
{ "lat": -6.205, "lng": 106.805 }
```
Response 200:
```json
{
  "zone": {
    "id": "uuid",
    "name": "Zone A",
    "priority": 100
  }
}
```

---

## 4) Zone Node Bindings

## GET `/zone-node-bindings?zone_id=uuid`
Response:
```json
{
  "data": [{
    "id": "uuid",
    "zone_id": "uuid",
    "node_id": "uuid",
    "is_primary": true,
    "weight": 100
  }]
}
```

## POST `/zone-node-bindings`
Request:
```json
{
  "zone_id": "uuid",
  "node_id": "uuid",
  "is_primary": true,
  "weight": 100
}
```

## DELETE `/zone-node-bindings/:id`
Response 204

---

## 5) Coverage Check (Customer Flow)

## POST `/coverage/check`
Request:
```json
{
  "lat": -6.205,
  "lng": 106.805,
  "customer_type": "residential"
}
```
Response 200:
```json
{
  "zone": { "id": "uuid", "name": "Zone A" },
  "candidate_nodes": [{ "id": "uuid", "name": "POP-CBD", "score": 92.5 }],
  "available_packages": [{ "package_template_id": "uuid", "name": "Basic 20 Mbps", "price_monthly": 165000 }]
}
```

---

## 6) Standard Errors
- `400`: validation failed
- `401`: unauthenticated
- `403`: permission denied
- `404`: resource not found
- `409`: unique conflict
- `422`: invalid geometry / spatial payload
- `500`: internal error

Example:
```json
{ "error": "Invalid geometry: polygon not closed" }
```

---

## 7) RBAC Requirements per Endpoint
- `nodes.*`: `network_topology:manage` (read uses `network_topology:read`)
- `links.*`: `network_topology:manage`
- `zones.*`: `service_zones:manage`
- `zone-node-bindings.*`: `service_zones:manage`
- `coverage/check`: `coverage:read` (admin + customer app flow)

