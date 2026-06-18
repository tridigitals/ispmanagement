# Plan: OLT Service ↔ Other Services Integration

**Created:** 2026-06-18
**Project:** ISPMANAGEMENT (Rust/SvelteKit ISP Management)
**Status:** ⏳ Planning — awaiting approval

---

## 1. Analysis Summary

Audit semua services di `src-tauri/src/services/`. OLT service sudah running dengan baik
(fase 0-7 done, HIOSO + MikroTik drivers, poller aktif, 105 ONU di OLT Ngampin),
tapi **terisolasi** dari business-critical services lainnya.

### 🔴 Critical (P0) — Found bypass / missing link

| # | Finding | Location |
|---|---|---|
| C1 | OLT create pakai raw SQL INSERT ke `network_assets` — bypass `NetworkAssetService::create_asset()` (ga lewat validasi, port cache, audit) | `src-tauri/src/services/olt_service/mod.rs:135-152` |
| C2 | `olt_onu_history.mac` cuma string — tidak ada link ke `network_assets` (ont/onu) | `src-tauri/migrations/20260617200000_add_olts.up.sql:29-43` |
| C3 | `network_assets.customer_id` field udah ada & dipakai di `work_orders.rs:136,215` untuk ONT/ONU, tapi OLT poller ga pernah update field `olt_id` & `pon_port` di sini | `src-tauri/src/services/customer_service/work_orders.rs:88` |
| C4 | `refresh_port_usage_cache_for_tenant()` ga dipanggil setelah OLT poll selesai | `src-tauri/src/services/network_asset_port_cache.rs:54` |

### 🟡 Medium (P1) — Visible gap, mudah diintegrate

| # | Finding | Location |
|---|---|---|
| M1 | OLT ga muncul sebagai node di network map (`network_nodes` table) — `sync_network_asset_row()` di integration.rs udah ada tapi ga dipanggil untuk OLT | `src-tauri/src/services/network_mapping_service/integration.rs` |
| M2 | Tidak ada model "OLT ↔ MikroTik uplink" — chain `OLT → Router → PPPoE customer` putus di tengah | `src-tauri/migrations/20260617200000_add_olts.up.sql` |
| M3 | `AlertService` cuma untuk HTTP metrics, ga ada method untuk OLT/ONU offline | `src-tauri/src/services/alert_service.rs:83` |
| M4 | OLT ga punya koordinat (lat/lng) — ga bisa ditaro di peta | `src-tauri/migrations/20260617200000_add_olts.up.sql:4-21` |

### 🟢 Low (P2) — Future automation

| # | Finding | Location |
|---|---|---|
| L1 | `subscription_lifecycle` ga trigger auto-suspend kalau ONU offline > 7 hari | `src-tauri/src/services/customer_service/subscriptions.rs` |
| L2 | ONU baru di OLT ga auto-create DHCP static lease | `src-tauri/src/services/dhcp_static_service.rs` |
| L3 | OLT stats ga di-export ke Prometheus | `src-tauri/src/services/metrics_service.rs` |
| L4 | Tidak ada link `package_id` → OLT untuk capacity planning | `src-tauri/src/services/isp_package_service.rs` |

---

## 2. Dependency Graph

```
Sprint A (refactor) ──┬── Sprint B (ONU→customer) ──┬── Sprint F (auto-suspend)
                      │                              ├── Sprint G (DHCP + metrics)
                      └── Sprint C (map node) ───────┤
                                                     │
Sprint D (OLT↔Router) ┬── Sprint C (map node)       │
                      └── Sprint E (alerts) ────────┘
```

**Parallel-ready:** Sprint D + E bisa paralel dengan Sprint C setelah Sprint A selesai.

---

## 3. Phased Breakdown

### Phase 1 — Sprint A: Refactor Raw SQL → NetworkAssetService (P0)

**Goal:** Bypass bug fix. OLT create harus lewat service proper.

