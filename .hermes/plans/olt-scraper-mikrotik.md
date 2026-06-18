# Plan: Fix HIOSO Scraper + MikroTik RouterOS OLT Driver

## Overview
Dua masalah independent:
1. **HIOSO scraper** — `parse_js_array_tokens()` gagal parse halaman OLT → 0 ONU data
2. **MikroTik RouterOS driver** — belum ada driver untuk ambil bandwidth + SFP data dari MikroTik

## Audited: Existing mikrotik-rs Infrastructure
- ISPMANAGEMENT sudah punya `mikrotik-rs` crate (line 31 mikrotik_service.rs)
- `MikrotikDevice::connect(addr, username, password)` — sudah teruji
- `CommandBuilder::new().command("/interface/print").build()` — sudah digunakan
- `CommandResponse::Reply` → `.attributes` HashMap<String, Option<String>>
- **BELUM ada**: `/interface/ethernet/monitor` (for SFP data) atau `/interface/monitor-traffic`

## Step 1: Fix HIOSO parse_js_array_tokens
**Root cause**: Pattern `\(([^)]*)\)` doesn't handle nested parens or multiline JS.
**Fix**: Replace with multi-strategy fallback — try `new Array(...)` first, then try regex-less extraction.

## Step 2: Create MikroTikRouterOSDriver
File: `src-tauri/src/services/olt_service/drivers/mikrotik_ros.rs`

Implement `OltDriver` trait:
- `connect()` → `MikrotikDevice::connect()`
- `get_system_info()` → `/system/resource/print` + `/system/identity/print`
- `get_global_stats()` → `/interface/print` filter SFP interfaces, count as "PON ports"
- `get_pon_onu_details()` → return empty (MikroTik doesn't have per-ONU data)
- `disconnect()` → close/RouterOS logout
- `get_onu_signal/get_onu_status` → `/interface/ethernet/monitor` for SFP RX

## Step 3: Add mikrotik_ros to create_driver()
In `mod.rs`, add mapping for `"mikrotik_ros"` → `MikroTikRouterOSDriver`
Add the OLT type to frontend dropdown.

## Step 4: Test & Commit
1. `cargo check` → PASS
2. Restart server, test koneksi with MikroTik OLT
3. Commit: "feat(olt): fix HIOSO scraper + add MikroTik RouterOS driver"

## Files Modified
| File | Change |
|------|--------|
| `drivers/hioso.rs` | Fixed parse_js_array_tokens for robustness |
| `drivers/mikrotik_ros.rs` | NEW — MikroTik RouterOS OLTDriver implementation |
| `drivers/mod.rs` | Register new driver, re-export |
| `mod.rs` | Add `mikrotik_ros` to `create_driver()` match |
| `models/olt.rs` | Add `mikrotik_ros` variant to OltType |
| `http/olt.rs` | Accept new OLT type in create/update handlers |
| Frontend `olt.ts` + `+page.svelte` | Add `mikrotik_ros` to type dropdown |