| Step | Action | File |
|---|---|---|
| 1.1 | Tambah `NetworkAssetService` sebagai dependency di `OltService::new()` | `src-tauri/src/services/olt_service/mod.rs:36-46` |
| 1.2 | Replace raw SQL INSERT (line 135-152) dengan `network_asset_service.create_asset()` | `src-tauri/src/services/olt_service/mod.rs:120-152` |
| 1.3 | Verify `create_asset()` udah handle port cache + audit (lihat `network_asset_service.rs:342`) | — |
| 1.4 | Test: create OLT baru → cek `network_assets` ada, `audit_log` ada entry, `port_usage_cache` ke-refresh | `apps/tauri/tests/olt_phase1.rs` |
| 1.5 | Test: update OLT name → audit log ada, asset ikut ke-update | — |
| 1.6 | Test: delete OLT → network_asset ke-soft-delete (lihat `network_asset_service.rs:1033`) | — |
| 1.7 | Commit: `refactor(olt): use NetworkAssetService for auto-create on OLT creation` | — |

**Validasi:** Tidak ada raw SQL di `olt_service/mod.rs` yang nyentuh `network_assets` table.

---

### Phase 2 — Sprint B: ONU MAC → network_assets (P0)

**Goal:** "Customer X komplain → langsung keliatan modemnya nyala di OLT mana."

| Step | Action | File |
|---|---|---|
| 2.1 | Migration: tambah `olt_id` + `pon_port` ke `network_assets`, tambah index | `src-tauri/migrations/20260618xxxxxx_link_assets_to_olt.up.sql` (new) |
| 2.2 | Tambah method `upsert_onu_link()` di `NetworkAssetService` — lookup by MAC, set `olt_id` + `pon_port` | `src-tauri/src/services/network_asset_service.rs` |
| 2.3 | Tambah method `link_onus_to_assets()` di `OltService` — dipanggil setelah `get_olt_all_details()` | `src-tauri/src/services/olt_service/mod.rs:359-413` |
| 2.4 | Logic: untuk setiap ONU, normalize MAC (uppercase, strip colons), query `network_assets WHERE mac = $1 AND asset_type IN ('ont', 'onu')`. Kalau match → update `olt_id`+`pon_port`. Kalau belum ada → create new unassigned asset. | — |
| 2.5 | Endpoint baru: `GET /api/admin/olts/{id}/customers` — return list customer yang ONUnya di OLT ini | `src-tauri/src/http/olt.rs` |
| 2.6 | Frontend: tambah column "Customer" di ONU table, join dengan `olt_onu_history.customer_id` | `src/routes/(app)/admin/network/olts/[id]/+page.svelte` |
| 2.7 | Test: assign 1 ONU ke customer → verify `network_assets.olt_id` ke-update, history ke-record | — |
| 2.8 | Commit: `feat(olt): link ONU MAC to network_assets for customer visibility` | — |

**Validasi:** Dari OLT details page, klik ONU → langsung link ke customer detail page.

---

### Phase 3 — Sprint C: OLT di Network Map (P1)

**Goal:** OLT jadi node di peta topologi network.

| Step | Action | File |
|---|---|---|
| 3.1 | Migration: tambah `latitude` + `longitude` ke `public.olts` | `src-tauri/migrations/20260618xxxxxx_add_olt_coordinates.up.sql` (new) |
| 3.2 | Tambah form input koordinat di OLT edit modal (geolocation picker) | `src/routes/(app)/admin/network/olts/[id]/+page.svelte` |
| 3.3 | Tambah method `register_olt_node()` di `NetworkMappingService` — buat `NetworkNode` dari OLT | `src-tauri/src/services/network_mapping_service/integration.rs` |
| 3.4 | Auto-call `register_olt_node()` di `OltService::create_olt()` setelah NetworkAsset dibuat | `src-tauri/src/services/olt_service/mod.rs:72-155` |
| 3.5 | Frontend: network map page filter untuk show OLT nodes | `src/routes/(app)/admin/network/map/+page.svelte` |
| 3.6 | Test: create OLT dengan koordinat → node muncul di map | — |
| 3.7 | Commit: `feat(olt): render OLT as node on network map` | — |

---

### Phase 4 — Sprint D: OLT ↔ MikroTik Uplink (P1)

**Goal:** Topology chain: `OLT → Router → Customer → ONU`.

| Step | Action | File |
|---|---|---|
| 4.1 | Migration: tambah `uplink_router_id` + `uplink_port` ke `public.olts` | `src-tauri/migrations/20260618xxxxxx_add_olt_uplink.up.sql` (new) |
| 4.2 | Tambah method `set_uplink()` di `OltService` — validate router exists, save link | `src-tauri/src/services/olt_service/mod.rs` |
| 4.3 | Auto-create `NetworkLink` antara OLT dan MikroTik saat uplink di-set | `src-tauri/src/services/network_mapping_service/integration.rs` |
| 4.4 | Frontend: dropdown pilih uplink router di OLT edit form | `src/routes/(app)/admin/network/olts/[id]/+page.svelte` |
| 4.5 | Endpoint: `GET /api/admin/routers/{id}/downstream-olts` — show OLTs yang konek ke router ini | `src-tauri/src/http/router.rs` |
| 4.6 | Test: set uplink → `network_links` ada entry OLT↔Router | — |
| 4.7 | Commit: `feat(olt): link OLT uplink to MikroTik router` | — |

---

### Phase 5 — Sprint E: Alert Service untuk OLT (P1)

**Goal:** Proactive notification kalau OLT/ONU ada masalah.

| Step | Action | File |
|---|---|---|
| 5.1 | Tambah trait `AlertNotifier` atau langsung pakai `NotificationService` untuk OLT events | `src-tauri/src/services/alert_service.rs:42` |
| 5.2 | Tambah method `notify_olt_offline(olt_id, duration_min)` — kirim email/WA kalau OLT down > 5 min | `src-tauri/src/services/olt_service/mod.rs:430-460` (di poll loop) |
| 5.3 | Tambah method `notify_low_signal(onu_id, rx_dbm)` — alert kalau signal < -27 dBm | `src-tauri/src/services/olt_service/mod.rs:364-369` (di get_pon_onu_details) |
| 5.4 | Settings: enable/disable alert per-tenant (audit_service pattern) | `src-tauri/src/services/notification_service.rs` |
| 5.5 | Frontend: bell icon badge di OLT list page kalau ada alert | `src/routes/(app)/admin/network/olts/+page.svelte` |
| 5.6 | Test: simulate OLT down → email terkirim | — |
| 5.7 | Commit: `feat(olt): add alert notifications for OLT/ONU events` | — |

---

### Phase 6 — Sprint F: Auto-Suspend dari ONU Offline (P2)

**Goal:** Customer yang modemnya mati > 7 hari → auto-suspend subscription.

| Step | Action | File |
|---|---|---|
| 6.1 | Migration: tambah `customer_id` ke `olt_onu_history` (atau pakai join lewat `network_assets`) | — (covered by 2.1) |
| 6.2 | Tambah background job `olt_offline_checker` — jalan tiap 6 jam, scan ONU offline > 7 hari | `src-tauri/src/services/olt_service/mod.rs` (new module) |
| 6.3 | Logic: cari ONU yang customer_id-nya aktif, ONU offline > 7 hari → trigger `subscription_lifecycle.suspend()` | `src-tauri/src/services/customer_service/subscriptions.rs` |
| 6.4 | Settings per-tenant: enable auto-suspend, threshold days | `src-tauri/src/services/settings_service.rs` |
| 6.5 | Frontend: badge "Auto-suspend aktif" di settings page | `src/routes/(app)/admin/settings/+page.svelte` |
| 6.6 | Test: mock ONU offline 7 hari → subscription suspended | — |
| 6.7 | Commit: `feat(olt): auto-suspend subscription when ONU offline > 7 days` | — |

---

### Phase 7 — Sprint G: DHCP Auto-Provision + Metrics (P2)

**Goal:** Zero-touch provisioning + observability.

| Step | Action | File |
|---|---|---|
| 7.1 | Tambah method `auto_provision_dhcp()` — saat ONU baru muncul di OLT, create DHCP static lease | `src-tauri/src/services/dhcp_static_service.rs` |
| 7.2 | Tambah `metrics_handler` untuk expose OLT stats ke `/metrics` (Prometheus format) | `src-tauri/src/services/metrics_service.rs` |
| 7.3 | Background job `olt_metrics_collector` — update prometheus gauge tiap 30s | `src-tauri/src/services/olt_service/mod.rs` |
| 7.4 | Dashboard panel: Grafana JSON template untuk OLT overview | `docs/grafana/olt-dashboard.json` (new) |
| 7.5 | Test: OLT new ONU → DHCP lease created automatically | — |
| 7.6 | Commit: `feat(olt): auto-provision DHCP + Prometheus metrics` | — |

---

## 4. Tracking Table

| Sprint | Task | Status | Notes |
|---|---|---|---|
| A | 1.1 Add NetworkAssetService dependency | ⏳ Pending | |
| A | 1.2 Replace raw SQL with service call | ⏳ Pending | |
| A | 1.3 Verify validation + cache + audit | ⏳ Pending | |
| A | 1.4 Test create OLT | ⏳ Pending | |
| A | 1.5 Test update OLT | ⏳ Pending | |
| A | 1.6 Test delete OLT | ⏳ Pending | |
| A | 1.7 Commit refactor | ⏳ Pending | |
| B | 2.1 Migration: olt_id + pon_port di network_assets | ⏳ Pending | |
| B | 2.2 upsert_onu_link() method | ⏳ Pending | |
| B | 2.3 link_onus_to_assets() in OltService | ⏳ Pending | |
| B | 2.4 MAC normalization + lookup logic | ⏳ Pending | |
| B | 2.5 /api/admin/olts/{id}/customers endpoint | ⏳ Pending | |
| B | 2.6 Frontend: Customer column di ONU table | ⏳ Pending | |
| B | 2.7 Test end-to-end MAC → customer | ⏳ Pending | |
| B | 2.8 Commit feat | ⏳ Pending | |
| C | 3.1 Migration: lat/lng di olts | ⏳ Pending | |
| C | 3.2 Geolocation picker di edit modal | ⏳ Pending | |
| C | 3.3 register_olt_node() in NetworkMappingService | ⏳ Pending | |
| C | 3.4 Auto-call on OltService::create_olt() | ⏳ Pending | |
| C | 3.5 Map filter untuk OLT nodes | ⏳ Pending | |
| C | 3.6 Test create OLT → map node | ⏳ Pending | |
| C | 3.7 Commit feat | ⏳ Pending | |
| D | 4.1 Migration: uplink_router_id + port | ⏳ Pending | |
| D | 4.2 set_uplink() method | ⏳ Pending | |
| D | 4.3 Auto-create NetworkLink OLT↔Router | ⏳ Pending | |
| D | 4.4 Frontend: uplink dropdown | ⏳ Pending | |
| D | 4.5 /api/admin/routers/{id}/downstream-olts | ⏳ Pending | |
| D | 4.6 Test topology | ⏳ Pending | |
| D | 4.7 Commit feat | ⏳ Pending | |
| E | 5.1 AlertNotifier trait | ⏳ Pending | |
| E | 5.2 notify_olt_offline() | ⏳ Pending | |
| E | 5.3 notify_low_signal() | ⏳ Pending | |
| E | 5.4 Settings: enable/disable per-tenant | ⏳ Pending | |
| E | 5.5 Bell badge di OLT list | ⏳ Pending | |
| E | 5.6 Test alert | ⏳ Pending | |
| E | 5.7 Commit feat | ⏳ Pending | |
| F | 6.1 customer_id di olt_onu_history | ⏳ Pending | |
| F | 6.2 olt_offline_checker background job | ⏳ Pending | |
| F | 6.3 Auto-suspend logic | ⏳ Pending | |
| F | 6.4 Settings UI | ⏳ Pending | |
| F | 6.5 Badge di settings | ⏳ Pending | |
| F | 6.6 Test | ⏳ Pending | |
| F | 6.7 Commit feat | ⏳ Pending | |
| G | 7.1 auto_provision_dhcp() | ⏳ Pending | |
| G | 7.2 metrics_handler | ⏳ Pending | |
| G | 7.3 olt_metrics_collector | ⏳ Pending | |
| G | 7.4 Grafana dashboard | ⏳ Pending | |
| G | 7.5 Test | ⏳ Pending | |
| G | 7.6 Commit feat | ⏳ Pending | |

---

## 5. Files Likely to Change

**Backend Rust:**
- `src-tauri/src/services/olt_service/mod.rs` (all sprints)
- `src-tauri/src/services/network_asset_service.rs` (Sprint A, B)
- `src-tauri/src/services/network_mapping_service/integration.rs` (Sprint C, D)
- `src-tauri/src/services/alert_service.rs` (Sprint E)
- `src-tauri/src/services/customer_service/subscriptions.rs` (Sprint F)
- `src-tauri/src/services/dhcp_static_service.rs` (Sprint G)
- `src-tauri/src/services/metrics_service.rs` (Sprint G)
- `src-tauri/src/http/olt.rs`, `http/router.rs` (Sprint B, D)
- `src-tauri/src/services/mod.rs` (dependency wiring)

**Migrations (new):**
- `src-tauri/migrations/20260618xxxxxx_link_assets_to_olt.up.sql`
- `src-tauri/migrations/20260618xxxxxx_add_olt_coordinates.up.sql`
- `src-tauri/migrations/20260618xxxxxx_add_olt_uplink.up.sql`

**Frontend SvelteKit:**
- `src/routes/(app)/admin/network/olts/[id]/+page.svelte` (Sprint B, C, D, E)
- `src/routes/(app)/admin/network/olts/+page.svelte` (Sprint E)
- `src/routes/(app)/admin/network/map/+page.svelte` (Sprint C)
- `src/lib/api/olt.ts` (Sprint B, D — new endpoint wrappers)

**Tests:**
- `src-tauri/tests/olt_phase_a.rs` (Sprint A)
- `src-tauri/tests/olt_phase_b.rs` (Sprint B)
- dst.

---

## 6. Risks & Tradeoffs

| Risk | Mitigation |
|---|---|
| **Migration lock** — Sprint B migration tambah kolom ke `network_assets` (large table) | `ADD COLUMN ... DEFAULT NULL` — no rewrite, fast |
| **Circular dependency** — OltService butuh NetworkAssetService, MapService, AlertService | Wire di `services/mod.rs` constructor, satu arah |
| **Poller overhead** — Tambah logic `link_onus_to_assets()` di poll loop bisa slow | Run only on `details` endpoint (manual), bukan di poller bg |
| **MAC format mismatch** — ONU pakai `AA:BB:CC:DD:EE:FF`, network_assets bisa `-` atau `:` | Normalize di Rust: `mac.replace([':', '-'], "").to_uppercase()` |
| **Auto-suspend false positive** — Customer offline tapi ga mau disuspend | Threshold per-tenant setting, default 14 hari (bukan 7) |
| **Existing OLT** — Yang udah dibuat (Ngampin, Jambu) belum lewat NetworkAssetService | Migration backfill: insert asset dari olt yang udah ada |

---

## 7. Open Questions

1. **Scope MVP:** Mau eksekusi Sprint A+B dulu (yang paling critical) atau sekaligus sampai Sprint E?
2. **Backfill:** Existing OLT (Ngampin, Jambu) perlu di-backfill ke `network_assets` table? (sangat direkomendasikan)
3. **Naming:** `olt_id` vs `parent_olt_id` di `network_assets` — prefer mana?
4. **Auto-suspend threshold:** Default 7 hari atau 14 hari? (depends on business rule)
5. **Geolocation:** Pake Google Maps API atau OSM (self-hosted)?

---

## 8. Next Step

Tunggu approval dari user, lalu mulai dari **Sprint A** (refactor raw SQL bypass).
Setelah Sprint A merge, lanjut **Sprint B** (ONU MAC → customer) yang paling impactful.
